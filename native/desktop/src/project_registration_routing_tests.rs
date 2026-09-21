use super::*;
use beaver_core::{project_storage::ProjectStore, project_storage_router::ProjectStorageRouter};
use serde_json::json;

#[test]
fn registration_log_context_uses_host_without_retaining_or_opening_project() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("project.godot"), "config_version=5\n")?;
    drop(ProjectStore::initialize(&root, "p")?);
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    host.lock()
        .unwrap()
        .put("project", "p", &json!({"id":"p","path":root}))?;
    let router = ProjectStorageRouter::new(host.clone());
    for method in [
        "project.unregister",
        "project.reassociate",
        "project.storage.status",
    ] {
        drop(router.open_registered("p")?);
        let context = call_log_context(
            &router,
            host.clone(),
            temp.path(),
            method,
            &json!({"id":"p"}),
        )
        .map_err(anyhow::Error::msg)?;
        assert!(Arc::ptr_eq(&context.handles.store, &host));
        assert_eq!(context.project_id.as_deref(), Some("p"));
        router.close("p")?;
        host.lock().unwrap().put(
            "project",
            "p",
            &json!({"id":"p","path":temp.path().join("offline")}),
        )?;
        let offline = call_log_context(
            &router,
            host.clone(),
            temp.path(),
            method,
            &json!({"id":"p"}),
        )
        .map_err(anyhow::Error::msg)?;
        assert!(Arc::ptr_eq(&offline.handles.store, &host));
        assert!(router.runtimes()?.is_empty());
        assert!(call_log_context(
            &router,
            host.clone(),
            temp.path(),
            method,
            &json!({"id":"p","projectId":"other"})
        )
        .is_err());
        host.lock()
            .unwrap()
            .put("project", "p", &json!({"id":"p","path":root}))?;
    }
    Ok(())
}
