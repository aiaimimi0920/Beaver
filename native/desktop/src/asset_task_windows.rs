use crate::Backend;
use beaver_core::asset_task;
use serde_json::{json, Value};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub(crate) const PREFIX: &str = "asset-task-";

pub(crate) fn authorize_image(backend: &Backend, label: &str, path: &str) -> Result<(), String> {
    if label == "main" {
        return Ok(());
    }
    let id = label.strip_prefix(PREFIX).ok_or("未知窗口")?;
    if let Some(target) = path
        .strip_prefix("/asset-task/")
        .and_then(|p| p.split('/').next())
    {
        return if target == id {
            Ok(())
        } else {
            Err("画面属于另一任务".into())
        };
    }
    let resource = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .ok_or("缺少资源标识")?;
    let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
    let bound = asset_task::get(&store, id).map_err(|e| e.to_string())?;
    let project = if let Some(task) = resource.strip_prefix("task-") {
        store
            .get::<Value>("task", task)
            .map_err(|e| e.to_string())?
            .ok_or("任务不存在")?["projectId"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    } else {
        resource.to_owned()
    };
    if project != bound.project_id {
        return Err("资源属于另一项目".into());
    }
    Ok(())
}

pub(crate) fn authorize(
    backend: &Backend,
    label: &str,
    method: &str,
    input: &Value,
) -> Result<(), String> {
    if label == "main" {
        return Ok(());
    }
    let id = label.strip_prefix(PREFIX).ok_or("未知窗口")?;
    let target = input["id"].as_str().ok_or("缺少任务标识")?;
    let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
    let bound = asset_task::get(&store, id).map_err(|e| e.to_string())?;
    if method.starts_with("assetTask.") && method != "assetTask.open" {
        return if target == id {
            Ok(())
        } else {
            Err("制作窗口只能控制当前绑定任务".into())
        };
    }
    if !method.starts_with("task.") && method != "assetTask.open" {
        return Err("制作窗口不提供此操作".into());
    }
    let task: Value = store
        .get("task", target)
        .map_err(|e| e.to_string())?
        .ok_or("任务不存在")?;
    if task["projectId"] != bound.project_id {
        return Err("关联任务属于另一项目".into());
    }
    Ok(())
}

pub(crate) async fn open(
    app: &tauri::AppHandle,
    backend: &Backend,
    id: &str,
) -> Result<Value, String> {
    asset_task::validate_id(id).map_err(|e| e.to_string())?;
    let title = {
        let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
        let state = asset_task::get(&store, id).map_err(|e| e.to_string())?;
        let task: Value = store
            .get("task", id)
            .map_err(|e| e.to_string())?
            .ok_or("任务不存在")?;
        let project: Value = store
            .get("project", &state.project_id)
            .map_err(|e| e.to_string())?
            .ok_or("项目不存在")?;
        format!(
            "{} · {} · 资产制作",
            project["name"].as_str().unwrap_or("Beaver"),
            task["title"].as_str().unwrap_or("任务")
        )
    };
    let label = format!("{PREFIX}{id}");
    let url = WebviewUrl::App(format!("index.html?assetTask={id}").into());
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = (|| -> tauri::Result<()> {
            let window = if let Some(window) = handle.get_webview_window(&label) {
                window
            } else {
                WebviewWindowBuilder::new(&handle, &label, url)
                    .title(title)
                    .inner_size(1440.0, 900.0)
                    .min_inner_size(1080.0, 720.0)
                    .decorations(false)
                    .build()?
            };
            window.show()?;
            window.unminimize()?;
            window.set_focus()
        })();
        let _ = tx.send(result.map_err(|e| e.to_string()));
    })
    .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())??;
    Ok(json!({"taskId":id,"label":format!("{PREFIX}{id}")}))
}

pub(crate) fn release(app: &tauri::AppHandle, label: &str) {
    let Some(id) = label.strip_prefix(PREFIX) else {
        return;
    };
    let Some(backend) = app.try_state::<std::sync::Arc<Backend>>() else {
        return;
    };
    if let Ok(client) = backend.scheduler.asset_client(id) {
        let subscriber = format!("window-{id}");
        tauri::async_runtime::spawn(async move {
            let _ = client
                .call("status", json!({"subscriber":subscriber,"release":true}))
                .await;
        });
    }
}
