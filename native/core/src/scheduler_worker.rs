use crate::{
    asset_agent,
    asset_sessions::Sessions,
    executor::{Control, Execution, Outcome},
    scheduler::RuntimeFactory,
    scheduler_runtime::TaskRuntime,
    validation::task_gate,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

pub(crate) async fn execute(
    task: Value,
    runtime: TaskRuntime,
    factory: RuntimeFactory,
    sessions: Sessions,
    cancelled: Arc<AtomicBool>,
    mut input: mpsc::Receiver<Control>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> (String, Outcome) {
    let id = task["id"].as_str().unwrap_or_default().to_owned();
    let store = runtime.store();
    let files = runtime.files();
    let result = async {
        let validate_only = task_gate::validation_only(&task);
        if !validate_only && task["decompose"] == true && task["plan"].is_object() {
            return Ok(Outcome::Completed);
        }
        let mut factory_task = task.clone();
        // A retained task-owned session supplies the existing HOME bridge port.
        if !validate_only && sessions.client(&id).is_ok() {
            if let Some(port) = sessions.reserve(&id)? {
                factory_task["retainedBlenderPort"] = json!(port);
                factory_task["assetTask"] = json!(true);
            }
        }
        let factory_runtime = runtime.clone();
        let launch = tokio::task::spawn_blocking(move || factory(&factory_task, &factory_runtime))
            .await
            .map_err(|_| "任务环境准备异常")??;
        if cancelled.load(Ordering::SeqCst) {
            return Ok(Outcome::Interrupted);
        }
        if !validate_only
            && crate::external_run_contract::enabled(&task) != launch.external.is_some()
        {
            return Err("Execution mode does not match the configured scheduler backend".into());
        }
        if let Some(request) = launch.blender.filter(|_| !validate_only) {
            if sessions.reserve(&id)?.is_some() {
                return Err("Duplicate Blender launch refused".into());
            }
            let session_store = store.clone();
            let session_files = files.clone();
            let session_task = task.clone();
            let session_cancelled = cancelled.clone();
            let session = tokio::task::spawn_blocking(move || {
                request.start_logged(
                    session_store,
                    session_files,
                    &session_task,
                    &session_cancelled,
                )
            })
            .await
            .map_err(|_| "Blender preparation worker failed")?
            .map_err(|error| error.to_string())?;
            sessions.insert(&id, session)?;
        }
        let asset = sessions
            .client(&id)
            .ok()
            .map(|client| asset_agent::Context {
                store: store.clone(),
                files: files.clone(),
                client,
            });
        let outcome = if cancelled.load(Ordering::SeqCst) {
            Outcome::Interrupted
        } else if validate_only {
            Outcome::Completed
        } else if let Some(external) = launch.external {
            external
                .run(
                    &task,
                    store.clone(),
                    files.clone(),
                    launch.prompt,
                    cancelled.clone(),
                    &mut input,
                )
                .await
        } else if let Some(command) = launch.command {
            Execution {
                store: store.clone(),
                files: files.clone(),
                task_id: id.clone(),
                model: launch.model,
                prompt: launch.prompt,
                ask_user_tool: launch.ask_user_tool,
                max_minutes: launch.max_minutes,
                secrets: launch.secrets,
            }
            .run_controlled(command, &mut input, Default::default(), asset.clone())
            .await
        } else {
            Outcome::Failed("Missing task execution command".into())
        };
        if let Some(asset) = asset {
            if let Err(error) = asset.checkpoint().await {
                if let Ok(db) = store.lock() {
                    let _ = db.event(
                        &id,
                        &crate::asset_task::now(),
                        "assetRecovery",
                        &format!(
                            "自动保存失败；保留现场，最近保存点之后的修改尚无恢复保证：{error}"
                        ),
                    );
                }
            }
        }
        if outcome == Outcome::Completed {
            Ok(crate::validation::task_control::run(
                store,
                files,
                id.clone(),
                launch.godot,
                cancelled.clone(),
                &mut input,
                changed,
            )
            .await)
        } else {
            Ok(outcome)
        }
    }
    .await;
    let outcome = match result {
        _ if cancelled.load(Ordering::SeqCst) => Outcome::Interrupted,
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failed(error),
    };
    (id, outcome)
}
