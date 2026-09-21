use super::{claim_next_runtimes, interrupt_task};
use crate::{
    files::Files, project_work_gate::ProjectWorkGate, scheduler::ParallelLimit,
    scheduler_runtime::TaskRuntime, store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[test]
fn pre_unregister_snapshot_cannot_coordinate_or_claim_but_keeps_active_controls() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    let local = Arc::new(Mutex::new(Store::open(&temp.path().join("local"))?));
    let project = json!({"id":"p"});
    host.lock().unwrap().put("project", "p", &project)?;
    let queued = json!({"id":"q","projectId":"p","status":"queued","workspacePrepared":true});
    let parent = json!({
        "id":"parent","projectId":"p","status":"waitingChildren",
        "subtaskIds":["child"],"plan":{"summary":"done"}
    });
    {
        let db = local.lock().unwrap();
        db.put("task", "q", &queued)?;
        db.put("task", "parent", &parent)?;
        db.put(
            "task",
            "child",
            &json!({
                "id":"child","projectId":"p","status":"completed","accepted":true
            }),
        )?;
        db.put(
            "task",
            "active",
            &json!({
                "id":"active","projectId":"p","status":"running"
            }),
        )?;
    }
    let snapshot = TaskRuntime::project(
        "p",
        local.clone(),
        Arc::new(Files::new(temp.path().join("local"))),
    )
    .with_work_gate(ProjectWorkGate::registered(host.clone(), "p"));
    // The enumerator has already returned a non-draining runtime before removal.
    let limit: ParallelLimit = Arc::new({
        let host = host.clone();
        move || {
            host.lock()
                .unwrap()
                .remove("project", "p")
                .map_err(|e| e.to_string())?;
            Ok(6)
        }
    });
    assert!(!snapshot.is_draining());
    assert!(claim_next_runtimes(&[snapshot.clone()], 0, &limit)
        .map_err(anyhow::Error::msg)?
        .is_empty());
    assert_eq!(
        local.lock().unwrap().get::<Value>("task", "q")?,
        Some(queued)
    );
    assert_eq!(
        local.lock().unwrap().get::<Value>("task", "parent")?,
        Some(parent)
    );
    let controlled = interrupt_task(&[snapshot.clone()], "active")
        .map_err(anyhow::Error::msg)?
        .unwrap();
    assert!(Arc::ptr_eq(&controlled.store(), &local));
    // Running cancellation is signalled by Scheduler; this helper only interrupts queued work.
    assert_eq!(
        local
            .lock()
            .unwrap()
            .get::<Value>("task", "active")?
            .unwrap()["status"],
        "running"
    );

    host.lock().unwrap().put("project", "p", &project)?;
    let limit: ParallelLimit = Arc::new(|| Ok(6));
    let claims = claim_next_runtimes(&[snapshot], 0, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].task["id"], "q");
    assert_eq!(
        local
            .lock()
            .unwrap()
            .get::<Value>("task", "parent")?
            .unwrap()["status"],
        "completed"
    );
    host.lock().unwrap().remove("project", "p")?;
    // Work already admitted keeps the exact original Store for its final writes.
    assert!(Arc::ptr_eq(&claims[0].runtime.store(), &local));
    let mut completed = claims[0].task.clone();
    completed["status"] = json!("completed");
    claims[0]
        .runtime
        .store()
        .lock()
        .unwrap()
        .put("task", "q", &completed)?;
    assert_eq!(
        local.lock().unwrap().get::<Value>("task", "q")?,
        Some(completed)
    );
    Ok(())
}
