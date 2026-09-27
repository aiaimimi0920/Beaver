use super::call_log_context;
use crate::object_task_test_fixture::Fixture;
use anyhow::Result;
use beaver_core::call_log;
use serde_json::json;
use std::sync::Arc;

#[test]
fn object_task_logs_use_explicit_project_even_when_legacy_task_id_collides() -> Result<()> {
    let f = Fixture::new()?;
    f.host.lock().unwrap().put(
        "task",
        "collision",
        &json!({"id":"collision","projectId":"legacy"}),
    )?;
    let runtime = f.router.runtime_for_project("p")?;
    for method in [
        "objectTask.get",
        "objectTask.getDraft",
        "objectTask.getRun",
        "objectTask.snapshot",
        "objectTask.saveDraft",
        "objectTask.commit",
        "objectTask.setPaused",
        "objectTask.setCoarsePaused",
    ] {
        let input = json!({"projectId":"p","taskId":"collision"});
        let context = call_log_context(&f.router, f.host.clone(), f.temp.path(), method, &input)
            .map_err(anyhow::Error::msg)?;
        assert!(context.handles.project_routed);
        assert!(Arc::ptr_eq(&context.handles.store, &runtime.store()));
        assert_eq!(context.project_id.as_deref(), Some("p"));
        let store = context.handles.store.lock().unwrap();
        let id = call_log::begin(
            &store,
            "test",
            method,
            context.task_id.as_deref(),
            context.project_id.as_deref(),
            &input,
        )?;
        call_log::finish(&store, &id, "succeeded", 1, &json!(null))?;
    }
    let local = call_log::query(&runtime.store().lock().unwrap(), &json!({}))?;
    assert_eq!(local["records"].as_array().unwrap().len(), 8);
    let host = call_log::query(&f.host.lock().unwrap(), &json!({}))?;
    assert!(host["records"].as_array().unwrap().is_empty());
    let other = f.router.runtime_for_project("other")?;
    assert!(
        call_log::query(&other.store().lock().unwrap(), &json!({}))?["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    Ok(())
}

#[test]
fn object_task_logs_never_use_host_fallback_or_implicitly_open_a_project() -> Result<()> {
    let f = Fixture::new()?;
    for project in ["legacy", "closed", "missing", ""] {
        let result = call_log_context(
            &f.router,
            f.host.clone(),
            f.temp.path(),
            "objectTask.snapshot",
            &json!({"projectId":project}),
        );
        assert!(result.is_err(), "{project}");
    }
    assert!(f.router.runtime_for_project("closed").is_err());
    Ok(())
}
