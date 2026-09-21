use super::{claim_next_runtimes, interrupt_task};
use crate::{
    files::Files, project_storage::ProjectStore, scheduler::ParallelLimit,
    scheduler_runtime::TaskRuntime, store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[test]
fn draining_project_blocks_claims_and_shadows_but_preserves_controls() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("project.godot"), "config_version=5\n")?;
    let project = ProjectStore::initialize(&root, "project-a")?.into_runtime();
    let active = TaskRuntime::project("project-a", project.store(), project.files());
    let host = TaskRuntime::host(
        Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?)),
        Arc::new(Files::new(temp.path().join("host"))),
    );
    let queued = json!({
        "id":"queued", "projectId":"project-a", "status":"queued", "workspacePrepared":true
    });
    project
        .store()
        .lock()
        .unwrap()
        .put("task", "queued", &queued)?;
    host.store()
        .lock()
        .unwrap()
        .put("task", "queued", &queued)?;
    host.store().lock().unwrap().put(
        "task", "shadow-only",
        &json!({"id":"shadow-only","projectId":"project-a","status":"queued","workspacePrepared":true}),
    )?;
    host.store().lock().unwrap().put(
        "task", "legacy",
        &json!({"id":"legacy","projectId":"legacy-project","status":"queued","workspacePrepared":true}),
    )?;
    host.store().lock().unwrap().put(
        "project",
        "legacy-project",
        &json!({"id":"legacy-project"}),
    )?;
    let limit: ParallelLimit = Arc::new(|| Ok(6));
    let runtimes = vec![host.clone(), active.clone().draining()];
    let claims = claim_next_runtimes(&runtimes, 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].task["id"], "legacy");
    assert_eq!(
        project
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "queued")?,
        Some(queued.clone())
    );
    assert_eq!(
        host.store()
            .lock()
            .unwrap()
            .get::<Value>("task", "queued")?,
        Some(queued.clone())
    );
    assert_eq!(
        host.store()
            .lock()
            .unwrap()
            .get::<Value>("task", "shadow-only")?
            .unwrap()["status"],
        "queued"
    );

    let claims =
        claim_next_runtimes(&[host.clone(), active], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].task["id"], "queued");
    assert_eq!(
        host.store()
            .lock()
            .unwrap()
            .get::<Value>("task", "queued")?,
        Some(queued.clone())
    );

    project
        .store()
        .lock()
        .unwrap()
        .put("task", "queued", &queued)?;
    assert!(interrupt_task(&runtimes, "queued")
        .map_err(anyhow::Error::msg)?
        .is_some());
    assert_eq!(
        project
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "queued")?
            .unwrap()["status"],
        "interrupted"
    );
    assert_eq!(
        host.store()
            .lock()
            .unwrap()
            .get::<Value>("task", "queued")?,
        Some(queued)
    );
    Ok(())
}

#[test]
fn closed_project_host_history_is_not_claimed_or_reconciled() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Arc::new(Mutex::new(Store::open(temp.path())?));
    let runtime = TaskRuntime::host(store.clone(), Arc::new(Files::new(temp.path().into())));
    let queued =
        json!({"id":"orphan","projectId":"removed","status":"queued","workspacePrepared":true});
    let parent = json!({"id":"parent","projectId":"removed","status":"waitingChildren","subtaskIds":["child"],"plan":{"summary":"done"}});
    {
        let db = store.lock().unwrap();
        db.put("task", "orphan", &queued)?;
        db.put("task", "parent", &parent)?;
        db.put(
            "task",
            "child",
            &json!({"id":"child","projectId":"removed","status":"completed","accepted":true}),
        )?;
        db.put("project", "legacy", &json!({"id":"legacy"}))?;
        db.put(
            "task",
            "valid",
            &json!({"id":"valid","projectId":"legacy","status":"queued","workspacePrepared":true}),
        )?;
    }
    let limit: ParallelLimit = Arc::new(|| Ok(6));
    let claims = claim_next_runtimes(&[runtime], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].task["id"], "valid");
    let db = store.lock().unwrap();
    assert_eq!(db.get::<Value>("task", "orphan")?, Some(queued));
    assert_eq!(db.get::<Value>("task", "parent")?, Some(parent));
    Ok(())
}

#[test]
fn draining_project_does_not_reconcile_completed_children() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Arc::new(Mutex::new(Store::open(temp.path())?));
    let parent = json!({
        "id":"parent","projectId":"project-a","status":"waitingChildren",
        "subtaskIds":["child"],"plan":{"summary":"done"}
    });
    store.lock().unwrap().put("task", "parent", &parent)?;
    store.lock().unwrap().put(
        "task",
        "child",
        &json!({
            "id":"child","projectId":"project-a","status":"completed","accepted":true
        }),
    )?;
    let runtime = TaskRuntime::project(
        "project-a",
        store.clone(),
        Arc::new(Files::new(temp.path().into())),
    );
    let limit: ParallelLimit = Arc::new(|| Ok(1));
    assert!(
        claim_next_runtimes(&[runtime.clone().draining()], 0, &limit)
            .map_err(anyhow::Error::msg)?
            .is_empty()
    );
    assert_eq!(
        store.lock().unwrap().get::<Value>("task", "parent")?,
        Some(parent)
    );
    claim_next_runtimes(&[runtime], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(
        store
            .lock()
            .unwrap()
            .get::<Value>("task", "parent")?
            .unwrap()["status"],
        "completed"
    );
    Ok(())
}
