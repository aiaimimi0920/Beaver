use super::*;

fn registered_host(root: &Path) -> Result<TaskRuntime> {
    let host = host_runtime(root)?;
    host.store()
        .lock()
        .unwrap()
        .put("project", "legacy", &json!({"id":"legacy"}))?;
    Ok(host)
}

#[test]
fn runtime_claim_prefers_project_duplicate_over_host_task() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = project_runtime(temp.path(), "project-a")?;
    project.store().lock().unwrap().put(
        "task",
        "shared",
        &queued_project_task("shared", "project-a"),
    )?;
    let host = registered_host(&temp.path().join("host"))?;
    let host_store = host.store();
    for id in ["shared", "host-only"] {
        host_store
            .lock()
            .unwrap()
            .put("task", id, &queued_project_task(id, "legacy"))?;
    }
    let limit: ParallelLimit = Arc::new(|| Ok(2));
    let claimed = claim_next_runtimes(&[project, host], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claimed.len(), 2);
    assert_eq!(claimed[0].task["id"], "shared");
    assert!(matches!(claimed[0].runtime.owner(), RuntimeOwner::Project(id) if id == "project-a"));
    assert_eq!(claimed[1].task["id"], "host-only");
    assert_eq!(
        host_store
            .lock()
            .unwrap()
            .get::<Value>("task", "shared")?
            .unwrap()["status"],
        "queued"
    );
    Ok(())
}

#[test]
fn runtime_claim_keeps_legacy_host_task_available() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = registered_host(&temp.path().join("host"))?;
    host.store()
        .lock()
        .unwrap()
        .put("task", "legacy", &queued_project_task("legacy", "legacy"))?;
    let limit: ParallelLimit = Arc::new(|| Ok(1));
    let claimed = claim_next_runtimes(&[host], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].task["id"], "legacy");
    assert_eq!(claimed[0].runtime.owner(), &RuntimeOwner::Host);
    Ok(())
}

#[test]
fn runtime_claim_applies_global_capacity_across_project_and_host() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = project_runtime(temp.path(), "project-a")?;
    project.store().lock().unwrap().put(
        "task",
        "project-running",
        &json!({"id":"project-running","projectId":"project-a","status":"running"}),
    )?;
    let host = registered_host(&temp.path().join("host"))?;
    host.store().lock().unwrap().put(
        "task",
        "host-queued",
        &queued_project_task("host-queued", "legacy"),
    )?;
    let one_slot: ParallelLimit = Arc::new(|| Ok(1));
    assert!(
        claim_next_runtimes(&[project.clone(), host.clone()], 0, &one_slot)
            .map_err(anyhow::Error::msg)?
            .is_empty()
    );
    let two_slots: ParallelLimit = Arc::new(|| Ok(2));
    let claimed =
        claim_next_runtimes(&[project, host], 0, &two_slots).map_err(anyhow::Error::msg)?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].task["id"], "host-queued");
    Ok(())
}
