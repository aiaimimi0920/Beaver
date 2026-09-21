use super::super::call_log_context;
use super::{host_store, project_root, register_project};
use beaver_core::{project_storage::ProjectStore, project_storage_router::ProjectStorageRouter};
use serde_json::{json, Value};
use std::path::Path;

fn initialize_delivery_project(root: &Path, id: &str, delivery: &Path) -> anyhow::Result<()> {
    let project = ProjectStore::initialize(root, id)?;
    project.store().put(
        "project",
        id,
        &json!({
            "id": id,
            "name": id,
            "path": root.to_string_lossy().to_string(),
            "delivery": {"path": delivery.to_string_lossy().to_string()}
        }),
    )?;
    drop(project);
    Ok(())
}

fn verify_input(delivery: &Path) -> Value {
    json!({"path": delivery.to_string_lossy().to_string()})
}

#[test]
fn verify_export_call_log_binds_to_project_store_by_path() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let delivery = temp.path().join("exports/project-a");
    initialize_delivery_project(&root, "project-a", &delivery)?;
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let context = call_log_context(
        &router,
        host.clone(),
        temp.path(),
        "game.verifyExport",
        &verify_input(&delivery),
    )
    .map_err(anyhow::Error::msg)?;
    assert_eq!(context.project_id.as_deref(), Some("project-a"));
    assert!(context.task_id.is_none());
    let call_id = {
        let store = context.handles.store.lock().unwrap();
        beaver_core::call_log::begin(
            &store,
            "test",
            "game.verifyExport",
            None,
            context.project_id.as_deref(),
            &verify_input(&delivery),
        )?
    };

    let project_page = beaver_core::call_log::query(
        &runtime.store().lock().unwrap(),
        &json!({"projectId":"project-a"}),
    )?;
    let project_records = project_page["records"].as_array().unwrap();
    assert_eq!(project_records.len(), 1);
    assert_eq!(project_records[0]["id"], call_id);
    let host_page = beaver_core::call_log::query(&host.lock().unwrap(), &json!({}))?;
    assert!(host_page["records"].as_array().unwrap().is_empty());

    drop(context);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn verify_export_opens_registered_local_project_instead_of_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let delivery = temp.path().join("exports/project-a");
    initialize_delivery_project(&root, "project-a", &delivery)?;
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());

    let context = call_log_context(
        &router,
        host.clone(),
        temp.path(),
        "game.verifyExport",
        &verify_input(&delivery),
    )
    .map_err(anyhow::Error::msg)?;
    assert_eq!(context.project_id.as_deref(), Some("project-a"));
    let runtime = router.runtime_for_project("project-a")?;
    assert!(std::sync::Arc::ptr_eq(
        &context.handles.store,
        &runtime.store()
    ));
    assert!(!std::sync::Arc::ptr_eq(&context.handles.store, &host));

    drop(context);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}

#[test]
fn verify_export_rejects_explicit_project_that_does_not_own_the_path() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root_a = project_root(temp.path(), "project-a")?;
    let root_b = project_root(temp.path(), "project-b")?;
    let delivery_a = temp.path().join("exports/project-a");
    let delivery_b = temp.path().join("exports/project-b");
    initialize_delivery_project(&root_a, "project-a", &delivery_a)?;
    initialize_delivery_project(&root_b, "project-b", &delivery_b)?;
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root_a)?;
    register_project(&host, "project-b", &root_b)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime_a = router.open_registered("project-a")?;
    let runtime_b = router.open_registered("project-b")?;

    let mut input = verify_input(&delivery_a);
    input["projectId"] = json!("project-b");
    let error = match call_log_context(&router, host, temp.path(), "game.verifyExport", &input) {
        Ok(_) => anyhow::bail!("a foreign export path must not bind to the explicit project"),
        Err(error) => error,
    };
    assert!(error.contains("项目与导出路径归属不一致"), "{error}");

    drop(runtime_a);
    drop(runtime_b);
    router.close("project-a")?;
    router.close("project-b")?;
    Ok(())
}

#[test]
fn verify_export_without_matching_delivery_keeps_host_context() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let delivery = temp.path().join("exports/project-a");
    initialize_delivery_project(&root, "project-a", &delivery)?;
    let host = host_store(temp.path())?;
    register_project(&host, "project-a", &root)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let context = call_log_context(
        &router,
        host.clone(),
        temp.path(),
        "game.verifyExport",
        &verify_input(&temp.path().join("exports/unknown")),
    )
    .map_err(anyhow::Error::msg)?;
    assert!(context.project_id.is_none());
    assert!(std::sync::Arc::ptr_eq(&context.handles.store, &host));

    drop(context);
    drop(runtime);
    router.close("project-a")?;
    Ok(())
}
