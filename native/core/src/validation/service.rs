use super::{model::Run, operations, repository, requests::Request};
use crate::store::Store;
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
    pub godot: PathBuf,
    pub ffmpeg: Option<PathBuf>,
}
pub type Resolver = Arc<dyn Fn(&Store, &Run) -> Result<Tools> + Send + Sync>;

pub(crate) struct State {
    pub store: Arc<Mutex<Store>>,
    pub data: PathBuf,
    pub resolve: Resolver,
    pub changed: Arc<dyn Fn() + Send + Sync>,
    pub stop: AtomicBool,
    pub active: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
}

/// Independent code and rendered lanes. Every process belongs to exactly one run.
pub struct Service {
    state: Arc<State>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl Service {
    pub fn start(
        store: Arc<Mutex<Store>>,
        data: PathBuf,
        resolve: Resolver,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self> {
        let service = Self {
            state: Arc::new(State {
                store,
                data,
                resolve,
                changed,
                stop: AtomicBool::new(false),
                active: Mutex::new(BTreeMap::new()),
            }),
            workers: Mutex::new(vec![]),
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
        let mut store = self
            .state
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

    pub fn shutdown(&self) -> Result<()> {
        self.state.stop.store(true, Ordering::SeqCst);
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
