use crate::Backend;
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) fn serve(
    backend: Arc<Backend>,
    window: String,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    tauri::async_runtime::spawn(async move {
        let origin = request
            .headers()
            .get("origin")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("http://tauri.localhost");
        if ![
            "http://tauri.localhost",
            "https://tauri.localhost",
            "tauri://localhost",
        ]
        .contains(&origin)
        {
            responder.respond(
                tauri::http::Response::builder()
                    .status(403)
                    .body(Vec::new())
                    .expect("static response"),
            );
            return;
        }
        let result = async {
            if request.method() != "GET" && request.method() != "HEAD" {
                return Ok(tauri::http::Response::builder()
                    .status(405)
                    .body(Vec::new())?);
            }
            crate::asset_task_windows::authorize_image(&backend, &window, request.uri().path())
                .map_err(anyhow::Error::msg)?;
            let asset = read(backend, &request).await?;
            let mut response = tauri::http::Response::builder()
                .status(asset.status)
                .header("Content-Type", asset.content_type)
                .header("Content-Length", asset.length)
                .header("Accept-Ranges", "bytes")
                .header("Cache-Control", "no-store")
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Security-Policy", "default-src 'none'");
            if let Some(range) = asset.content_range {
                response = response.header("Content-Range", range);
            }
            Ok::<_, anyhow::Error>(
                response
                    .header("Access-Control-Allow-Origin", origin)
                    .header(
                        "Access-Control-Expose-Headers",
                        "Content-Range, Accept-Ranges",
                    )
                    .body(asset.bytes)?,
            )
        }
        .await;
        responder.respond(result.unwrap_or_else(|_| {
            tauri::http::Response::builder()
                .status(404)
                .header("Access-Control-Allow-Origin", origin)
                .body(Vec::new())
                .expect("static response")
        }));
    });
}

async fn read(
    backend: Arc<Backend>,
    request: &tauri::http::Request<Vec<u8>>,
) -> Result<beaver_core::assets::AssetResponse> {
    let uri = request.uri().path().to_owned();
    let head = request.method() == "HEAD";
    if uri.starts_with("/asset-task/") {
        let bytes = crate::asset_task_preview::image(backend, &uri).await?;
        return Ok(beaver_core::assets::AssetResponse {
            status: 200,
            content_type: "image/png".into(),
            length: bytes.len() as u64,
            content_range: None,
            bytes: if head { Vec::new() } else { bytes },
        });
    }
    let range = request
        .headers()
        .get("range")
        .and_then(|s| s.to_str().ok())
        .map(str::to_owned);
    tauri::async_runtime::spawn_blocking(move || {
        let path = if uri.starts_with("/validation/") {
            crate::validation_runtime::media(&backend, &uri)?
        } else {
            resolve_asset_path(&backend, &uri)?
        };
        beaver_core::assets::read_asset(&path, range.as_deref(), head)
    })
    .await?
}

fn resolve_asset_path(backend: &Backend, uri: &str) -> Result<PathBuf> {
    let handles = asset_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        uri,
    )
    .map_err(anyhow::Error::msg)?;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(beaver_core::assets::resolve_asset(
        &store,
        &handles.files,
        uri,
    )?)
}

fn asset_runtime_handles(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<beaver_core::store::Store>>,
    host_root: &Path,
    uri: &str,
) -> std::result::Result<crate::business_routing::TaskRuntimeHandles, String> {
    let id = uri
        .strip_prefix('/')
        .and_then(|path| path.split_once('/'))
        .map(|(id, _)| id)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "资源地址缺少所属标识".to_owned())?;
    if let Some(task_id) = id.strip_prefix("task-") {
        return crate::business_routing::task_runtime_handles(
            router, host_store, host_root, task_id,
        );
    }
    crate::business_routing::project_runtime_handles(router, host_store, host_root, id)
}

#[cfg(test)]
mod tests {
    use super::asset_runtime_handles;
    use beaver_core::{
        project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
    };
    use serde_json::json;
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    #[test]
    fn project_asset_uses_open_project_runtime() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_root = temp.path().join("project");
        fs::create_dir(&project_root)?;
        fs::write(project_root.join("project.godot"), "config_version=5\n")?;
        drop(ProjectStore::initialize(&project_root, "project-a")?);
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        host.lock().unwrap().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":project_root}),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        let runtime = router.open_registered("project-a")?;
        runtime.store().lock().unwrap().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":runtime.project_root()}),
        )?;
        let handles =
            asset_runtime_handles(&router, host, temp.path(), "/project-a/references/a.png")
                .map_err(anyhow::Error::msg)?;
        let store = handles.store.lock().unwrap();
        let path = beaver_core::assets::resolve_asset(
            &store,
            &handles.files,
            "/project-a/references/a.png",
        )?;
        assert_eq!(path, runtime.project_root().join("references/a.png"));
        Ok(())
    }

    #[test]
    fn task_asset_uses_task_project_runtime() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_root = temp.path().join("project");
        fs::create_dir(&project_root)?;
        fs::write(project_root.join("project.godot"), "config_version=5\n")?;
        drop(ProjectStore::initialize(&project_root, "project-a")?);
        fs::create_dir_all(project_root.join(".beaver/workspaces/task-a"))?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        host.lock().unwrap().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":project_root}),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        let runtime = router.open_registered("project-a")?;
        runtime.store().lock().unwrap().put(
            "task",
            "task-a",
            &json!({
                "id":"task-a",
                "projectId":"project-a",
                "workspace":".beaver/workspaces/task-a"
            }),
        )?;
        router.index_task(&json!({
            "id":"task-a",
            "projectId":"project-a",
            "workspace":".beaver/workspaces/task-a"
        }))?;
        let output_dir = project_root.join(".beaver/workspaces/task-a/output");
        fs::create_dir_all(&output_dir)?;
        fs::write(output_dir.join("model.glb"), b"model")?;
        let handles =
            asset_runtime_handles(&router, host, temp.path(), "/task-task-a/output/model.glb")
                .map_err(anyhow::Error::msg)?;
        let store = handles.store.lock().unwrap();
        let path = beaver_core::assets::resolve_asset(
            &store,
            &handles.files,
            "/task-task-a/output/model.glb",
        )?;
        assert_eq!(
            fs::canonicalize(path)?,
            fs::canonicalize(project_root.join(".beaver/workspaces/task-a/output/model.glb"))?
        );
        Ok(())
    }

    #[test]
    fn closed_local_project_cannot_fall_back_to_host_assets() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_root = temp.path().join("project");
        fs::create_dir(&project_root)?;
        fs::write(project_root.join("project.godot"), "config_version=5\n")?;
        drop(ProjectStore::initialize(&project_root, "project-a")?);
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        host.lock().unwrap().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":project_root}),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        let error =
            asset_runtime_handles(&router, host, temp.path(), "/project-a/references/a.png")
                .err()
                .expect("closed local project must reject fallback");
        assert!(error.contains("禁止回退到宿主存储"));
        Ok(())
    }

    #[test]
    fn legacy_project_keeps_host_asset_fallback() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project_root = temp.path().join("legacy");
        fs::create_dir(&project_root)?;
        fs::create_dir_all(project_root.join("references"))?;
        fs::write(project_root.join("references/a.png"), b"legacy")?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        host.lock().unwrap().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":project_root}),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        let handles =
            asset_runtime_handles(&router, host, temp.path(), "/project-a/references/a.png")
                .map_err(anyhow::Error::msg)?;
        let store = handles.store.lock().unwrap();
        let path = beaver_core::assets::resolve_asset(
            &store,
            &handles.files,
            "/project-a/references/a.png",
        )?;
        assert_eq!(
            fs::canonicalize(path)?,
            fs::canonicalize(project_root.join("references/a.png"))?
        );
        Ok(())
    }
}
