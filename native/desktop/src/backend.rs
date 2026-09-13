use crate::{business_routing, instance};
use beaver_core::store::Store;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub(crate) struct Backend {
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) scheduler: beaver_core::scheduler::Scheduler,
    pub(crate) validation: beaver_core::validation::service::Service,
    pub(crate) players: beaver_core::game_play::Players,
    pub(crate) root: PathBuf,
    pub(crate) closing: AtomicBool,
    pub(crate) api_calls: Arc<tokio::sync::Semaphore>,
    pub(crate) operations: Mutex<()>,
    pub(crate) setup: beaver_core::tool_setup::Setup,
    pub(crate) setup_gate: Mutex<()>,
    pub(crate) template_gate: Mutex<()>,
    pub(crate) template_cancel: Mutex<Option<Arc<AtomicBool>>>,
    pub(crate) captures: Mutex<beaver_core::capture::Capture>,
    pub(crate) _instance: instance::Instance,
}

pub(crate) async fn business_call(
    app: tauri::AppHandle,
    state: Arc<Backend>,
    method: String,
    input: Option<Value>,
    source: &str,
) -> Result<Value, String> {
    use beaver_core::call_log;
    if crate::asset_task_runtime::transient(&method) {
        return business_routing::call(app, state, method, input, source).await;
    }
    let started = std::time::Instant::now();
    let raw = input.clone().unwrap_or_else(|| json!({}));
    let id = {
        let store = state.store.lock().map_err(|_| "数据库锁不可用")?;
        let task_id = if method.starts_with("task.") || method.starts_with("assetTask.") {
            raw["id"].as_str()
        } else {
            raw["taskId"].as_str()
        };
        let task = task_id.and_then(|id| store.get::<Value>("task", id).ok().flatten());
        let project_id = raw["projectId"]
            .as_str()
            .or_else(|| task.as_ref().and_then(|t| t["projectId"].as_str()))
            .or_else(|| {
                if method.starts_with("project.")
                    || method.starts_with("game.")
                    || method.starts_with("workflow.")
                    || method.starts_with("asset.")
                    || method.starts_with("document.")
                    || matches!(method.as_str(), "assets" | "screenshot.capture")
                {
                    raw["id"].as_str()
                } else {
                    None
                }
            });
        call_log::begin(&store, source, &method, task_id, project_id, &raw)
            .map_err(|e| e.to_string())?
    };
    let result = business_routing::call(app, state.clone(), method.clone(), input, source).await;
    if let Ok(store) = state.store.lock() {
        let output = match &result {
            Ok(v) => v.clone(),
            Err(e) => json!({"error":e}),
        };
        if let Ok(value) = &result {
            let task_id = if method.starts_with("task.") && value.get("projectId").is_some() {
                value["id"].as_str()
            } else {
                None
            };
            let project_id = value["projectId"].as_str().or_else(|| {
                if matches!(method.as_str(), "project.create" | "project.import") {
                    value["id"].as_str()
                } else {
                    None
                }
            });
            let _ = call_log::link(&store, &id, task_id, project_id);
        }
        // Never turn an already committed mutation into an apparent retryable failure.
        if call_log::finish(
            &store,
            &id,
            if result.as_ref().is_ok_and(|v| v["ok"] != false) {
                "succeeded"
            } else {
                "failed"
            },
            started.elapsed().as_millis() as u64,
            &output,
        )
        .is_err()
        {
            eprintln!("Beaver call-log completion write failed for {id}");
        }
    }
    result
}
