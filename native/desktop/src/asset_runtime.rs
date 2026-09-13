use crate::Backend;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::Emitter;

pub(crate) async fn call(
    app: tauri::AppHandle,
    state: Arc<Backend>,
    method: String,
    input: Option<Value>,
) -> Result<Value, String> {
    if matches!(method.as_str(), "screenshots" | "screenshot.capture") {
        let backend = state.clone();
        let changed = method == "screenshot.capture";
        let result = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<Value> {
            anyhow::ensure!(!backend.closing.load(Ordering::SeqCst), "应用正在退出");
            if method == "screenshots" {
                return backend
                    .captures
                    .lock()
                    .map_err(|_| anyhow::anyhow!("截图锁不可用"))?
                    .sources(&backend.closing);
            }
            let input = input
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("缺少截图参数"))?;
            let id = input["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少项目标识"))?;
            let source = input["source"]
                .as_str()
                .filter(|id| !id.is_empty() && id.len() <= 100)
                .ok_or_else(|| anyhow::anyhow!("缺少窗口来源"))?;
            {
                let store = backend
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
                beaver_core::assets::import_root(&store, id)?;
            }
            let png = backend
                .captures
                .lock()
                .map_err(|_| anyhow::anyhow!("截图锁不可用"))?
                .capture(source, &backend.closing)?;
            let mut store = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            anyhow::ensure!(!backend.closing.load(Ordering::SeqCst), "应用正在退出");
            Ok(json!(beaver_core::assets::save_capture(
                &mut store,
                &backend.root,
                id,
                &png
            )?))
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string());
        if changed && result.is_ok() {
            let _ = app.emit("beaver:changed", ());
        }
        return result;
    }
    if method == "asset.import" {
        let supplied_paths: Option<Vec<PathBuf>> = input
            .as_ref()
            .and_then(|v| v.get("paths"))
            .map(|v| serde_json::from_value(v.clone()).map_err(|_| "素材路径列表无效".to_owned()))
            .transpose()?;
        let id = input
            .as_ref()
            .and_then(|value| value["id"].as_str())
            .ok_or("缺少项目标识")?
            .to_owned();
        let backend = state.clone();
        let result = tauri::async_runtime::spawn_blocking(move || -> Result<Value, String> {
            {
                let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
                if backend.closing.load(Ordering::SeqCst) {
                    return Err("应用正在退出".into());
                }
                beaver_core::assets::import_root(&store, &id).map_err(|error| error.to_string())?;
            }
            // Never hold the project/database lock while waiting for a human.
            let Some(paths) = supplied_paths
                .or_else(|| rfd::FileDialog::new().set_title("导入素材").pick_files())
            else {
                return Ok(Value::Null);
            };
            let mut store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
            if backend.closing.load(Ordering::SeqCst) {
                return Err("应用正在退出".into());
            }
            beaver_core::assets::import_files(&mut store, &backend.root, &id, &paths)
                .map_err(|error| error.to_string())?;
            Ok(Value::Null)
        })
        .await
        .map_err(|error| error.to_string())?;
        // Partial imports remain visible even when a later file fails.
        let _ = app.emit("beaver:changed", ());
        return result;
    }
    Err(format!("Unknown asset operation: {method}"))
}
