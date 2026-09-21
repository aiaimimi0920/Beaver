use super::*;
use crate::project_storage::ProjectStore;

fn local(root: &std::path::Path) -> Result<()> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let storage = ProjectStore::initialize(root, "p")?;
    storage
        .store()
        .put("project", "p", &json!({"id":"p","path":root}))?;
    storage.store().put(
        "task",
        "history",
        &json!({"id":"history","projectId":"p","status":"running"}),
    )?;
    Ok(())
}

#[test]
fn unregister_checks_path_and_drains_active_storage_without_deleting_history() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    local(&root)?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    let registration = json!({"id":"p","path":root});
    host.lock().unwrap().put("project", "p", &registration)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("p")?;
    let gate = router.work_gate("p");
    let path = registration["path"].as_str().unwrap();
    assert!(router.unregister("p", "stale").is_err());
    assert!(gate.acquire()?.is_some());
    assert_eq!(
        router.unregister("p", path)?,
        json!({"id":"p","path":root,"draining":true})
    );
    assert!(gate.acquire()?.is_none());
    assert!(host.lock().unwrap().get::<Value>("project", "p")?.is_none());
    assert!(router.runtime_for_task("history").is_ok());
    runtime.store().lock().unwrap().put(
        "task",
        "history",
        &json!({"id":"history","projectId":"p","status":"completed"}),
    )?;
    assert_eq!(router.close_unregistered()?.retained, vec!["p"]);
    drop(runtime);
    assert_eq!(router.close_unregistered()?.closed, vec!["p"]);
    assert!(router.runtime_for_task("history").is_err());
    let reopened = ProjectStore::open(&root, "p")?;
    assert_eq!(
        reopened.store().get::<Value>("task", "history")?.unwrap()["status"],
        "completed"
    );
    assert_eq!(
        fs::read_to_string(root.join("project.godot"))?,
        "config_version=5\n"
    );
    Ok(())
}

#[test]
fn failed_unregister_preserves_registration_runtime_and_routes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    local(&root)?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    let registration = json!({"id":"p","path":root});
    host.lock().unwrap().put("project", "p", &registration)?;
    let router = ProjectStorageRouter::new(host.clone());
    drop(router.open_registered("p")?);
    host.lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only = ON")?;
    let path = registration["path"].as_str().unwrap();
    assert!(router.unregister("p", path).is_err());
    assert_eq!(
        host.lock().unwrap().get::<Value>("project", "p")?,
        Some(registration.clone())
    );
    assert!(router.runtime_for_task("history").is_ok());
    host.lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only = OFF")?;
    assert_eq!(router.unregister("p", path)?["draining"], false);
    assert!(router.runtime_for_task("history").is_err());
    assert!(ProjectStore::open(&root, "p")?
        .store()
        .get::<Value>("task", "history")?
        .is_some());
    Ok(())
}

#[test]
fn unregister_offline_legacy_project_preserves_host_history_and_other_projects() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("offline");
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    let registration = json!({"id":"p","path":root});
    let history = json!({"id":"history","projectId":"p","status":"queued"});
    host.lock().unwrap().put("project", "p", &registration)?;
    host.lock()
        .unwrap()
        .put("project", "other", &json!({"id":"other"}))?;
    host.lock().unwrap().put("task", "history", &history)?;
    let router = ProjectStorageRouter::new(host.clone());
    assert_eq!(
        router.unregister("p", registration["path"].as_str().unwrap())?["draining"],
        false
    );
    assert_eq!(
        host.lock().unwrap().get::<Value>("task", "history")?,
        Some(history)
    );
    assert_eq!(router.registered_project_ids()?, vec!["other"]);
    assert!(!root.exists());
    assert!(router
        .unregister("p", registration["path"].as_str().unwrap())
        .is_err());
    Ok(())
}
