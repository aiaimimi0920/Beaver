use super::{attempt_fixture::*, queue_fixture};
use crate::{
    object_attempt,
    scheduler::ParallelLimit,
    scheduler_runtime::TaskRuntime,
    scheduler_work::{self, Batch, Work},
};
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

fn claim(runtimes: &[TaskRuntime], active: usize, limit: usize) -> Result<Batch> {
    let limit: ParallelLimit = Arc::new(move || Ok(limit));
    scheduler_work::claim(runtimes, active, &limit, true, "scheduler").map_err(anyhow::Error::msg)
}

#[test]
fn later_claim_failure_preserves_the_earlier_preparation_capability() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head", "independent"])?;
    let run_id = task_record(&fixture, "independent")?.run_id.unwrap();
    mutate(
        &fixture,
        "object_run",
        &run_id,
        "/mediumTaskId",
        json!("wrong"),
    )?;
    let mut batch = claim(&[TaskRuntime::from_project(fixture.runtime.clone())], 0, 2)?;
    assert!(batch
        .failure
        .unwrap()
        .contains("OBJECT_TASK_MEDIUM_RUN_MISMATCH"));
    assert_eq!(batch.claimed.len(), 1);
    let Work::Object(preparation) = batch.claimed.remove(0).work else {
        panic!("object work routed to legacy execution");
    };
    assert_eq!(preparation.record().medium.id, "head");
    let lease = object_attempt::start(&fixture.runtime, preparation)?.unwrap();
    assert_eq!(lease.record().fine.id, "fine-a");
    assert_eq!(task_record(&fixture, "head")?.status, "running");
    assert_eq!(task_record(&fixture, "independent")?.status, "planned");
    Ok(())
}

#[test]
fn duplicate_project_runtimes_count_one_running_object_and_claim_once() -> Result<()> {
    let fixture = fixture()?;
    let _lease = start(&fixture)?;
    queue_fixture::enqueue(&fixture, &["independent"])?;
    let runtime = TaskRuntime::from_project(fixture.runtime.clone());
    let batch = claim(&[runtime.clone(), runtime], 1, 2)?;
    assert!(batch.failure.is_none());
    assert_eq!(batch.claimed.len(), 1);
    assert_eq!(
        batch.claimed[0].work.id(),
        scheduler_work::object_key("project-1", "independent")
    );
    assert_eq!(task_record(&fixture, "next")?.status, "planned");
    assert_eq!(fixture.count("object_run_preparation")?, 2);
    Ok(())
}

#[test]
fn legacy_running_work_and_active_workers_share_the_object_slot_limit() -> Result<()> {
    let fixture = fixture()?;
    let _lease = start(&fixture)?;
    queue_fixture::enqueue(&fixture, &["independent"])?;
    fixture.runtime.store().lock().unwrap().put(
        "task",
        "legacy",
        &json!({"id":"legacy","projectId":"project-1","status":"running"}),
    )?;
    let runtimes = [TaskRuntime::from_project(fixture.runtime.clone())];
    assert!(claim(&runtimes, 0, 2)?.claimed.is_empty());
    assert!(claim(&runtimes, 3, 3)?.claimed.is_empty());
    let batch = claim(&runtimes, 2, 3)?;
    assert!(batch.failure.is_none());
    assert_eq!(batch.claimed.len(), 1);
    assert_eq!(
        batch.claimed[0].work.id(),
        scheduler_work::object_key("project-1", "independent")
    );
    Ok(())
}
