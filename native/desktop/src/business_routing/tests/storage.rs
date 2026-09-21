use super::super::{call_log_context, project_runtime_handles, query_runtime_handles};
use super::{host_store, project_root, register_project};
use beaver_core::{project_storage::ProjectStore, project_storage_router::ProjectStorageRouter};
use serde_json::{json, Value};

#[test]
fn object_framework_status_reads_open_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project.store().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":root.to_string_lossy().to_string()}),
    )?;
    drop(project);

    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":temp.path().join("host-only").to_string_lossy().to_string()}),
    )?;

    let handles = query_runtime_handles(
        &router,
        host.clone(),
        temp.path(),
        &json!({"projectId":"project-a"}),
    )
    .map_err(anyhow::Error::msg)?;
    assert!(handles.project_routed);
    let status = beaver_core::object_framework_status::status_with_routing(
        &handles.store.lock().unwrap(),
        "project-a",
        handles.project_routed,
    )?;
    assert_eq!(status["storage"]["state"], "detected");
    assert_eq!(status["storage"]["routed"], true);
    assert_eq!(status["projectId"], "project-a");
    assert_eq!(status["capabilities"]["objectsRead"], false);
    assert_eq!(status["capabilities"]["manufactureRead"], false);
    assert_eq!(status["capabilities"]["execution"], false);
    let blockers = status["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|blocker| blocker["code"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        blockers,
        ["OBJECT_QUERIES_UNAVAILABLE", "OBJECT_FRAMEWORK_DISABLED"]
    );

    drop(handles);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn object_framework_status_rejects_closed_local_project_without_host_fallback() -> anyhow::Result<()>
{
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    project.store().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":root.to_string_lossy().to_string()}),
    )?;
    drop(project);
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());

    let error = match query_runtime_handles(
        &router,
        host,
        temp.path(),
        &json!({"projectId":"project-a"}),
    ) {
        Ok(_) => anyhow::bail!("closed local projects must not fall back to the host store"),
        Err(error) => error,
    };
    assert!(
        error.contains("项目本地存储未打开，禁止回退到宿主存储"),
        "{error}"
    );
    Ok(())
}

#[test]
fn logs_query_reads_open_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let project = ProjectStore::initialize(&root, "project-a")?;
    let project_call_id = {
        let store = project.store();
        let id = beaver_core::call_log::begin(
            store,
            "test",
            "project.method",
            None,
            Some("project-a"),
            &json!({"scope":"project"}),
        )?;
        beaver_core::call_log::finish(store, &id, "succeeded", 1, &json!({"ok":true}))?;
        id
    };
    drop(project);

    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let host_call_id = {
        let store = host.lock().unwrap();
        let id = beaver_core::call_log::begin(
            &store,
            "test",
            "host.method",
            None,
            Some("project-a"),
            &json!({"scope":"host"}),
        )?;
        beaver_core::call_log::finish(&store, &id, "succeeded", 1, &json!({"ok":true}))?;
        id
    };

    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    let handles = query_runtime_handles(
        &router,
        host.clone(),
        temp.path(),
        &json!({"projectId":"project-a"}),
    )
    .map_err(anyhow::Error::msg)?;

    let project_page = beaver_core::call_log::query(
        &handles.store.lock().unwrap(),
        &json!({"projectId":"project-a"}),
    )?;
    let project_records = project_page["records"].as_array().unwrap();
    assert_eq!(project_records.len(), 1);
    assert_eq!(project_records[0]["id"], project_call_id);
    assert_eq!(project_records[0]["method"], "project.method");

    let host_page =
        beaver_core::call_log::query(&host.lock().unwrap(), &json!({"projectId":"project-a"}))?;
    let host_records = host_page["records"].as_array().unwrap();
    assert_eq!(host_records.len(), 1);
    assert_eq!(host_records[0]["id"], host_call_id);
    assert_eq!(host_records[0]["method"], "host.method");

    drop(handles);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn project_blueprint_save_reads_open_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let catalog: Value = serde_json::from_str(include_str!(
        "../../../../../dist-native/blueprint-catalog.json"
    ))?;
    let project_value = json!({
        "id":"project-a",
        "name":"Local project",
        "path":root.to_string_lossy().to_string(),
        "blueprint":catalog["default"],
        "blueprintRevision":1
    });
    let project = ProjectStore::initialize(&root, "project-a")?;
    project
        .store()
        .put("project", "project-a", &project_value)?;
    drop(project);

    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({
            "id":"project-a",
            "name":"Host shadow",
            "path":temp.path().join("host-only").to_string_lossy().to_string(),
            "blueprint":catalog["default"],
            "blueprintRevision":9
        }),
    )?;

    let handles = project_runtime_handles(&router, host.clone(), temp.path(), "project-a")
        .map_err(anyhow::Error::msg)?;
    let mut blueprint = catalog["default"].clone();
    blueprint["priorities"]["story"] = json!(5);
    let saved = beaver_core::blueprint::save(
        &handles.store.lock().unwrap(),
        "project-a",
        blueprint,
        1,
        false,
        &catalog,
    )?;
    assert_eq!(saved["name"], "Local project");
    assert_eq!(saved["blueprintRevision"], 2);
    assert_eq!(saved["blueprint"]["priorities"]["story"], 5);

    let host_project = host
        .lock()
        .unwrap()
        .get::<Value>("project", "project-a")?
        .unwrap();
    assert_eq!(host_project["name"], "Host shadow");
    assert_eq!(host_project["blueprintRevision"], 9);

    drop(handles);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn project_overview_save_reads_open_project_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let catalog: Value = serde_json::from_str(include_str!(
        "../../../../../dist-native/blueprint-catalog.json"
    ))?;
    let project_value = json!({
        "id":"project-a",
        "name":"Local project",
        "path":root.to_string_lossy().to_string(),
        "blueprint":catalog["default"],
        "blueprintRevision":1
    });
    let project = ProjectStore::initialize(&root, "project-a")?;
    project
        .store()
        .put("project", "project-a", &project_value)?;
    drop(project);

    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    host.lock().unwrap().put(
        "project",
        "project-a",
        &json!({
            "id":"project-a",
            "name":"Host shadow",
            "path":temp.path().join("host-only").to_string_lossy().to_string(),
            "blueprint":catalog["default"],
            "blueprintRevision":9
        }),
    )?;

    let mut overview = serde_json::Map::new();
    for key in beaver_core::blueprint::OVERVIEW {
        overview.insert(
            key.into(),
            if key == "name" {
                json!("Local overview")
            } else {
                catalog["default"][key].clone()
            },
        );
    }
    overview.insert("theme".into(), json!({"mode":"custom","value":"校园青春"}));
    let saved = beaver_core::blueprint::save(
        &runtime.store().lock().unwrap(),
        "project-a",
        Value::Object(overview),
        1,
        true,
        &catalog,
    )?;
    assert_eq!(saved["name"], "Local overview");
    assert_eq!(saved["blueprintRevision"], 2);
    assert_eq!(saved["blueprint"]["theme"]["value"], "校园青春");

    let host_project = host
        .lock()
        .unwrap()
        .get::<Value>("project", "project-a")?
        .unwrap();
    assert_eq!(host_project["name"], "Host shadow");
    assert_eq!(host_project["blueprintRevision"], 9);

    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn call_log_context_rejects_task_and_project_identity_conflict() -> anyhow::Result<()> {
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

    let error = match call_log_context(
        &router,
        host,
        temp.path(),
        "task.callbackState",
        &json!({"id":"task-a","projectId":"project-b"}),
    ) {
        Ok(_) => anyhow::bail!("conflicting task and project identities must fail"),
        Err(error) => error,
    };
    assert_eq!(error, "任务与项目标识不一致");
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn call_log_context_rejects_method_and_project_identity_conflict() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let router = ProjectStorageRouter::new(host.clone());
    let error = match call_log_context(
        &router,
        host,
        temp.path(),
        "project.blueprint.save",
        &json!({"id":"project-a","projectId":"project-b"}),
    ) {
        Ok(_) => anyhow::bail!("conflicting method and project identities must fail"),
        Err(error) => error,
    };
    assert_eq!(error, "项目与请求标识不一致");
    Ok(())
}
