use super::{call_log_context, query_runtime_handles, task_runtime_handles};
use beaver_core::{
    project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

fn host_store(parent: &Path) -> anyhow::Result<Arc<Mutex<Store>>> {
    Ok(Arc::new(Mutex::new(Store::open(&parent.join("host"))?)))
}

fn project_root(parent: &Path, project_id: &str) -> anyhow::Result<PathBuf> {
    let root = parent.join(project_id);
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    Ok(root)
}

fn register_project(
    store: &Arc<Mutex<Store>>,
    project_id: &str,
    root: &Path,
) -> anyhow::Result<()> {
    store.lock().unwrap().put(
        "project",
        project_id,
        &json!({"id":project_id,"path":root.to_string_lossy().to_string()}),
    )?;
    Ok(())
}

#[test]
fn task_runtime_handles_prefers_project_route_over_matching_host_task() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project.store().put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"project-a","status":"running"}),
    )?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    host.lock().unwrap().put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"host-project","status":"running"}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let handles =
        task_runtime_handles(&router, host, temp.path(), "task-a").map_err(anyhow::Error::msg)?;
    assert!(handles.project_routed);
    let task = handles
        .store
        .lock()
        .unwrap()
        .get::<Value>("task", "task-a")?
        .unwrap();
    assert_eq!(task["projectId"], "project-a");
    let state = beaver_core::task_callback::business(
        &mut handles.store.lock().unwrap(),
        "task.callbackState",
        &json!({"id":"task-a"}),
    )?;
    assert_eq!(state["task"]["projectId"], "project-a");

    drop(handles);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn task_runtime_handles_refreshes_open_project_before_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    runtime.store().lock().unwrap().put(
        "task",
        "late-task",
        &json!({"id":"late-task","projectId":"project-a","status":"running"}),
    )?;
    register_project(&host, "project-a", &temp.path().join("missing"))?;
    host.lock().unwrap().put(
        "task",
        "late-task",
        &json!({"id":"late-task","projectId":"project-a","status":"running"}),
    )?;

    let handles = task_runtime_handles(&router, host, temp.path(), "late-task")
        .map_err(anyhow::Error::msg)?;
    assert!(handles.project_routed);
    let task = handles
        .store
        .lock()
        .unwrap()
        .get::<Value>("task", "late-task")?
        .unwrap();
    assert_eq!(task["projectId"], "project-a");
    assert_eq!(
        router.runtime_for_task("late-task")?.project_id(),
        "project-a"
    );
    Ok(())
}

#[test]
fn task_runtime_handles_keeps_legacy_host_tasks_available() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let root = project_root(temp.path(), "legacy-project")?;
    register_project(&host, "legacy-project", &root)?;
    host.lock().unwrap().put(
        "task",
        "legacy-task",
        &json!({"id":"legacy-task","projectId":"legacy-project","status":"running"}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());

    let handles = task_runtime_handles(&router, host, temp.path(), "legacy-task")
        .map_err(anyhow::Error::msg)?;
    assert!(!handles.project_routed);
    let task = handles
        .store
        .lock()
        .unwrap()
        .get::<Value>("task", "legacy-task")?
        .unwrap();
    assert_eq!(task["projectId"], "legacy-project");
    Ok(())
}

#[test]
fn query_runtime_handles_routes_project_queries_without_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project
        .store()
        .put("calls", "record-a", &json!({"id":"record-a"}))?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let handles = query_runtime_handles(
        &router,
        host.clone(),
        temp.path(),
        &json!({"projectId":"project-a"}),
    )
    .map_err(anyhow::Error::msg)?;
    assert!(handles
        .store
        .lock()
        .unwrap()
        .get::<Value>("calls", "record-a")?
        .is_some());
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("calls", "record-a")?
        .is_none());
    drop(runtime);
    Ok(())
}

#[test]
fn query_runtime_handles_keeps_unfiltered_legacy_queries_on_host_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    host.lock()
        .unwrap()
        .put("calls", "record-legacy", &json!({"id":"record-legacy"}))?;
    let router = ProjectStorageRouter::new(host.clone());
    let handles = query_runtime_handles(&router, host, temp.path(), &json!({}))
        .map_err(anyhow::Error::msg)?;
    assert!(handles
        .store
        .lock()
        .unwrap()
        .get::<Value>("calls", "record-legacy")?
        .is_some());
    Ok(())
}

#[test]
fn query_runtime_handles_rejects_task_and_project_filter_conflict() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project.store().put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"project-a","status":"running"}),
    )?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let error = match query_runtime_handles(
        &router,
        host,
        temp.path(),
        &json!({"taskId":"task-a","projectId":"project-b"}),
    ) {
        Ok(_) => anyhow::bail!("conflicting task and project filters must fail"),
        Err(error) => error,
    };
    assert_eq!(error, "任务与项目标识不一致");
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn call_log_context_routes_project_task_to_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project.store().put(
        "task",
        "task-a",
        &json!({"id":"task-a","projectId":"project-a","status":"running"}),
    )?;
    drop(project);

    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    let context = call_log_context(
        &router,
        host.clone(),
        temp.path(),
        "task.callbackState",
        &json!({"id":"task-a"}),
    )
    .map_err(anyhow::Error::msg)?;
    let call_id = {
        let store = context.handles.store.lock().unwrap();
        beaver_core::call_log::begin(
            &store,
            "test",
            "task.callbackState",
            context.task_id.as_deref(),
            context.project_id.as_deref(),
            &json!({"id":"task-a"}),
        )?
    };
    {
        let store = context.handles.store.lock().unwrap();
        beaver_core::call_log::finish(&store, &call_id, "succeeded", 1, &json!({"ok":true}))?;
        let page = beaver_core::call_log::query(&store, &json!({"taskId":"task-a"}))?;
        assert_eq!(page["records"].as_array().unwrap().len(), 1);
        assert_eq!(page["records"][0]["projectId"], "project-a");
    }
    let host_page = {
        let store = host.lock().unwrap();
        beaver_core::call_log::query(&store, &json!({"taskId":"task-a"}))?
    };
    assert!(host_page["records"].as_array().unwrap().is_empty());

    drop(context);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn call_log_context_rejects_closed_local_project_instead_of_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let _project = ProjectStore::initialize(&root, "project-a")?;
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());

    let error = match call_log_context(
        &router,
        host,
        temp.path(),
        "task.create",
        &json!({"id":"task-a","projectId":"project-a"}),
    ) {
        Ok(_) => anyhow::bail!("closed local projects must not fall back to the host store"),
        Err(error) => error,
    };
    assert!(error.contains("项目本地存储未打开，禁止回退到宿主存储"));
    Ok(())
}

#[path = "tests/storage.rs"]
mod storage;

#[test]
fn unregistered_closed_project_rejects_host_shadow_task() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "removed")?;
    let project = ProjectStore::initialize(&root, "removed")?;
    let task = json!({"id":"task","projectId":"removed","status":"queued"});
    project.store().put("task", "task", &task)?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "removed", &root)?;
    host.lock().unwrap().put("task", "task", &task)?;
    let router = ProjectStorageRouter::new(host.clone());
    let active = router.open_registered("removed")?;
    host.lock().unwrap().remove("project", "removed")?;
    router.close_unregistered()?;
    assert!(
        task_runtime_handles(&router, host.clone(), temp.path(), "task")
            .map_err(anyhow::Error::msg)?
            .project_routed
    );
    drop(active);
    router.close_unregistered()?;
    let error = match task_runtime_handles(&router, host.clone(), temp.path(), "task") {
        Ok(_) => anyhow::bail!("unregistered task must not use host shadow"),
        Err(error) => error,
    };
    assert!(error.contains("项目未登记"), "{error}");
    assert_eq!(
        host.lock().unwrap().get::<Value>("task", "task")?,
        Some(task)
    );
    assert!(root.join(".beaver").is_dir());
    Ok(())
}

#[path = "tests/export.rs"]
mod export;
