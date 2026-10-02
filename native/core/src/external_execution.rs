//! Scheduler-owned external execution uses the same validation and merge path.
use crate::{
    executor::{Control, Outcome},
    external_runs::ExternalRegistry,
    files::Files,
    store::Store,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::mpsc;

pub struct ExternalLaunch {
    pub registry: ExternalRegistry,
    pub blender_path: Option<PathBuf>,
}

impl ExternalLaunch {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn run(
        self,
        task: &Value,
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        prompt: String,
        cancelled: Arc<AtomicBool>,
        input: &mut mpsc::Receiver<Control>,
    ) -> Outcome {
        let (live, mut finish) = match self.registry.register(
            task,
            store,
            files,
            prompt,
            self.blender_path,
            cancelled.clone(),
        ) {
            Ok(run) => run,
            Err(_) if self.registry.is_closing() || cancelled.load(Ordering::SeqCst) => {
                cancelled.store(true, Ordering::SeqCst);
                return Outcome::Interrupted;
            }
            Err(error) => return Outcome::Failed(error.to_string()),
        };
        let minutes = task["maxMinutes"].as_u64().unwrap_or(0);
        let deadline = tokio::time::sleep(Duration::from_secs(if minutes == 0 {
            86400 * 365
        } else {
            minutes.saturating_mul(60)
        }));
        tokio::pin!(deadline);
        let outcome = loop {
            if cancelled.load(Ordering::SeqCst) {
                break Outcome::Interrupted;
            }
            tokio::select! {
                result = &mut finish => break result.unwrap_or_else(|_| Outcome::Failed("External finish channel closed".into())),
                _ = live.stopped.notified() => break Outcome::Interrupted,
                _ = &mut deadline, if minutes != 0 => {
                    cancelled.store(true, Ordering::SeqCst);
                    break Outcome::Interrupted;
                }
                control = input.recv() => match control {
                    Some(Control::Steer { reply, .. }) => { let _ = reply.send(Err("External execution accepts updates through its explicit run contract".into())); }
                    Some(Control::Interrupt) | None => {
                        cancelled.store(true, Ordering::SeqCst);
                        break Outcome::Interrupted;
                    }
                }
            }
        };
        let status = if cancelled.load(Ordering::SeqCst) || outcome == Outcome::Interrupted {
            "revoked"
        } else {
            "finished"
        };
        if let Err(error) = live
            .close(status)
            .await
            .and_then(|()| self.registry.remove_live(&live))
        {
            return Outcome::Failed(format!("External execution cleanup failed: {error}"));
        }
        if cancelled.load(Ordering::SeqCst) {
            Outcome::Interrupted
        } else {
            outcome
        }
    }
}
