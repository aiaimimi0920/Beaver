use crate::{
    asset_agent,
    asset_sessions::Sessions,
    executor::{Control, Execution, Outcome},
    scheduler::Factory,
    store::Store,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::mpsc;

pub async fn run(
    task: Value,
    store: Arc<Mutex<Store>>,
    root: PathBuf,
    factory: Factory,
    sessions: Sessions,
    cancelled: Arc<AtomicBool>,
    input: mpsc::Receiver<Control>,
) -> (String, Outcome) {
    let id = task["id"].as_str().unwrap_or_default().to_owned();
    let result = execute(&task, store, root, factory, sessions, &cancelled, input).await;
    let outcome = match result {
        _ if cancelled.load(Ordering::SeqCst) => Outcome::Interrupted,
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failed(error),
    };
    (id, outcome)
}

async fn execute(
    task: &Value,
    store: Arc<Mutex<Store>>,
    root: PathBuf,
    factory: Factory,
    sessions: Sessions,
    cancelled: &Arc<AtomicBool>,
    input: mpsc::Receiver<Control>,
) -> Result<Outcome, String> {
    if task["decompose"] == true && task["plan"].is_object() {
        return Ok(Outcome::Completed);
    }
    let id = task["id"].as_str().ok_or("Missing task ID")?;
    let mut factory_task = task.clone();
    // Only an existing registered session can supply a retained port to HOME.
    if let Ok(client) = sessions.client(id) {
        if let Some(port) = sessions.reserve(id)? {
            factory_task["retainedBlenderPort"] = json!(port);
            factory_task["assetTask"] = json!(true);
        }
        drop(client);
    }
    let launch = tokio::task::spawn_blocking(move || factory(&factory_task))
        .await
        .map_err(|_| "任务环境准备异常")??;
    if cancelled.load(Ordering::SeqCst) {
        return Ok(Outcome::Interrupted);
    }
    if let Some(request) = launch.blender {
        if sessions.reserve(id)?.is_some() {
            return Err("Duplicate Blender launch refused".into());
        }
        let session_store = store.clone();
        let session_task = task.clone();
        let session_cancelled = cancelled.clone();
        let session = tokio::task::spawn_blocking(move || {
            request.start_logged(session_store, &session_task, &session_cancelled)
        })
        .await
        .map_err(|_| "Blender preparation worker failed")?
        .map_err(|e| e.to_string())?;
        sessions.insert(id, session)?;
    }
    let execution = Execution {
        store: store.clone(),
        task_id: id.into(),
        model: launch.model,
        prompt: launch.prompt,
        ask_user_tool: launch.ask_user_tool,
        max_minutes: launch.max_minutes,
        secrets: launch.secrets,
    };
    let asset = sessions.client(id).ok().map(|client| asset_agent::Context {
        store: store.clone(),
        root,
        client,
    });
    let outcome = if cancelled.load(Ordering::SeqCst) {
        Outcome::Interrupted
    } else if let Some(asset) = &asset {
        execution
            .run_asset(launch.command, input, asset.clone())
            .await
    } else {
        execution.run(launch.command, input).await
    };
    if let Some(asset) = asset {
        if let Err(error) = asset.checkpoint().await {
            if let Ok(db) = store.lock() {
                let _ = db.event(
                    id,
                    &crate::asset_task::now(),
                    "assetRecovery",
                    &format!("自动保存失败；保留现场，最近保存点之后的修改尚无恢复保证：{error}"),
                );
            }
        }
    }
    Ok(outcome)
}
