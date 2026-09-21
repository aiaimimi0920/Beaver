#[allow(dead_code)]
mod project_storage_support;

use anyhow::Result;
use beaver_core::{
    project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
};
use project_storage_support::project;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

fn host(parent: &Path) -> Result<Arc<Mutex<Store>>> {
    Ok(Arc::new(Mutex::new(Store::open(&parent.join("host"))?)))
}

fn register(host: &Arc<Mutex<Store>>, id: &str, root: &Path, internal_id: &str) -> Result<()> {
    host.lock().unwrap().put(
        "project",
        id,
        &json!({
            "id": internal_id,
            "path": root.to_string_lossy().to_string()
        }),
    )?;
    Ok(())
}

fn task_project(
    parent: &Path,
    name: &str,
    project_id: &str,
    key: &str,
    task: Value,
) -> Result<PathBuf> {
    let root = project(parent, name, project_id)?;
    let owner = ProjectStore::open(&root, project_id)?;
    owner.store().put("task", key, &task)?;
    drop(owner);
    Ok(root)
}

fn task(id: &str, project_id: &str) -> Value {
    json!({"id": id, "projectId": project_id, "status": "queued"})
}

#[test]
fn opens_registered_project_and_routes_only_project_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = task_project(
        temp.path(),
        "game",
        "project-a",
        "task-a",
        task("task-a", "project-a"),
    )?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    host.lock()
        .unwrap()
        .put("task", "host-task", &task("host-task", "project-a"))?;

    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    assert_eq!(runtime.project_id(), "project-a");
    assert_eq!(runtime.project_root(), std::fs::canonicalize(&root)?);
    assert_eq!(router.runtime_for_task("task-a")?.project_id(), "project-a");
    assert!(router.runtime_for_task("host-task").is_err());
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn initializes_new_runtime_once_before_indexing_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host);
    let calls = AtomicUsize::new(0);

    router.open_registered_with("project-a", |store, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        store.put(
            "task",
            "initialized-task",
            &task("initialized-task", "project-a"),
        )
    })?;
    router.open_registered_with("project-a", |_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })?;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        router.runtime_for_task("initialized-task")?.project_id(),
        "project-a"
    );
    Ok(())
}

#[test]
fn failed_runtime_initialization_is_hidden_and_can_retry() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host);

    let error = router
        .open_registered_with("project-a", |_, _| anyhow::bail!("initialization failed"))
        .err()
        .expect("initialization failure")
        .to_string();
    assert!(error.contains("initialization failed"), "{error}");
    assert!(router.runtime_for_project("project-a").is_err());
    assert!(router.runtimes()?.is_empty());

    let runtime = router.open_registered_with("project-a", |_, _| Ok(()))?;
    assert_eq!(runtime.project_id(), "project-a");
    Ok(())
}

#[test]
fn refuses_unregistered_projects_and_registration_identity_mismatches() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    let router = ProjectStorageRouter::new(host.clone());
    assert!(router.open_registered("project-a").is_err());

    register(&host, "project-a", &root, "other")?;
    let error = router
        .open_registered("project-a")
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.contains("登记 key") || error.contains("内部 ID"),
        "{error}"
    );
    Ok(())
}

#[test]
fn refuses_manifest_identity_mismatch_before_opening_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-b", &root, "project-b")?;
    let error = ProjectStorageRouter::new(host)
        .open_registered("project-b")
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("项目清单 ID"), "{error}");
    Ok(())
}

#[test]
fn refuses_host_registration_while_project_storage_is_pending() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("game");
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    fs::create_dir(root.join(".beaver"))?;
    fs::write(root.join(".beaver/.storage-pending"), b"interrupted")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let error = ProjectStorageRouter::new(host)
        .open_registered("project-a")
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("初始化未完成"), "{error}");
    Ok(())
}

#[test]
fn reuses_open_runtime_but_rejects_same_id_at_another_root() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let other = project(temp.path(), "other", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host.clone());
    let first = router.open_registered("project-a")?;
    let first_store = first.store();
    let second = router.open_registered("project-a")?;
    let second_store = second.store();
    assert!(Arc::ptr_eq(&first_store, &second_store));
    drop(second_store);
    drop(second);

    register(
        &host,
        "project-a",
        &temp.path().join("missing"),
        "project-a",
    )?;
    let reopened = router.open_registered("project-a")?;
    assert!(Arc::ptr_eq(&first_store, &reopened.store()));
    drop(reopened);
    drop(first_store);
    drop(first);

    register(&host, "project-a", &other, "project-a")?;
    assert!(router.open_registered("project-a").is_err());
    router.close("project-a")?;
    Ok(())
}

#[test]
fn refresh_open_task_route_ignores_stale_host_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    runtime
        .store()
        .lock()
        .unwrap()
        .put("task", "late-task", &task("late-task", "project-a"))?;

    register(
        &host,
        "project-a",
        &temp.path().join("missing"),
        "project-a",
    )?;
    let refreshed = router.refresh_open_task_route("project-a", "late-task")?;
    assert_eq!(refreshed.project_id(), "project-a");
    assert_eq!(
        router.runtime_for_task("late-task")?.project_id(),
        "project-a"
    );

    drop(refreshed);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn rejects_task_identity_and_project_ownership_mismatches() -> Result<()> {
    for (key, value) in [
        ("task-key", task("other-id", "project-a")),
        ("task-a", task("task-a", "other-project")),
    ] {
        let temp = tempfile::tempdir()?;
        let root = task_project(temp.path(), "game", "project-a", key, value)?;
        let host = host(temp.path())?;
        register(&host, "project-a", &root, "project-a")?;
        assert!(ProjectStorageRouter::new(host)
            .open_registered("project-a")
            .is_err());
    }
    Ok(())
}

#[test]
fn rejects_duplicate_task_ids_across_projects() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let first_root = task_project(
        temp.path(),
        "first",
        "first",
        "same-task",
        task("same-task", "first"),
    )?;
    let second_root = task_project(
        temp.path(),
        "second",
        "second",
        "same-task",
        task("same-task", "second"),
    )?;
    let host = host(temp.path())?;
    register(&host, "first", &first_root, "first")?;
    register(&host, "second", &second_root, "second")?;
    let router = ProjectStorageRouter::new(host.clone());
    let first = router.open_registered("first")?;
    assert!(router.open_registered("second").is_err());
    assert_eq!(router.runtime_for_task("same-task")?.project_id(), "first");
    drop(first);
    router.close("first")?;
    Ok(())
}

#[test]
fn indexes_new_task_and_removes_routes_on_close() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host);
    let runtime = router.open_registered("project-a")?;
    let value = task("new-task", "project-a");
    runtime
        .store()
        .lock()
        .unwrap()
        .put("task", "new-task", &value)?;
    router.index_task(&value)?;
    assert_eq!(
        router.runtime_for_task("new-task")?.project_id(),
        "project-a"
    );
    drop(runtime);
    router.close("project-a")?;
    assert!(router.runtime_for_task("new-task").is_err());
    Ok(())
}

#[test]
fn close_waits_for_external_runtime_handles() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host);
    let runtime = router.open_registered("project-a")?;
    let store = runtime.store();
    drop(runtime);
    assert!(router.close("project-a").is_err());
    drop(store);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn close_all_closes_open_projects_and_removes_task_routes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let first_root = task_project(
        temp.path(),
        "first",
        "first",
        "first-task",
        task("first-task", "first"),
    )?;
    let second_root = task_project(
        temp.path(),
        "second",
        "second",
        "second-task",
        task("second-task", "second"),
    )?;
    let host = host(temp.path())?;
    register(&host, "first", &first_root, "first")?;
    register(&host, "second", &second_root, "second")?;
    let router = ProjectStorageRouter::new(host);
    let first = router.open_registered("first")?;
    let second = router.open_registered("second")?;
    drop(first);
    drop(second);
    router.close_all()?;
    assert!(router.runtimes()?.is_empty());
    assert!(router.runtime_for_task("first-task").is_err());
    assert!(router.runtime_for_task("second-task").is_err());
    Ok(())
}

#[test]
fn close_all_preserves_projects_with_external_handles() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "game", "project-a")?;
    let host = host(temp.path())?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host);
    let runtime = router.open_registered("project-a")?;
    let store = runtime.store();
    drop(runtime);
    assert!(router.close_all().is_err());
    assert_eq!(router.runtimes()?.len(), 1);
    drop(store);
    router.close_all()?;
    assert!(router.runtimes()?.is_empty());
    Ok(())
}

#[test]
fn opens_newly_registered_local_projects_without_touching_legacy_projects() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let legacy_root = temp.path().join("legacy");
    fs::create_dir(&legacy_root)?;
    fs::write(legacy_root.join("project.godot"), "config_version=5\n")?;
    let host = host(temp.path())?;
    register(&host, "legacy-project", &legacy_root, "legacy-project")?;

    let router = ProjectStorageRouter::new(host.clone());
    assert!(router.open_registered_local()?.is_empty());

    let local_root = project(temp.path(), "local", "local-project")?;
    register(&host, "local-project", &local_root, "local-project")?;

    let runtimes = router.open_registered_local()?;

    assert_eq!(
        runtimes
            .iter()
            .map(|runtime| runtime.project_id())
            .collect::<Vec<_>>(),
        vec!["local-project"]
    );
    assert!(router.runtime_for_project("local-project").is_ok());
    assert!(router.runtime_for_project("legacy-project").is_err());
    Ok(())
}

#[test]
fn close_unregistered_closes_removed_projects_and_keeps_registered_ones() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host(temp.path())?;
    let root_a = task_project(
        temp.path(),
        "a",
        "project-a",
        "task-a",
        task("task-a", "project-a"),
    )?;
    let root_b = task_project(
        temp.path(),
        "b",
        "project-b",
        "task-b",
        task("task-b", "project-b"),
    )?;
    register(&host, "project-a", &root_a, "project-a")?;
    register(&host, "project-b", &root_b, "project-b")?;
    let router = ProjectStorageRouter::new(host.clone());
    router.open_registered("project-a")?;
    router.open_registered("project-b")?;
    assert_eq!(router.close_unregistered()?, Default::default());

    host.lock().unwrap().remove("project", "project-a")?;
    let outcome = router.close_unregistered()?;
    assert_eq!(outcome.closed, vec!["project-a".to_owned()]);
    assert!(outcome.retained.is_empty());
    assert!(router.runtime_for_project("project-a").is_err());
    assert!(router.runtime_for_task("task-a").is_err());
    assert_eq!(router.runtime_for_task("task-b")?.project_id(), "project-b");
    assert_eq!(router.open_registered_local()?.len(), 1);
    Ok(())
}

#[test]
fn close_unregistered_retains_runtimes_with_active_references() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host(temp.path())?;
    let root = task_project(
        temp.path(),
        "a",
        "project-a",
        "task-a",
        task("task-a", "project-a"),
    )?;
    register(&host, "project-a", &root, "project-a")?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    host.lock().unwrap().remove("project", "project-a")?;

    let outcome = router.close_unregistered()?;
    assert!(outcome.closed.is_empty());
    assert_eq!(outcome.retained, vec!["project-a".to_owned()]);
    assert_eq!(router.runtime_for_task("task-a")?.project_id(), "project-a");

    drop(runtime);
    let outcome = router.close_unregistered()?;
    assert_eq!(outcome.closed, vec!["project-a".to_owned()]);
    assert!(router.runtime_for_task("task-a").is_err());
    assert!(router.runtimes()?.is_empty());
    Ok(())
}
