use super::*;
use beaver_core::{
    project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

fn host_root(parent: &Path) -> PathBuf {
    parent.join("host")
}

fn host_store(parent: &Path) -> anyhow::Result<Arc<Mutex<Store>>> {
    Ok(Arc::new(Mutex::new(Store::open(&host_root(parent))?)))
}

fn project_root(parent: &Path, project_id: &str) -> anyhow::Result<PathBuf> {
    let root = parent.join(project_id);
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "[application]\n")?;
    Ok(root)
}

fn register_project(
    store: &Arc<Mutex<Store>>,
    project_id: &str,
    root: &Path,
) -> anyhow::Result<Value> {
    let project = json!({
        "id": project_id,
        "name": "Game",
        "path": root.to_string_lossy().to_string()
    });
    store.lock().unwrap().put("project", project_id, &project)?;
    Ok(project)
}

fn initialize_project_store(root: &Path, project_id: &str, project: &Value) -> anyhow::Result<()> {
    let project_store = ProjectStore::initialize(root, project_id)?;
    project_store.store().put("project", project_id, project)?;
    Ok(())
}

fn open_project(
    temp: &tempfile::TempDir,
    project_id: &str,
) -> anyhow::Result<(
    PathBuf,
    Arc<Mutex<Store>>,
    ProjectStorageRouter,
    beaver_core::project_runtime::ProjectRuntime,
)> {
    let root = project_root(temp.path(), project_id)?;
    let host = host_store(temp.path())?;
    let project = register_project(&host, project_id, &root)?;
    initialize_project_store(&root, project_id, &project)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered(project_id)?;
    Ok((root, host, router, runtime))
}

#[test]
fn task_resources_and_text_use_project_workspace() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, _runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"产出资源","title":"资源任务"})),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let workspace = root.join(format!(".beaver/workspaces/{task_id}"));
    fs::write(workspace.join("generated.txt"), "项目资源")?;

    let resources = task_resource_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.resources",
        Some(json!({"id":task_id})),
    )?;
    assert_eq!(
        resources,
        json!([{"path":"generated.txt","origin":"修改","exists":true}])
    );
    assert_eq!(
        task_resource_operation(
            &router,
            host,
            &host_root(temp.path()),
            "task.resourceText",
            Some(json!({"id":task_id,"path":"generated.txt"})),
        )?,
        json!("项目资源")
    );
    Ok(())
}

#[test]
fn task_events_and_accept_use_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (_root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"等待认可","title":"认可任务"})),
    )?;
    let task_id = task["id"].as_str().unwrap();

    let events = task_events(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    )?;
    assert_eq!(events.as_array().unwrap().len(), 1);
    assert_eq!(events[0]["text"], "等待认可");

    let project_store = runtime.store();
    let mut completed = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    completed["status"] = json!("completed");
    project_store
        .lock()
        .unwrap()
        .put("task", task_id, &completed)?;
    complete_task_action(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.accept",
        Some(json!({"id":task_id})),
    )?;

    let accepted = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    assert_eq!(accepted["accepted"], true);
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    Ok(())
}

#[test]
fn task_callback_state_uses_project_store_without_host_shadow() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (_root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"查看回调状态","title":"回调状态"})),
    )?;
    let task_id = task["id"].as_str().unwrap();
    host.lock().unwrap().put(
        "task",
        task_id,
        &json!({"id":task_id,"projectId":"host-shadow","prompt":"错误副本","status":"running"}),
    )?;

    let state = task_callback_state(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    )?;

    assert_eq!(state["task"]["projectId"], "project-a");
    assert_eq!(state["task"]["prompt"], "查看回调状态");
    assert_eq!(
        host.lock().unwrap().get::<Value>("task", task_id)?.unwrap()["projectId"],
        "host-shadow"
    );
    assert_eq!(
        runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", task_id)?
            .unwrap()["projectId"],
        "project-a"
    );
    Ok(())
}

#[test]
fn task_reveal_resolves_project_workspace_from_relative_record() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, _runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"打开目录","title":"打开目录"})),
    )?;
    let task_id = task["id"].as_str().unwrap();

    let target = resolve_task_reveal(
        &router,
        host,
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    )?;

    assert!(!target.select);
    assert_eq!(
        target.path,
        fs::canonicalize(root.join(format!(".beaver/workspaces/{task_id}")))?
    );
    Ok(())
}

#[test]
fn task_actions_keep_legacy_host_tasks_available() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let root = project_root(temp.path(), "legacy-project")?;
    register_project(&host, "legacy-project", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    host.lock().unwrap().put(
        "task",
        "legacy-completed",
        &json!({
            "id":"legacy-completed",
            "projectId":"legacy-project",
            "status":"completed",
            "changes":[],
            "accepted":false
        }),
    )?;

    complete_task_action(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.accept",
        Some(json!({"id":"legacy-completed"})),
    )?;

    assert_eq!(
        host.lock()
            .unwrap()
            .get::<Value>("task", "legacy-completed")?
            .unwrap()["accepted"],
        true
    );
    Ok(())
}

#[test]
fn task_settings_use_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (_root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"调整任务","title":"调整任务"})),
    )?;
    let task_id = task["id"].as_str().unwrap();

    let autonomy = task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.autonomy",
        Some(json!({"id":task_id,"askRatio":30})),
    )?;
    assert_eq!(autonomy["askRatio"], json!(30));
    assert_eq!(autonomy["effectiveAskRatio"], json!(30));
    task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.approval",
        Some(json!({"id":task_id,"autoAccept":true})),
    )?;
    task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.direction",
        Some(json!({"id":task_id,"direction":"visual"})),
    )?;

    let project_store = runtime.store();
    let updated = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    assert_eq!(updated["askRatio"], json!(30));
    assert_eq!(updated["autoAccept"], true);
    assert_eq!(updated["direction"], "visual");
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    Ok(())
}

#[test]
fn task_settings_keep_legacy_host_tasks_available() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let root = project_root(temp.path(), "legacy-project")?;
    register_project(&host, "legacy-project", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    host.lock().unwrap().put(
        "task",
        "legacy-task",
        &json!({
            "id":"legacy-task",
            "projectId":"legacy-project",
            "status":"running",
            "accepted":false
        }),
    )?;

    task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.direction",
        Some(json!({"id":"legacy-task","direction":"engineering"})),
    )?;
    task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.autonomy",
        Some(json!({"id":"legacy-task","askRatio":70})),
    )?;

    let updated = host
        .lock()
        .unwrap()
        .get::<Value>("task", "legacy-task")?
        .unwrap();
    assert_eq!(updated["direction"], "engineering");
    assert_eq!(updated["askRatio"], json!(70));
    assert!(router.runtime_for_task("legacy-task").is_err());
    Ok(())
}

#[test]
fn task_settings_require_registered_project_or_legacy_host_task() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let router = ProjectStorageRouter::new(host.clone());

    let error = task_setting_operation(
        &router,
        host,
        &host_root(temp.path()),
        "task.autonomy",
        Some(json!({"id":"missing-task","askRatio":30})),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("任务未注册：missing-task"), "{error}");
    Ok(())
}

#[test]
fn document_save_uses_project_store_and_indexes_route() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    fs::write(root.join("story.md"), "before")?;
    let revision = beaver_core::documents::read_document(&root, "story.md")?["revision"]
        .as_str()
        .map(str::to_owned)
        .unwrap();

    let task = document_save_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({
            "id":"project-a",
            "path":"story.md",
            "text":"after",
            "revision":revision
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();

    assert_eq!(task["status"], "completed");
    assert_eq!(fs::read_to_string(root.join("story.md"))?, "after");
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    assert!(runtime
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_some());
    assert_eq!(task["workspace"], format!(".beaver/workspaces/{task_id}"));
    assert_eq!(router.runtime_for_task(task_id)?.project_id(), "project-a");
    Ok(())
}

#[test]
fn document_save_keeps_legacy_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "legacy-project")?;
    let host = host_store(temp.path())?;
    register_project(&host, "legacy-project", &root)?;
    fs::write(root.join("story.md"), "before")?;
    let revision = beaver_core::documents::read_document(&root, "story.md")?["revision"]
        .as_str()
        .map(str::to_owned)
        .unwrap();
    let router = ProjectStorageRouter::new(host.clone());

    let task = document_save_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({
            "id":"legacy-project",
            "path":"story.md",
            "text":"after",
            "revision":revision
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();

    assert_eq!(task["status"], "completed");
    assert_eq!(fs::read_to_string(root.join("story.md"))?, "after");
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_some());
    assert!(router.runtime_for_task(task_id).is_err());
    Ok(())
}

#[test]
fn feature_add_uses_project_store_when_project_is_open() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (_root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = feature_add_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"projectId":"project-a","featureId":"crafting"})),
    )?;
    let task_id = task["id"].as_str().unwrap();
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    let project_task = runtime
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .expect("feature task is stored in project database");
    assert_eq!(project_task["projectId"], "project-a");
    assert_eq!(router.runtime_for_task(task_id)?.project_id(), "project-a");
    Ok(())
}

#[test]
fn feature_add_keeps_legacy_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "legacy-project")?;
    let host = host_store(temp.path())?;
    register_project(&host, "legacy-project", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let task = feature_add_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"projectId":"legacy-project","featureId":"crafting"})),
    )?;
    let task_id = task["id"].as_str().unwrap();
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_some());
    assert!(router.runtime_for_task(task_id).is_err());
    Ok(())
}

#[path = "data_dispatch_project_tests.rs"]
mod project_tests;

#[path = "data_dispatch_task_tests.rs"]
mod task_tests;

#[path = "data_dispatch_task_lock_tests.rs"]
mod task_lock_tests;

#[path = "data_dispatch_state_tests.rs"]
mod state_tests;

#[path = "data_dispatch_callback_tests.rs"]
mod callback_tests;
