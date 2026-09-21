use crate::{business_routing::project_runtime_handles, Backend};
use beaver_core::{project_storage_router::ProjectStorageRouter, store::Store};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex},
};
use tauri::Emitter;

fn import_asset_paths(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &std::path::Path,
    closing: &AtomicBool,
    id: &str,
    paths: &[PathBuf],
) -> Result<(), String> {
    let handles = project_runtime_handles(router, host_store, host_root, id)?;
    {
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        if closing.load(Ordering::SeqCst) {
            return Err("应用正在退出".into());
        }
        beaver_core::assets::import_root(&store, id).map_err(|error| error.to_string())?;
    }
    let mut store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
    if closing.load(Ordering::SeqCst) {
        return Err("应用正在退出".into());
    }
    beaver_core::assets::import_files(&mut store, handles.files.as_ref(), id, paths)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn save_capture_for_project(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &std::path::Path,
    closing: &AtomicBool,
    id: &str,
    png: &[u8],
) -> anyhow::Result<String> {
    let handles =
        project_runtime_handles(router, host_store, host_root, id).map_err(anyhow::Error::msg)?;
    let mut store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    anyhow::ensure!(!closing.load(Ordering::SeqCst), "应用正在退出");
    Ok(beaver_core::assets::save_capture(
        &mut store,
        handles.files.as_ref(),
        id,
        png,
    )?)
}

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
            let handles = project_runtime_handles(
                &backend.project_storage,
                backend.store.clone(),
                &backend.root,
                id,
            )
            .map_err(anyhow::Error::msg)?;
            {
                let store = handles
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
            Ok(json!(save_capture_for_project(
                &backend.project_storage,
                backend.store.clone(),
                &backend.root,
                &backend.closing,
                id,
                &png,
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
            // Never hold the project/database lock while waiting for a human.
            let Some(paths) = supplied_paths
                .or_else(|| rfd::FileDialog::new().set_title("导入素材").pick_files())
            else {
                return Ok(Value::Null);
            };
            import_asset_paths(
                &backend.project_storage,
                backend.store.clone(),
                &backend.root,
                &backend.closing,
                &id,
                &paths,
            )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use beaver_core::{
        project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
    };
    use serde_json::json;
    use std::{fs, sync::atomic::AtomicBool};

    fn local_project(
        temp: &tempfile::TempDir,
        project_id: &str,
    ) -> anyhow::Result<(
        std::path::PathBuf,
        Arc<Mutex<Store>>,
        ProjectStorageRouter,
        beaver_core::project_runtime::ProjectRuntime,
    )> {
        let root = temp.path().join(project_id);
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "[application]\n")?;
        let host_root = temp.path().join("host");
        let host = Arc::new(Mutex::new(Store::open(&host_root)?));
        let project = json!({
            "id": project_id,
            "name": "Game",
            "path": root.to_string_lossy().to_string()
        });
        host.lock().unwrap().put("project", project_id, &project)?;
        let project_store = ProjectStore::initialize(&root, project_id)?;
        project_store.store().put("project", project_id, &project)?;
        drop(project_store);
        let router = ProjectStorageRouter::new(host.clone());
        let runtime = router.open_registered(project_id)?;
        Ok((root, host, router, runtime))
    }

    #[test]
    fn local_import_and_capture_use_project_runtime_storage() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_id = "44444444-4444-4444-4444-444444444444";
        let (root, host, router, runtime) = local_project(&temp, project_id)?;
        let source = temp.path().join("source.txt");
        fs::write(&source, "imported")?;
        let closing = AtomicBool::new(false);

        import_asset_paths(
            &router,
            host.clone(),
            &temp.path().join("host"),
            &closing,
            project_id,
            std::slice::from_ref(&source),
        )
        .map_err(anyhow::Error::msg)?;
        let references = fs::read_dir(root.join("references"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(references.len(), 1);
        assert_eq!(fs::read_to_string(&references[0])?, "imported");

        let relative = save_capture_for_project(
            &router,
            host.clone(),
            &temp.path().join("host"),
            &closing,
            project_id,
            b"\x89PNG\r\n\x1a\nmock",
        )?;
        assert!(relative.starts_with("references/capture-"));
        assert!(root.join(&relative).is_file());
        assert!(runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>("project", project_id)?
            .is_some());
        assert!(!temp.path().join("host/references").exists());
        Ok(())
    }

    #[test]
    fn local_asset_operations_reject_host_fallback_when_runtime_is_closed() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_id = "55555555-5555-5555-5555-555555555555";
        let (_root, host, router, runtime) = local_project(&temp, project_id)?;
        drop(runtime);
        router.close(project_id)?;
        let source = temp.path().join("source.txt");
        fs::write(&source, "imported")?;
        let error = import_asset_paths(
            &router,
            host.clone(),
            &temp.path().join("host"),
            &AtomicBool::new(false),
            project_id,
            std::slice::from_ref(&source),
        )
        .unwrap_err();
        assert!(error.contains("禁止回退到宿主存储"), "{error}");

        let error = save_capture_for_project(
            &router,
            host,
            &temp.path().join("host"),
            &AtomicBool::new(false),
            project_id,
            b"\x89PNG\r\n\x1a\nmock",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("禁止回退到宿主存储"), "{error}");
        Ok(())
    }

    #[test]
    fn legacy_asset_operations_keep_host_storage_compatibility() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_id = "66666666-6666-6666-6666-666666666666";
        let root = temp.path().join("legacy");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "[application]\n")?;
        let host_root = temp.path().join("host");
        let host = Arc::new(Mutex::new(Store::open(&host_root)?));
        host.lock().unwrap().put(
            "project",
            project_id,
            &json!({
                "id": project_id,
                "name": "Legacy",
                "path": root.to_string_lossy().to_string()
            }),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        let source = temp.path().join("legacy.txt");
        fs::write(&source, "legacy")?;

        import_asset_paths(
            &router,
            host.clone(),
            &host_root,
            &AtomicBool::new(false),
            project_id,
            std::slice::from_ref(&source),
        )
        .map_err(anyhow::Error::msg)?;
        assert_eq!(fs::read_dir(root.join("references"))?.count(), 1);
        let relative = save_capture_for_project(
            &router,
            host,
            &host_root,
            &AtomicBool::new(false),
            project_id,
            b"\x89PNG\r\n\x1a\nmock",
        )?;
        assert!(root.join(relative).is_file());
        Ok(())
    }
}
