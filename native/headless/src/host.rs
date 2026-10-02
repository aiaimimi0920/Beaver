use crate::{api, audit, contract, runtime};
use anyhow::{Context, Result};
use beaver_core::{
    call_log, external_runs::ExternalRegistry, project_storage_router::ProjectStorageRouter,
    scheduler::Scheduler, store::Store,
};
use fs2::FileExt;
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub struct Host {
    pub(crate) root: PathBuf,
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) router: Arc<ProjectStorageRouter>,
    pub(crate) scheduler: Scheduler,
    pub(crate) external: ExternalRegistry,
    pub(crate) closing: AtomicBool,
    // Admission serializes accepted mutations; stop cancels external runs before draining it.
    admission: Arc<tokio::sync::Mutex<()>>,
    shutdown: tokio::sync::Mutex<Option<std::result::Result<(), String>>>,
    // Keep exclusive instance ownership until every host-owned field has dropped.
    _instance: File,
}

impl Host {
    pub fn open(root: &Path) -> Result<Arc<Self>> {
        anyhow::ensure!(root.is_absolute(), "--data-dir must be absolute");
        fs::create_dir_all(root)?;
        let root = fs::canonicalize(root)?;
        let instance = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".beaver-native.lock"))?;
        instance
            .try_lock_exclusive()
            .context("Another Beaver instance owns this data directory")?;
        let host = Store::open(&root)?;
        call_log::recover(&host)?;
        let store = Arc::new(Mutex::new(host));
        let router = Arc::new(ProjectStorageRouter::new(store.clone()));
        runtime::open_all(&router)?;
        let external = ExternalRegistry::new();
        let scheduler = runtime::start(&root, store.clone(), router.clone(), external.clone())?;
        Ok(Arc::new(Self {
            root,
            store,
            router,
            scheduler,
            external,
            closing: AtomicBool::new(false),
            admission: Arc::new(tokio::sync::Mutex::new(())),
            shutdown: tokio::sync::Mutex::new(None),
            _instance: instance,
        }))
    }

    pub async fn call(self: &Arc<Self>, method: &str, input: Value) -> Result<Value> {
        // Cancellation before admission does no work. Once admitted, the owned job
        // retains the permit through mutation and logging even if its caller drops.
        let admitted = self.admission.clone().lock_owned().await;
        anyhow::ensure!(
            !self.closing.load(Ordering::SeqCst),
            "Headless host is shutting down"
        );
        contract::validate(method, &input)?;
        let host = self.clone();
        let method = method.to_owned();
        tokio::spawn(async move {
            let _admitted = admitted;
            let record = audit::begin(&host, &method, &input)?;
            let result = api::call(host.clone(), method.clone(), input).await;
            if audit::finish(&host, record, &method, &result).is_err() {
                // A mutation may already be durable. Never turn success into a
                // retryable failure because diagnostic logging failed afterward.
                eprintln!(
                    "Headless audit completion failed; the operation result remains authoritative"
                );
            }
            result
        })
        .await
        .context("Headless operation worker failed")?
    }

    pub(crate) fn request_stop(&self) -> Result<()> {
        self.closing.store(true, Ordering::SeqCst);
        // Uses the same cancellation flags owned by Scheduler. This must precede
        // the admission wait: a long tool owns that permit until its receipt lands.
        self.external.cancel_all()
    }

    pub async fn shutdown(&self) -> Result<()> {
        let mut failure = self.request_stop().err();
        let mut finished = self.shutdown.lock().await;
        if let Some(result) = finished.as_ref() {
            return result.clone().map_err(anyhow::Error::msg);
        }
        let _admitted = self.admission.lock().await;
        // Ordinary admitted mutations can still wake Scheduler until this point.
        if let Err(error) = self.scheduler.shutdown().await {
            failure.get_or_insert_with(|| anyhow::Error::msg(error));
        }
        if let Err(error) = self.external.release_closed() {
            failure.get_or_insert(error);
        }
        // Core framework jobs can outlive their Codex caller. Drain each owned
        // runtime even when scheduler shutdown reported a finalization error.
        match self.router.runtimes() {
            Ok(runtimes) => {
                for runtime in &runtimes {
                    if let Err(error) = beaver_core::framework::shutdown(&runtime.store()).await {
                        failure.get_or_insert(error);
                    }
                }
                drop(runtimes);
            }
            Err(error) => {
                failure.get_or_insert(error);
            }
        }
        if let Err(error) = self.router.close_all() {
            failure.get_or_insert(error);
        }
        let result = failure.map_or(Ok(()), Err);
        *finished = Some(
            result
                .as_ref()
                .map(|_| ())
                .map_err(|error| format!("{error:#}")),
        );
        result
    }
}

#[cfg(test)]
#[path = "host_audit_tests.rs"]
mod tests;
