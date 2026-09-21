use super::*;

fn local(root: &Path, id: &str, task: &str) -> Result<()> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let storage = ProjectStore::initialize(root, id)?;
    storage
        .store()
        .put("project", id, &json!({"id":id,"path":root}))?;
    storage.store().put(
        "task",
        task,
        &json!({"id":task,"projectId":id,"status":"completed"}),
    )?;
    Ok(())
}

#[test]
fn reassociation_checks_revision_identity_and_active_ownership() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let old = temp.path().join("old");
    let new = temp.path().join("new");
    let wrong = temp.path().join("wrong");
    local(&old, "p", "old-task")?;
    local(&new, "p", "new-task")?;
    local(&wrong, "other", "other-task")?;
    let host = Arc::new(Mutex::new(Store::open(&data)?));
    let original = json!({"id":"p","path":old,"name":"Retained host metadata"});
    host.lock().unwrap().put("project", "p", &original)?;
    let router = ProjectStorageRouter::new(host.clone());
    let expected = original["path"].as_str().unwrap();
    let active = router.open_registered("p")?;
    assert!(router.reassociate("p", "stale", &new, &data).is_err());
    assert!(router.reassociate("p", expected, &wrong, &data).is_err());
    let error = router.reassociate("p", expected, &new, &data).unwrap_err();
    assert!(error.to_string().contains("运行时引用"));
    assert_eq!(
        host.lock().unwrap().get::<Value>("project", "p")?.unwrap(),
        original
    );
    assert!(router.runtime_for_task("old-task").is_ok());
    drop(active);
    let changed = router.reassociate("p", expected, &new, &data)?;
    assert_eq!(changed["name"], original["name"]);
    assert_eq!(changed["path"], json!(fs::canonicalize(&new)?));
    assert!(router.runtime_for_task("old-task").is_err());
    let opened = router.open_registered("p")?;
    assert_eq!(opened.project_root(), fs::canonicalize(&new)?);
    assert!(router.runtime_for_task("new-task").is_ok());
    assert!(old.join(layout::CONTROL_DIR).is_dir());
    Ok(())
}

#[test]
fn failed_host_write_preserves_runtime_and_task_routes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let old = temp.path().join("old");
    let new = temp.path().join("new");
    local(&old, "p", "old-task")?;
    local(&new, "p", "new-task")?;
    let host = Arc::new(Mutex::new(Store::open(&data)?));
    let original = json!({"id":"p","path":old});
    host.lock().unwrap().put("project", "p", &original)?;
    let router = ProjectStorageRouter::new(host.clone());
    drop(router.open_registered("p")?);
    host.lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only = ON")?;
    let expected = original["path"].as_str().unwrap();
    assert!(router.reassociate("p", expected, &new, &data).is_err());
    assert_eq!(
        host.lock().unwrap().get::<Value>("project", "p")?,
        Some(original.clone())
    );
    assert_eq!(
        router.runtime_for_task("old-task")?.project_root(),
        fs::canonicalize(&old)?
    );
    assert!(router.runtime_for_task("new-task").is_err());
    host.lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only = OFF")?;
    router.reassociate("p", expected, &new, &data)?;
    assert!(router.runtime_for_task("old-task").is_err());
    Ok(())
}

#[test]
fn reassociation_recovers_moved_project_without_rewriting_local_entity() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let old = temp.path().join("old");
    let new = temp.path().join("moved");
    local(&old, "p", "history")?;
    let host = Arc::new(Mutex::new(Store::open(&data)?));
    let original = json!({"id":"p","path":old});
    host.lock().unwrap().put("project", "p", &original)?;
    fs::rename(&old, &new)?;
    let router = ProjectStorageRouter::new(host);
    router.reassociate("p", original["path"].as_str().unwrap(), &new, &data)?;
    let runtime = router.open_registered("p")?;
    assert_eq!(
        runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>("project", "p")?
            .unwrap()["path"],
        original["path"]
    );
    assert!(router.runtime_for_task("history").is_ok());
    Ok(())
}
