use crate::{
    executor::{Control, Execution, Outcome},
    files::Files,
    scheduler::Factory,
    store::Store,
    validation::task_gate,
};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;

pub(crate) async fn execute(
    task: Value,
    factory: Factory,
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    cancelled: Arc<AtomicBool>,
    mut input: mpsc::Receiver<Control>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> (String, Outcome) {
    let id = task["id"].as_str().unwrap_or_default().to_owned();
    let factory_task = task.clone();
    let prepared = tokio::task::spawn_blocking(move || factory(&factory_task)).await;
    let outcome = match prepared {
        Ok(Ok(launch)) if !cancelled.load(Ordering::SeqCst) => {
            let validate_only = task_gate::validation_only(&task);
            let session = if let Some(request) = launch.blender.filter(|_| !validate_only) {
                let session_store = store.clone();
                let session_task = task.clone();
                let session_cancelled = cancelled.clone();
                match tokio::task::spawn_blocking(move || {
                    request.start_logged(session_store, &session_task, &session_cancelled)
                })
                .await
                {
                    Ok(Ok(session)) => Some(session),
                    Ok(Err(error)) => {
                        return (
                            id,
                            if cancelled.load(Ordering::SeqCst) {
                                Outcome::Interrupted
                            } else {
                                Outcome::Failed(error.to_string())
                            },
                        )
                    }
                    Err(_) => {
                        return (
                            id,
                            Outcome::Failed("Blender preparation worker failed".into()),
                        )
                    }
                }
            } else {
                None
            };
            let outcome = if cancelled.load(Ordering::SeqCst) {
                Outcome::Interrupted
            } else if validate_only {
                Outcome::Completed
            } else if let Some(command) = launch.command {
                Execution {
                    store: store.clone(),
                    task_id: id.clone(),
                    model: launch.model,
                    prompt: launch.prompt,
                    ask_user_tool: launch.ask_user_tool,
                    max_minutes: launch.max_minutes,
                    secrets: launch.secrets,
                }
                .run_controlled(command, &mut input, Default::default())
                .await
            } else {
                Outcome::Failed("Missing task execution command".into())
            };
            if let Some(session) = session {
                let _ = tokio::task::spawn_blocking(move || drop(session)).await;
            }
            if outcome == Outcome::Completed {
                crate::validation::task_control::run(
                    store,
                    files,
                    id.clone(),
                    launch.godot,
                    cancelled,
                    &mut input,
                    changed,
                )
                .await
            } else {
                outcome
            }
        }
        Ok(Ok(_)) => Outcome::Interrupted,
        Ok(Err(error)) => Outcome::Failed(error),
        Err(_) => Outcome::Failed("任务环境准备异常".into()),
    };
    (id, outcome)
}
