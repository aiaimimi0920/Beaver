use super::*;

#[test]
fn state_aggregates_open_project_records_and_prefers_runtime_values() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let host_project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &host_project)?;
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"askRatio":70}))?;
    host.lock().unwrap().put(
        "task",
        "shared-task",
        &json!({"id":"shared-task","projectId":"project-a","title":"host","askRatio":null}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    let project_store = runtime.store();
    project_store
        .lock()
        .unwrap()
        .put("settings", "main", &json!({"askRatio":30}))?;
    project_store.lock().unwrap().put(
        "project",
        "project-a",
        &json!({"id":"project-a","name":"runtime","path":root,"delivery":{"channel":"runtime"}}),
    )?;
    project_store.lock().unwrap().put(
        "task",
        "shared-task",
        &json!({"id":"shared-task","projectId":"project-a","title":"runtime","askRatio":null}),
    )?;
    project_store.lock().unwrap().put(
        "task",
        "project-only",
        &json!({"id":"project-only","projectId":"project-a","title":"project-only","askRatio":null}),
    )?;

    let state = state_operation(host.clone(), &router)?;
    assert_eq!(state["projects"][0]["name"], "runtime");
    assert_eq!(state["projects"][0]["delivery"]["channel"], "runtime");
    assert_eq!(state["tasks"].as_array().unwrap().len(), 2);
    let shared = state["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["id"] == "shared-task")
        .unwrap();
    assert_eq!(shared["title"], "runtime");
    assert_eq!(shared["effectiveAskRatio"], 30);
    assert_eq!(shared["delivery"]["channel"], "runtime");
    Ok(())
}

#[test]
fn state_keeps_legacy_host_records_when_no_project_is_open() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"askRatio":70}))?;
    host.lock().unwrap().put(
        "project",
        "legacy-project",
        &json!({"id":"legacy-project","delivery":{"channel":"legacy"}}),
    )?;
    host.lock().unwrap().put(
        "task",
        "legacy-task",
        &json!({"id":"legacy-task","projectId":"legacy-project","askRatio":null}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());

    let state = state_operation(host, &router)?;
    assert_eq!(state["projects"][0]["id"], "legacy-project");
    assert_eq!(state["tasks"][0]["effectiveAskRatio"], 70);
    assert_eq!(state["tasks"][0]["delivery"]["channel"], "legacy");
    Ok(())
}

#[test]
fn state_hides_host_shadow_tasks_of_local_projects_without_opening_them() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let host_project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &host_project)?;
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"askRatio":70}))?;
    host.lock().unwrap().put(
        "task",
        "shadow-task",
        &json!({"id":"shadow-task","projectId":"project-a","title":"host","askRatio":null}),
    )?;
    host.lock().unwrap().put(
        "project",
        "legacy-project",
        &json!({"id":"legacy-project","delivery":{"channel":"legacy"}}),
    )?;
    host.lock().unwrap().put(
        "task",
        "legacy-task",
        &json!({"id":"legacy-task","projectId":"legacy-project","askRatio":null}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());

    let state = state_operation(host.clone(), &router)?;
    let project_ids = state["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|project| project["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(project_ids, ["legacy-project", "project-a"]);
    assert_eq!(state["projects"][1]["name"], "Game");
    let task_ids = state["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(task_ids, ["legacy-task"]);
    assert!(router.runtimes()?.is_empty(), "读取状态不得隐式打开项目");
    assert!(router.runtime_for_task("shadow-task").is_err());
    Ok(())
}

#[test]
fn state_does_not_fall_back_to_host_shadow_tasks_after_runtime_closes() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let host_project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &host_project)?;
    host.lock().unwrap().put(
        "task",
        "shared-task",
        &json!({"id":"shared-task","projectId":"project-a","title":"host","askRatio":null}),
    )?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    runtime.store().lock().unwrap().put(
        "task",
        "shared-task",
        &json!({"id":"shared-task","projectId":"project-a","title":"runtime","askRatio":null}),
    )?;
    let open_state = state_operation(host.clone(), &router)?;
    assert_eq!(open_state["tasks"][0]["title"], "runtime");
    drop(runtime);
    router.close("project-a")?;

    let closed_state = state_operation(host.clone(), &router)?;
    assert_eq!(closed_state["projects"][0]["id"], "project-a");
    assert!(closed_state["tasks"].as_array().unwrap().is_empty());
    assert!(router.runtimes()?.is_empty());
    Ok(())
}

#[test]
fn state_returns_project_tasks_again_after_runtime_reopens() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let host_project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &host_project)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    runtime.store().lock().unwrap().put(
        "task",
        "project-task",
        &json!({"id":"project-task","projectId":"project-a","title":"runtime","askRatio":null}),
    )?;
    drop(runtime);
    router.close("project-a")?;
    assert!(state_operation(host.clone(), &router)?["tasks"]
        .as_array()
        .unwrap()
        .is_empty());

    let _runtime = router.open_registered("project-a")?;
    let state = state_operation(host, &router)?;
    assert_eq!(state["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(state["tasks"][0]["id"], "project-task");
    assert_eq!(state["tasks"][0]["title"], "runtime");
    Ok(())
}
