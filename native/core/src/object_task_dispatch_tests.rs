use super::{attempt_fixture::*, queue_fixture};
use crate::{
    object_attempt::{self, State},
    object_catalog_test_fixture::Fixture,
    object_run_preparation,
    object_task_dispatch::{self as dispatch, SetPausedRequest},
    object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

pub(super) fn request(f: &Fixture, id: &str, paused: bool) -> Result<SetPausedRequest> {
    let task = task_record(f, "head")?;
    let snapshot = object_tasks::snapshot(&f.runtime, "project-1")?;
    Ok(SetPausedRequest {
        project_id: "project-1".into(),
        task_id: task.id,
        object_id: task.object_id.unwrap(),
        run_id: task.run_id.unwrap(),
        request_id: id.into(),
        expected_task_revision: task.revision,
        expected_control_revision: snapshot
            .dispatch_controls
            .iter()
            .find(|control| control.task_id == "head")
            .map_or(0, |control| control.revision),
        paused,
    })
}

#[test]
fn paused_head_blocks_its_successor_but_independent_objects_proceed() -> Result<()> {
    let f = fixture()?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
    queue_fixture::enqueue(&f, &["head", "next", "independent"])?;
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(after.tasks, before.tasks);
    assert_eq!(after.runs, before.runs);
    assert_eq!(after.plan_revision, before.plan_revision);
    assert!(after.dispatch_controls[0].paused);
    let other = object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    assert_eq!(other.record().medium.id, "independent");
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    dispatch::set_paused(&f.runtime, &request(&f, "unpause", false)?)?;
    let head = object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    assert_eq!(head.record().medium.id, "head");
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    Ok(())
}

#[test]
fn receipt_replay_is_immutable_after_unpause_and_claim_and_rejects_payload_change() -> Result<()> {
    let f = fixture()?;
    let pause = request(&f, "pause", true)?;
    let receipt = dispatch::set_paused(&f.runtime, &pause)?;
    dispatch::set_paused(&f.runtime, &request(&f, "unpause", false)?)?;
    let lease = start(&f)?;
    assert_eq!(dispatch::set_paused(&f.runtime, &pause)?, receipt);
    let mut conflicting = pause;
    conflicting.paused = false;
    assert!(dispatch::set_paused(&f.runtime, &conflicting)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_TASK_DISPATCH_REQUEST_CONFLICT"));
    assert!(!object_tasks::snapshot(&f.runtime, "project-1")?.dispatch_controls[0].paused);
    object_attempt::validate(&f.runtime, &lease)?;
    Ok(())
}

#[test]
fn stale_revisions_and_foreign_identities_do_not_write() -> Result<()> {
    let f = fixture()?;
    let input = request(&f, "pause", true)?;
    for (field, value, expected) in [
        (
            "projectId",
            serde_json::json!("other"),
            "PROJECT_RUNTIME_MISMATCH",
        ),
        (
            "taskId",
            serde_json::json!("fine-a"),
            "OBJECT_TASK_QUEUE_REQUIRES_MEDIUM",
        ),
        (
            "objectId",
            serde_json::json!("rival"),
            "OBJECT_TASK_DISPATCH_TARGET_MISMATCH",
        ),
        (
            "runId",
            serde_json::json!("other-run"),
            "OBJECT_TASK_DISPATCH_TARGET_MISMATCH",
        ),
        (
            "expectedTaskRevision",
            serde_json::json!(1),
            "OBJECT_TASK_REVISION_CONFLICT",
        ),
        (
            "expectedControlRevision",
            serde_json::json!(1),
            "OBJECT_TASK_DISPATCH_REVISION_CONFLICT",
        ),
    ] {
        let mut altered = serde_json::to_value(&input)?;
        altered[field] = value;
        let error =
            dispatch::set_paused(&f.runtime, &serde_json::from_value(altered)?).unwrap_err();
        assert_eq!(error.to_string(), expected);
        assert_eq!(f.count("object_task_dispatch_control")?, 0);
        assert_eq!(f.count("object_task_dispatch_receipt")?, 0);
    }
    dispatch::set_paused(&f.runtime, &input)?;
    let mut stale = input;
    stale.request_id = "competing-request".into();
    assert_eq!(
        dispatch::set_paused(&f.runtime, &stale)
            .unwrap_err()
            .to_string(),
        "OBJECT_TASK_DISPATCH_REVISION_CONFLICT"
    );
    assert_eq!(f.count("object_task_dispatch_receipt")?, 1);
    Ok(())
}

#[test]
fn pause_survives_reopen_and_receipt_replay_does_not_unpause() -> Result<()> {
    let f = fixture()?;
    queue_fixture::enqueue(&f, &["head", "next"])?;
    let input = request(&f, "pause", true)?;
    let receipt = dispatch::set_paused(&f.runtime, &input)?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(dispatch::set_paused(&runtime, &input)?, receipt);
    assert!(object_run_preparation::claim_next(&runtime, "project-1", "restart")?.is_none());
    assert_eq!(
        object_tasks::snapshot(&runtime, "project-1")?.dispatch_controls,
        vec![receipt.result]
    );
    Ok(())
}

#[test]
fn pause_during_dispatch_preserves_lease_and_finished_ownership() -> Result<()> {
    for result in [State::AwaitingGate, State::Failed, State::Interrupted] {
        let f = fixture()?;
        queue_fixture::enqueue(&f, &["head", "next"])?;
        let claim = object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
        dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
        // A claim already won dispatch before pause; it can still prepare and finish.
        let mut lease = object_attempt::start(&f.runtime, claim)?.unwrap();
        object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
        dispatch::set_paused(&f.runtime, &request(&f, "unpause-running", false)?)?;
        object_attempt::validate(&f.runtime, &lease)?;
        object_attempt::finish(
            &f.runtime,
            lease,
            result.clone(),
            None,
            &AtomicBool::new(false),
        )?;
        let before = object_tasks::snapshot(&f.runtime, "project-1")?;
        dispatch::set_paused(&f.runtime, &request(&f, "pause-finished", true)?)?;
        dispatch::set_paused(&f.runtime, &request(&f, "unpause-finished", false)?)?;
        let after = object_tasks::snapshot(&f.runtime, "project-1")?;
        assert_eq!(before.tasks, after.tasks);
        assert_eq!(before.runs, after.runs);
        assert_eq!(attempts(&f, "head")?[0].state, result);
        assert_eq!(attempts(&f, "head")?.len(), 1);
        assert_eq!(task_record(&f, "fine-b")?.status, "planned");
        assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    }
    Ok(())
}

#[test]
fn unpause_after_ready_restart_cannot_recreate_execution_capability() -> Result<()> {
    let f = fixture()?;
    queue_fixture::enqueue(&f, &["head", "next"])?;
    let claim = object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    object_run_preparation::prepare(&f.runtime, claim)?;
    dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
    let input = request(&f, "unpause", false)?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    dispatch::set_paused(&runtime, &input)?;
    assert!(object_run_preparation::claim_next(&runtime, "project-1", "restart")?.is_none());
    assert!(object_attempt::list(&runtime, &input.run_id)?.is_empty());
    Ok(())
}

#[test]
fn invalid_persisted_control_fails_closed_for_snapshot_and_claim() -> Result<()> {
    for (field, value, paused) in [
        ("/objectId", serde_json::json!("rival"), true),
        ("/revision", serde_json::json!(0), false),
        (
            "/revision",
            serde_json::json!(9_007_199_254_740_992_u64),
            false,
        ),
    ] {
        let f = fixture()?;
        queue_fixture::enqueue(&f, &["head", "next"])?;
        let input = request(&f, "control", paused)?;
        dispatch::set_paused(&f.runtime, &input)?;
        mutate(
            &f,
            "object_task_dispatch_control",
            &input.run_id,
            field,
            value,
        )?;
        assert!(object_tasks::snapshot(&f.runtime, "project-1").is_err());
        assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker").is_err());
        assert!(object_tasks::queue(&f.runtime, "project-1")?
            .iter()
            .all(|entry| entry.state == "queued"));
    }
    Ok(())
}
