use super::{ensure_host_media_fallback_allowed, feedback_task_to_index, refresh_open_task_routes};
use beaver_core::{
    project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::json;
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[test]
fn feedback_task_to_index_reads_created_task_and_replay_result() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    store.put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"project-a","status":"queued"}),
    )?;
    let result = Ok(json!({"feedbackId":"feedback-a","taskId":"task-a"}));
    let task = feedback_task_to_index(&store, &result)?.expect("feedback task");
    assert_eq!(task["id"], "task-a");
    assert_eq!(task["projectId"], "project-a");
    Ok(())
}

#[test]
fn feedback_task_to_index_rejects_success_without_task_record() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    let result = Ok(json!({"feedbackId":"feedback-a","taskId":"missing"}));
    let error = feedback_task_to_index(&store, &result).unwrap_err();
    assert!(error.to_string().contains("验证反馈任务不存在：missing"));
    Ok(())
}

#[test]
fn refresh_open_task_routes_discovers_tasks_added_after_project_open() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_root = temp.path().join("project");
    fs::create_dir(&project_root)?;
    fs::write(project_root.join("project.godot"), "config_version=5\n")?;
    drop(ProjectStore::initialize(&project_root, "project-a")?);
    let host_path = temp.path().join("host");
    let host = Arc::new(Mutex::new(Store::open(&host_path)?));
    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":project_root}),
    )?;
    let router = ProjectStorageRouter::new(host);
    let runtime = router.open_registered("project-a")?;
    runtime.store().lock().unwrap().put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"project-a","status":"queued"}),
    )?;
    assert!(router.runtime_for_task("task-a").is_err());
    refresh_open_task_routes(&router)?;
    assert_eq!(router.runtime_for_task("task-a")?.project_id(), "project-a");
    Ok(())
}

#[test]
fn host_media_fallback_rejects_registered_local_project() -> anyhow::Result<()> {
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
    let router = ProjectStorageRouter::new(host);
    let error = ensure_host_media_fallback_allowed(&router, "project-a").unwrap_err();
    assert!(error.to_string().contains("禁止从宿主读取验证媒体"));
    Ok(())
}

#[test]
fn host_media_fallback_does_not_parse_stale_path_for_open_runtime() -> anyhow::Result<()> {
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
    router.open_registered("project-a")?;
    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({
            "id":"project-a",
            "path":temp.path().join("missing-project")
        }),
    )?;

    let error = ensure_host_media_fallback_allowed(&router, "project-a").unwrap_err();
    assert!(error
        .to_string()
        .contains("项目本地存储已打开，禁止从宿主读取验证媒体"));
    Ok(())
}

#[test]
fn host_media_fallback_allows_registered_legacy_project() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_root = temp.path().join("project");
    fs::create_dir(&project_root)?;
    fs::write(project_root.join("project.godot"), "config_version=5\n")?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":project_root}),
    )?;
    let router = ProjectStorageRouter::new(host);
    ensure_host_media_fallback_allowed(&router, "project-a")?;
    Ok(())
}

#[test]
fn host_media_fallback_rejects_unregistered_project() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    let router = ProjectStorageRouter::new(host);
    let error = ensure_host_media_fallback_allowed(&router, "orphan").unwrap_err();
    assert!(error.to_string().contains("项目未登记"));
    Ok(())
}
