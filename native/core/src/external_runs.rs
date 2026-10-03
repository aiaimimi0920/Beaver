//! A capability registry owned by one host lifetime. It cannot be deserialized.
pub use crate::external_run_history::{read_job_evidence, read_receipt, run_history};
use crate::{executor::Outcome, external_run_records as records, files::Files, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

#[derive(Clone)]
pub struct ExternalRegistry {
    pub(crate) inner: Arc<Registry>,
}
pub(crate) struct Registry {
    owner: String,
    runs: Mutex<HashMap<String, Arc<Live>>>,
    closing: AtomicBool,
}
pub(crate) struct Live {
    owner: String,
    pub(crate) task_id: String,
    pub(crate) run_id: String,
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) files: Arc<Files>,
    pub(crate) workspace: PathBuf,
    pub(crate) blender_path: Option<PathBuf>,
    pub(crate) prompt: String,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) closed: AtomicBool,
    pub(crate) cleanup_confirmed: AtomicBool,
    pub(crate) stopped: tokio::sync::Notify,
    pub(crate) serial: Arc<AsyncMutex<()>>,
    pub(crate) finish: Mutex<Option<oneshot::Sender<Outcome>>>,
}

impl Default for ExternalRegistry {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Registry {
                owner: uuid::Uuid::new_v4().to_string(),
                runs: Mutex::new(HashMap::new()),
                closing: AtomicBool::new(false),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn register(
        &self,
        task: &Value,
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        prompt: String,
        blender_path: Option<PathBuf>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(Arc<Live>, oneshot::Receiver<Outcome>)> {
        ensure!(
            !self.inner.closing.load(Ordering::SeqCst),
            "External host is shutting down"
        );
        let id = task["id"].as_str().context("Task identity missing")?;
        let workspace = files.resolve_workspace(
            id,
            std::path::Path::new(
                task["workspace"]
                    .as_str()
                    .context("Task workspace missing")?,
            ),
        )?;
        let mut runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?;
        ensure!(!self.is_closing(), "External host is shutting down");
        ensure!(
            runs.get(id)
                .is_none_or(|run| run.closed.load(Ordering::SeqCst)),
            "External task already has a live run"
        );
        ensure!(!cancelled.load(Ordering::SeqCst), "External task cancelled");
        let run_id = uuid::Uuid::new_v4().to_string();
        records::create(
            &mut *store
                .lock()
                .map_err(|_| anyhow::anyhow!("Store unavailable"))?,
            id,
            &run_id,
        )?;
        let (finish, receiver) = oneshot::channel();
        let live = Arc::new(Live {
            owner: self.inner.owner.clone(),
            task_id: id.into(),
            run_id,
            store,
            files,
            workspace,
            blender_path,
            prompt,
            cancelled,
            closed: AtomicBool::new(false),
            cleanup_confirmed: AtomicBool::new(false),
            stopped: tokio::sync::Notify::new(),
            serial: Arc::new(AsyncMutex::new(())),
            finish: Mutex::new(Some(finish)),
        });
        runs.insert(id.into(), live.clone());
        Ok((live, receiver))
    }

    pub(crate) fn remove_live(&self, live: &Live) -> Result<()> {
        ensure!(
            live.cleanup_confirmed.load(Ordering::SeqCst),
            "Cannot release an unconfirmed external process lifetime"
        );
        let mut runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?;
        if runs
            .get(&live.task_id)
            .is_some_and(|current| current.run_id == live.run_id)
        {
            runs.remove(&live.task_id);
        }
        Ok(())
    }

    pub(crate) fn is_closing(&self) -> bool {
        self.inner.closing.load(Ordering::SeqCst)
    }

    /// Revoke tool admission immediately, even while Host::call is draining.
    pub fn cancel_all(&self) -> Result<()> {
        self.inner.closing.store(true, Ordering::SeqCst);
        let runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?;
        for run in runs.values() {
            run.cancelled.store(true, Ordering::SeqCst);
            run.stopped.notify_one();
        }
        Ok(())
    }

    /// Call after scheduler shutdown and admitted request draining to release runtime leases.
    pub fn release_closed(&self) -> Result<()> {
        ensure!(
            self.is_closing(),
            "External host must stop before releasing runs"
        );
        let mut runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?;
        ensure!(
            runs.values()
                .all(|run| run.cleanup_confirmed.load(Ordering::SeqCst)),
            "External runs are still draining"
        );
        runs.clear();
        Ok(())
    }

    pub(crate) fn live(&self, task_id: &str, run_id: Option<&str>) -> Result<Arc<Live>> {
        let runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?;
        let run = runs
            .get(task_id)
            .context("External run is not ready in this host")?;
        ensure!(
            run.owner == self.inner.owner && run_id.is_none_or(|id| id == run.run_id),
            "Stale external run"
        );
        Ok(run.clone())
    }

    pub fn context(&self, task_id: &str) -> Result<Value> {
        let live = self.live(task_id, None)?;
        let store = live
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
        let run: Value = store
            .get(records::RUN, &live.run_id)?
            .context("Run missing")?;
        let task: Value = store.get("task", task_id)?.context("Task missing")?;
        Ok(
            json!({"taskId":task_id,"runId":live.run_id,"revision":run["revision"],"status":run["status"],
            "workspace":live.workspace,"prompt":live.prompt,"task":task,
            "tools":crate::external_run_permissions::contract(&task),
            "instructions":include_str!("../../../resources/instructions/code-structure.md")}),
        )
    }

    pub fn pending(&self) -> Result<Value> {
        let ids: Vec<_> = self
            .inner
            .runs
            .lock()
            .map_err(|_| anyhow::anyhow!("External registry unavailable"))?
            .values()
            .filter(|run| {
                !run.closed.load(Ordering::SeqCst) && !run.cancelled.load(Ordering::SeqCst)
            })
            .map(|run| run.task_id.clone())
            .collect();
        let contexts: Result<Vec<_>> = ids.iter().map(|id| self.context(id)).collect();
        Ok(json!(contexts?))
    }

    pub fn receipt(&self, task_id: &str, run_id: &str, request_id: &str) -> Result<Value> {
        let live = self.live(task_id, Some(run_id))?;
        let store = live
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
        store
            .get(records::RECEIPT, &records::key(run_id, request_id))?
            .context("External receipt not found")
    }
}

impl Live {
    pub(crate) fn require_active(&self) -> Result<()> {
        ensure!(
            !self.closed.load(Ordering::SeqCst) && !self.cancelled.load(Ordering::SeqCst),
            "External run is revoked"
        );
        Ok(())
    }
    pub(crate) async fn close(self: &Arc<Self>, status: &str) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        let _serial = self.serial.lock().await;
        let id = self.run_id.clone();
        let cleanup = tokio::task::spawn_blocking(move || crate::external_tools::cleanup(&id))
            .await
            .context("External tool cleanup worker failed")
            .and_then(|result| result);
        self.record_cleanup(status, cleanup)
    }

    pub(crate) fn record_cleanup(&self, status: &str, cleanup: Result<()>) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        let mut store = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
        match cleanup {
            Ok(()) => {
                crate::external_run_recovery::confirmed(&mut store, &self.run_id, status)?;
                self.cleanup_confirmed.store(true, Ordering::SeqCst);
                Ok(())
            }
            Err(error) => {
                crate::external_run_recovery::quarantine(
                    &mut store,
                    &self.run_id,
                    &error.to_string(),
                )?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
#[path = "external_run_tests.rs"]
mod tests;
