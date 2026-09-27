//! Host-owned bounded jobs survive dropped callers and are joined during shutdown.
use crate::{
    codex_read_only::Prepared,
    object_task_title::{self, Request},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{oneshot, watch, Semaphore},
    task::JoinHandle,
};

struct Inner {
    gate: Arc<Semaphore>,
    stop: watch::Sender<bool>,
    jobs: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct Service(Arc<Inner>);

impl Default for Service {
    fn default() -> Self {
        Self(Arc::new(Inner {
            gate: Arc::new(Semaphore::new(2)),
            stop: watch::channel(false).0,
            jobs: Mutex::new(Vec::new()),
        }))
    }
}

impl Service {
    pub async fn suggest(
        &self,
        request: Request,
        prepare: impl FnOnce() -> anyhow::Result<Prepared> + Send + 'static,
    ) -> Result<String, String> {
        request.validate()?;
        let receiver = {
            let mut jobs = self
                .0
                .jobs
                .lock()
                .map_err(|_| "OBJECT_TASK_TITLE_LOCK_FAILED")?;
            if *self.0.stop.borrow() {
                return Err("OBJECT_TASK_TITLE_STOPPED".into());
            }
            let permit = self
                .0
                .gate
                .clone()
                .try_acquire_owned()
                .map_err(|_| "OBJECT_TASK_TITLE_BUSY")?;
            // Finished handles hold no process; retain only jobs that still need joining.
            jobs.retain(|job| !job.is_finished());
            let stop = self.0.stop.subscribe();
            let (sender, receiver) = oneshot::channel();
            jobs.push(tokio::spawn(async move {
                let _permit = permit;
                let result = async {
                    let launch = tokio::task::spawn_blocking(prepare)
                        .await
                        .map_err(|error| error.to_string())?
                        .map_err(|error| error.to_string())?;
                    object_task_title::run(launch, request, stop, Duration::from_secs(60)).await
                }
                .await;
                let _ = sender.send(result);
            }));
            receiver
        };
        receiver
            .await
            .map_err(|_| "OBJECT_TASK_TITLE_WORKER_FAILED")?
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        let jobs = {
            let mut jobs = self
                .0
                .jobs
                .lock()
                .map_err(|_| "OBJECT_TASK_TITLE_LOCK_FAILED")?;
            self.0.stop.send_replace(true);
            std::mem::take(&mut *jobs)
        };
        let mut failure = None;
        for job in jobs {
            if let Err(error) = job.await {
                failure = Some(error.to_string());
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
