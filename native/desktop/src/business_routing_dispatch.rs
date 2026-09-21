use crate::{
    data_dispatch, game_runtime, setup_runtime, template_runtime, workflow_runtime, Backend,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::Emitter;

use super::task_runtime_handles;

pub(crate) async fn call(
    app: tauri::AppHandle,
    state: Arc<Backend>,
    method: String,
    input: Option<Value>,
    source: &str,
) -> Result<Value, String> {
    if state.closing.load(Ordering::SeqCst) {
        return Err("应用正在退出".into());
    }
    if method.starts_with("migration.") {
        let input = input.unwrap_or_else(|| json!({}));
        crate::business_catalog::validate(&method, &input).map_err(|(_, message)| message)?;
        let backend = state.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            crate::migration_runtime::call(&backend.root, &backend.project_storage, &method, &input)
                .map_err(|error| format!("{error:#}"))
        })
        .await
        .map_err(|error| error.to_string())??;
        return Ok(outcome.publish(
            || {
                let _ = app.emit("beaver:changed", ());
            },
            || state.scheduler.wake(),
        ));
    }
    if method == "task.framework" {
        let input = input.unwrap_or_else(|| json!({}));
        crate::business_catalog::validate(&method, &input).map_err(|(_, message)| message)?;
        let id = input["id"].as_str().ok_or("Missing task ID")?.to_owned();
        let handles = task_runtime_handles(
            &state.project_storage,
            state.store.clone(),
            &state.root,
            &id,
        )?;
        let result = beaver_core::framework::call(
            handles.store,
            handles.files,
            id,
            None,
            input["request"].clone(),
        )
        .await
        .map_err(|error| error.to_string())?;
        let _ = app.emit("beaver:changed", ());
        return Ok(result);
    }
    if method == "task.callback" {
        let input = input.unwrap_or_else(|| json!({}));
        crate::business_catalog::validate(&method, &input).map_err(|(_, message)| message)?;
        let id = input["id"].as_str().ok_or("Missing task ID")?.to_owned();
        let handles = task_runtime_handles(
            &state.project_storage,
            state.store.clone(),
            &state.root,
            &id,
        )?;
        let params = json!({"threadId":input["threadId"],"turnId":input["turnId"],"arguments":input["request"]});
        let result =
            beaver_core::task_callback_runtime::dynamic(handles.store, handles.files, &id, &params)
                .await
                .map_err(|error| error.to_string())?;
        let _ = app.emit("beaver:changed", ());
        return Ok(result);
    }
    if method == "task.callbackState" {
        let input = input.unwrap_or_else(|| json!({}));
        crate::business_catalog::validate(&method, &input).map_err(|(_, message)| message)?;
        let id = input["id"].as_str().ok_or("Missing task ID")?.to_owned();
        let handles = task_runtime_handles(
            &state.project_storage,
            state.store.clone(),
            &state.root,
            &id,
        )?;
        return tauri::async_runtime::spawn_blocking(move || {
            let mut store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
            beaver_core::task_callback::business(&mut store, &method, &input)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?;
    }
    if method.starts_with("assetTask.") {
        return crate::asset_task_runtime::call(
            app,
            state,
            &method,
            input.unwrap_or_else(|| json!({})),
        )
        .await
        .map_err(|error| error.to_string());
    }
    if method.starts_with("validation.") {
        let backend = state.clone();
        let source = source.to_owned();
        let changed = !crate::validation_runtime::is_query(&method);
        let result = tauri::async_runtime::spawn_blocking(move || {
            crate::validation_runtime::call(
                &backend,
                &method,
                input.unwrap_or_else(|| json!({})),
                &source,
            )
        })
        .await
        .map_err(|error| error.to_string())?;
        if changed && result.is_ok() {
            state.scheduler.wake()?;
            let _ = app.emit("beaver:changed", ());
        }
        return result;
    }
    if method == "project.create"
        || method == "project.npr.install"
        || method.starts_with("workflow.")
    {
        let backend = state.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            workflow_runtime::call(&backend, &method, input.unwrap_or_else(|| json!({})))
        })
        .await
        .map_err(|e| e.to_string())?;
        let _ = app.emit("beaver:changed", ());
        return result;
    }
    if matches!(method.as_str(), "screenshots" | "screenshot.capture") {
        return crate::asset_runtime::call(app, state, method, input).await;
    }
    if method == "game.cancelTemplates" {
        template_runtime::cancel(&state).map_err(|error| error.to_string())?;
        return Ok(Value::Null);
    }
    if matches!(
        method.as_str(),
        "game.prepareTemplates" | "game.importTemplates"
    ) {
        let backend = state.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            template_runtime::prepare(
                &backend,
                method == "game.importTemplates",
                input
                    .as_ref()
                    .and_then(|v| v["path"].as_str())
                    .map(PathBuf::from),
            )
            .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?;
    }
    if method == "tools.cancelSetup" {
        state.setup.cancel().map_err(|error| error.to_string())?;
        return Ok(Value::Null);
    }
    if matches!(method.as_str(), "tools.setup" | "tools.install") {
        let backend = state.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            setup_runtime::prepare(&backend, app, &method, input.unwrap_or_else(|| json!({})))
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?;
    }
    if method == "game.play" {
        let backend = state.clone();
        let (executable, root) = tauri::async_runtime::spawn_blocking(move || {
            game_runtime::prepare_play(&backend, input.unwrap_or(Value::Null))
        })
        .await
        .map_err(|error| error.to_string())??;
        state
            .players
            .play(&executable, &root)
            .await
            .map_err(|error| error.to_string())?;
        return Ok(Value::Null);
    }
    if matches!(
        method.as_str(),
        "game.export" | "game.presets" | "game.verifyExport"
    ) {
        let backend = state.clone();
        let changed = method != "game.presets";
        let result = tauri::async_runtime::spawn_blocking(move || {
            game_runtime::call(&backend, &method, input.unwrap_or(Value::Null))
        })
        .await
        .map_err(|error| error.to_string())?;
        if changed {
            let _ = app.emit("beaver:changed", ());
        }
        return result;
    }
    if method == "tools.detect" {
        let backend = state.clone();
        return tauri::async_runtime::spawn_blocking(move || -> Result<Value, String> {
            let _operation = backend.operations.lock().map_err(|_| "工具操作锁不可用")?;
            let input = input.unwrap_or_else(|| json!({}));
            if !input.is_object() {
                return Err("工具检测参数无效".into());
            }
            let (configured, secrets) = {
                let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
                if backend.closing.load(Ordering::SeqCst) {
                    return Err("应用正在退出".into());
                }
                let settings = beaver_core::preferences::read(
                    &store,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))
                    .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let secrets: Vec<String> =
                    ["code", "review", "image", "speech", "music", "translation"]
                        .into_iter()
                        .filter_map(|capability| {
                            beaver_core::preferences::resolve(
                                &store,
                                &beaver_core::preferences::SystemVault,
                                &settings,
                                capability,
                            )
                            .ok()
                            .map(|provider| provider.key)
                        })
                        .collect();
                (
                    input
                        .get("tools")
                        .cloned()
                        .unwrap_or_else(|| settings["tools"].clone()),
                    secrets,
                )
            };
            beaver_core::tools::detect(&configured, &secrets).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?;
    }
    if matches!(
        method.as_str(),
        "task.answer" | "task.continue" | "task.interrupt"
    ) {
        return crate::task_control::call(app, state, method, input).await;
    }
    if method == "asset.import" {
        return crate::asset_runtime::call(app, state, method, input).await;
    }
    if matches!(method.as_str(), "chooseDirectory" | "chooseTool") {
        if state.closing.load(Ordering::SeqCst) {
            return Err("应用正在退出".into());
        }
        return tauri::async_runtime::spawn_blocking(move || {
            let dialog = rfd::FileDialog::new();
            let path = if method == "chooseDirectory" {
                dialog.set_title("选择项目目录").pick_folder()
            } else {
                dialog.set_title("选择工具程序").pick_file()
            };
            Ok(json!(path))
        })
        .await
        .map_err(|error| error.to_string())?;
    }
    let backend = state.clone();
    let changed = matches!(
        method.as_str(),
        "document.save"
            | "project.import"
            | "project.reassociate"
            | "project.unregister"
            | "project.create"
            | "settings.save"
            | "settings.importLocalCodex"
            | "settings.clearKey"
            | "task.rollback"
            | "task.accept"
            | "task.retryMerge"
            | "project.blueprint.save"
            | "project.overview.save"
            | "task.create"
            | "task.followup"
            | "task.delegate"
            | "task.dialogueRollback"
            | "task.direction"
            | "task.autonomy"
            | "task.approval"
            | "feature.add"
    );
    let notify = changed;
    let result =
        tauri::async_runtime::spawn_blocking(move || data_dispatch::call(&backend, method, input))
            .await
            .map_err(|e| e.to_string())?;
    if changed && result.is_ok() {
        state.scheduler.wake()?;
    }
    if notify && result.is_ok() {
        // Callback receipts are already committed. Notification failure must not
        // turn a successful mutation into an apparent retryable failure.
        let _ = app.emit("beaver:changed", ());
    }
    result
}
