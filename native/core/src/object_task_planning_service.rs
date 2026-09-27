//! Owns bounded planning jobs independently of the object execution scheduler.
use crate::{
    object_task_planning::{
        self as planning, AnswerRequest, RoundKey, Session, SessionRequest, StartRequest, Status,
        Transition,
    },
    object_task_planning_executor as executor,
    object_task_planning_launch::Launch,
    object_task_planning_round as round,
    project_runtime::ProjectRuntime,
};
use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{sync::watch, task::JoinHandle};

pub use crate::object_task_planning_executor::Notify;
pub type Factory = Arc<dyn Fn(&ProjectRuntime, &RoundKey) -> Result<Launch> + Send + Sync>;

struct Job {
    project_id: String,
    draft_id: String,
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

struct State {
    stopped: bool,
    jobs: HashMap<String, Job>,
    persistence_failed: bool,
}

struct Inner {
    state: Mutex<State>,
    factory: Factory,
    notify: Notify,
    limit: usize,
}

#[derive(Clone)]
pub struct Service(Arc<Inner>);

impl Service {
    pub fn new(factory: Factory, notify: Notify, limit: usize) -> Self {
        Self(Arc::new(Inner {
            state: Mutex::new(State {
                stopped: false,
                jobs: HashMap::new(),
                persistence_failed: false,
            }),
            factory,
            notify,
            limit: limit.clamp(1, 6),
        }))
    }

    pub fn start(&self, runtime: ProjectRuntime, input: &StartRequest) -> Result<Session> {
        let transition = planning::start(&runtime, input)?;
        self.launch(runtime, transition)
    }

    pub fn answer(&self, runtime: ProjectRuntime, input: &AnswerRequest) -> Result<Session> {
        let transition = planning::answer(&runtime, input)?;
        self.launch(runtime, transition)
    }

    pub fn cancel(&self, runtime: &ProjectRuntime, input: &SessionRequest) -> Result<Session> {
        let session = planning::cancel(runtime, input)?;
        if let Some(job) = self
            .0
            .state
            .lock()
            .map_err(|_| anyhow!("planning jobs lock poisoned"))?
            .jobs
            .get(&session.round_id)
        {
            let _ = job.stop.send(true);
        }
        (self.0.notify)(&input.project_id);
        Ok(session)
    }

    pub fn adopt(&self, runtime: &ProjectRuntime, input: &SessionRequest) -> Result<Session> {
        let session = planning::adopt(runtime, input)?;
        (self.0.notify)(&input.project_id);
        Ok(session)
    }

    fn launch(&self, runtime: ProjectRuntime, transition: Transition) -> Result<Session> {
        if !transition.launch {
            return Ok(transition.session);
        }
        let key = RoundKey::from(&transition.session);
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| anyhow!("planning jobs lock poisoned"))?;
        let previous_id = state
            .jobs
            .iter()
            .find(|(_, job)| {
                job.project_id == key.project_id
                    && job.draft_id == transition.session.input.draft_id
            })
            .map(|(id, _)| id.clone());
        let failure = if state.stopped {
            Some((Status::Interrupted, "OBJECT_PLANNING_HOST_STOPPING"))
        } else if previous_id.is_none() && state.jobs.len() >= self.0.limit {
            Some((Status::Failed, "OBJECT_PLANNING_CAPACITY: another planning round is still active; start a new request when it ends"))
        } else {
            None
        };
        if let Some((status, error)) = failure {
            drop(state);
            round::finish(&runtime, &key, status, error)?;
            (self.0.notify)(&key.project_id);
            return planning::get(
                &runtime,
                &key.project_id,
                &transition.session.input.draft_id,
            )?
            .ok_or_else(|| anyhow!("OBJECT_PLANNING_NOT_FOUND"));
        }
        let (stop, receiver) = watch::channel(false);
        let inner = self.0.clone();
        let round_id = key.round_id.clone();
        let project_id = key.project_id.clone();
        let draft_id = transition.session.input.draft_id.clone();
        // A fast answer/restart keeps its slot, but cannot spawn before the old child is gone.
        let previous = previous_id
            .and_then(|id| state.jobs.remove(&id))
            .map(|job| {
                let _ = job.stop.send(true);
                job.handle
            });
        // Register under the same lock used by shutdown before the worker can complete.
        let handle = tokio::spawn(async move {
            work(inner, runtime, key, receiver, previous).await;
        });
        state.jobs.insert(
            round_id,
            Job {
                project_id,
                draft_id,
                stop,
                handle,
            },
        );
        drop(state);
        (self.0.notify)(&transition.session.project_id);
        Ok(transition.session)
    }

    /// Stop accepting work, cancel every owned job, and await factory/process disposal.
    pub async fn shutdown(&self) -> Result<()> {
        let jobs = {
            let mut state = self
                .0
                .state
                .lock()
                .map_err(|_| anyhow!("planning jobs lock poisoned"))?;
            state.stopped = true;
            state
                .jobs
                .drain()
                .map(|(_, job)| {
                    let _ = job.stop.send(true);
                    job.handle
                })
                .collect::<Vec<_>>()
        };
        let mut failed = false;
        for handle in jobs {
            failed |= handle.await.is_err();
        }
        let state = self
            .0
            .state
            .lock()
            .map_err(|_| anyhow!("planning jobs lock poisoned"))?;
        anyhow::ensure!(
            !failed && !state.persistence_failed,
            "OBJECT_PLANNING_SHUTDOWN_FAILED"
        );
        Ok(())
    }
}

async fn work(
    inner: Arc<Inner>,
    runtime: ProjectRuntime,
    key: RoundKey,
    stop: watch::Receiver<bool>,
    previous: Option<JoinHandle<()>>,
) {
    if let Some(previous) = previous {
        if previous.await.is_err() {
            if let Ok(mut state) = inner.state.lock() {
                state.persistence_failed = true;
            }
        }
    }
    let factory = inner.factory.clone();
    let prepared_runtime = runtime.clone();
    let prepared_key = key.clone();
    let prepared_stop = stop.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        anyhow::ensure!(!*prepared_stop.borrow(), "OBJECT_PLANNING_INTERRUPTED");
        round::context(&prepared_runtime, &prepared_key)?;
        factory(&prepared_runtime, &prepared_key)
    })
    .await;
    let result = match prepared {
        Ok(Ok(launch)) => executor::run(&runtime, &key, launch, stop.clone(), &inner.notify).await,
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("OBJECT_PLANNING_PREPARATION_FAILED".into()),
    };
    let persisted = if let Err(error) = result {
        let status = if *stop.borrow() {
            Status::Interrupted
        } else {
            Status::Failed
        };
        round::finish(&runtime, &key, status, &error)
    } else {
        Ok(())
    };
    if let Ok(mut state) = inner.state.lock() {
        state.persistence_failed |= persisted.is_err();
        state.jobs.remove(&key.round_id);
    }
    (inner.notify)(&key.project_id);
}

#[cfg(test)]
#[path = "object_task_planning_service_tests.rs"]
mod tests;
