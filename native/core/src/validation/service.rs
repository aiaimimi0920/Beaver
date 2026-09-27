use super::{operations, repository, requests::Request, settings::Settings};
use crate::{files::Files, project_work_gate::ProjectWorkGate, store::Store};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

pub struct Tools {
    pub engine: PathBuf,
    pub ffmpeg: Option<PathBuf>,
}
pub struct ToolContext {
    pub engine: &'static str,
    pub project: Value,
    pub settings: Settings,
}
pub type Resolver = Arc<dyn Fn(&ToolContext) -> Result<Tools> + Send + Sync>;

#[derive(Clone)]
pub struct Storage {
    pub store: Arc<Mutex<Store>>,
    pub files: Arc<Files>,
    /// `Some` identifies a project-owned store; `None` is the legacy host store.
    pub project_id: Option<String>,
    /// Retained for active work and shadow suppression, but unavailable for new work.
    pub draining: bool,
    pub work_gate: ProjectWorkGate,
}

pub type StorageResolver = Arc<dyn Fn(&str) -> Result<Storage> + Send + Sync>;
pub type StorageEnumerator = Arc<dyn Fn() -> Result<Vec<Storage>> + Send + Sync>;

pub(crate) struct State {
    pub store: Arc<Mutex<Store>>,
    pub files: Arc<Files>,
    pub storage: StorageResolver,
    pub enumerate: StorageEnumerator,
    pub resolve: Resolver,
    pub changed: Arc<dyn Fn() + Send + Sync>,
    pub stop: AtomicBool,
    pub active: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
}

/// Independent code and rendered lanes. Every process belongs to exactly one run.
pub struct Service {
    state: Arc<State>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    previews: super::live_preview::Sessions,
}

impl Service {
    pub fn start(
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        resolve: Resolver,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self> {
        let fallback = Storage {
            store: store.clone(),
            files: files.clone(),
            project_id: None,
            draining: false,
            work_gate: ProjectWorkGate::default(),
        };
        let storage = Arc::new({
            let fallback = fallback.clone();
            move |_project_id: &str| Ok(fallback.clone())
        });
        let enumerate = Arc::new(move || Ok(vec![fallback.clone()]));
        Self::start_with_routing(store, files, storage, enumerate, resolve, changed)
    }

    pub fn start_with_routing(
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        storage: StorageResolver,
        enumerate: StorageEnumerator,
        resolve: Resolver,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self> {
        let service = Self {
            state: Arc::new(State {
                store,
                files,
                storage,
                enumerate,
                resolve,
                changed,
                stop: AtomicBool::new(false),
                active: Mutex::new(BTreeMap::new()),
            }),
            workers: Mutex::new(vec![]),
            previews: super::live_preview::Sessions::default(),
        };
        for kind in ["code", "visual"] {
            let state = service.state.clone();
            let worker = thread::Builder::new()
                .name(format!("validation-{kind}"))
                .spawn(move || super::jobs::work(state, kind))?;
            service
                .workers
                .lock()
                .map_err(|_| anyhow!("Validation worker lock unavailable"))?
                .push(worker);
        }
        Ok(service)
    }

    pub fn cancel(&self, input: &Value) -> Result<Value> {
        let project_id = operations::string(input, "projectId")?;
        let storage = (self.state.storage)(project_id)?;
        let mut store = storage
            .store
            .lock()
            .map_err(|_| anyhow!("Database lock unavailable"))?;
        let request = Request::new("validation.run.cancel", input)?;
        if let Some(result) = request.replay(&store)? {
            return Ok(result);
        }
        let mut run = operations::owned_run(&store, input)?;
        if let Some(token) = self
            .state
            .active
            .lock()
            .map_err(|_| anyhow!("Validation lock unavailable"))?
            .get(&run.id)
        {
            token.store(true, Ordering::SeqCst);
        } else if run.status == "queued" && run.managed {
            run.status = "cancelled".into();
            run.phase = "cancelled".into();
            run.verdict = "needsReview".into();
            run.error = Some("Cancelled before execution".into());
            run.finished_at = Some(repository::now());
        } else if run.status == "running" && !run.managed {
            anyhow::bail!("Use task.interrupt to cancel a task's code gate");
        }
        let result = request.finish(
            &mut store,
            json!({"runId":run.id}),
            vec![("validationRun", run.id.clone(), serde_json::to_value(&run)?)],
        )?;
        (self.state.changed)();
        Ok(result)
    }

    pub fn preview(&self, method: &str, input: &Value) -> Result<Value> {
        anyhow::ensure!(
            !self.state.stop.load(Ordering::SeqCst),
            "PREVIEW_SERVICE_CLOSED"
        );
        self.previews.call(&self.state, method, input)
    }

    pub fn shutdown(&self) -> Result<()> {
        self.state.stop.store(true, Ordering::SeqCst);
        self.previews.shutdown()?;
        for token in self
            .state
            .active
            .lock()
            .map_err(|_| anyhow!("Validation lock unavailable"))?
            .values()
        {
            token.store(true, Ordering::SeqCst);
        }
        for worker in self
            .workers
            .lock()
            .map_err(|_| anyhow!("Validation worker lock unavailable"))?
            .drain(..)
        {
            worker
                .join()
                .map_err(|_| anyhow!("Validation worker stopped unexpectedly"))?;
        }
        Ok(())
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
