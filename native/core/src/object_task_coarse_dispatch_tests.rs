use super::{
    attempt_fixture::{mutate, task_record},
    commit::{commit, save, task},
    queue_fixture::{enqueue, fixture},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_run_preparation,
    object_task_coarse_dispatch::{self as dispatch, SetPausedRequest},
    object_task_types::{Granularity, PlanProposal},
    object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::json;

fn request(f: &Fixture, id: &str, paused: bool) -> Result<SetPausedRequest> {
    let snapshot = object_tasks::snapshot(&f.runtime, "project-1")?;
    Ok(SetPausedRequest {
        project_id: "project-1".into(),
        task_id: "root".into(),
        request_id: id.into(),
        expected_task_revision: task_record(f, "root")?.revision,
        expected_control_revision: snapshot
            .coarse_dispatch_controls
            .first()
            .map_or(0, |c| c.revision),
        paused,
    })
}

#[test]
fn coarse_pause_blocks_future_children_across_objects_and_reserves_queue_heads() -> Result<()> {
    let f = fixture(false)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(before.tasks, after.tasks);
    assert_eq!(before.runs, after.runs);
    assert_eq!(before.plan_revision, after.plan_revision);
    // A new child on a different object inherits the policy without copying it.
    save(
        &f,
        "later",
        1,
        PlanProposal {
            objects: vec![],
            assumptions: vec![],
            tasks: vec![task(
                "later",
                Granularity::Medium,
                Some("rival"),
                Some("root"),
                None,
            )],
        },
    )?;
    commit(&f, "later-commit", "later", 1)?;
    // A successor outside the scope must still wait behind the paused object head.
    mutate(&f, "object_task", "next", "/parentTaskId", json!(null))?;
    enqueue(&f, &["head", "next", "later", "independent"])?;
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    dispatch::set_paused(&f.runtime, &request(&f, "unpause", false)?)?;
    assert_eq!(
        object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?
            .unwrap()
            .record()
            .medium
            .id,
        "later"
    );
    assert_eq!(
        object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?
            .unwrap()
            .record()
            .medium
            .id,
        "head"
    );
    Ok(())
}

#[test]
fn independent_work_proceeds_and_unpause_preserves_medium_policy_after_reopen() -> Result<()> {
    let f = fixture(false)?;
    let pause = request(&f, "pause", true)?;
    let receipt = dispatch::set_paused(&f.runtime, &pause)?;
    let medium = super::object_task_dispatch::request(&f, "medium-pause", true)?;
    crate::object_task_dispatch::set_paused(&f.runtime, &medium)?;
    enqueue(&f, &["head", "next", "independent"])?;
    assert_eq!(
        object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?
            .unwrap()
            .record()
            .medium
            .id,
        "independent"
    );
    let unpause = request(&f, "unpause", false)?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(dispatch::set_paused(&runtime, &pause)?, receipt);
    assert!(object_run_preparation::claim_next(&runtime, "project-1", "restart")?.is_none());
    dispatch::set_paused(&runtime, &unpause)?;
    assert!(object_run_preparation::claim_next(&runtime, "project-1", "restart")?.is_none());
    assert!(object_tasks::snapshot(&runtime, "project-1")?.dispatch_controls[0].paused);
    assert_eq!(dispatch::set_paused(&runtime, &pause)?, receipt);
    assert!(!object_tasks::snapshot(&runtime, "project-1")?.coarse_dispatch_controls[0].paused);
    Ok(())
}

#[test]
fn already_claimed_preparation_can_finish_while_coarse_is_paused() -> Result<()> {
    let f = fixture(false)?;
    enqueue(&f, &["head", "next"])?;
    let claim = object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    object_run_preparation::prepare(&f.runtime, claim)?;
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(before.tasks, after.tasks);
    assert!(after.coarse_dispatch_controls[0].paused);
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    Ok(())
}

#[test]
fn stale_requests_and_bad_targets_are_atomic_and_receipts_bind_payloads() -> Result<()> {
    let f = fixture(false)?;
    let input = request(&f, "pause", true)?;
    for (field, value) in [
        ("projectId", json!("other")),
        ("taskId", json!("head")),
        ("expectedTaskRevision", json!(1)),
        ("expectedControlRevision", json!(1)),
    ] {
        let mut invalid = serde_json::to_value(&input)?;
        invalid[field] = value;
        assert!(dispatch::set_paused(&f.runtime, &serde_json::from_value(invalid)?).is_err());
        assert_eq!(f.count("object_task_coarse_dispatch_control")?, 0);
        assert_eq!(f.count("object_task_coarse_dispatch_receipt")?, 0);
    }
    dispatch::set_paused(&f.runtime, &input)?;
    let mut conflicting = input.clone();
    conflicting.paused = false;
    assert_eq!(
        dispatch::set_paused(&f.runtime, &conflicting)
            .unwrap_err()
            .to_string(),
        "OBJECT_TASK_DISPATCH_REQUEST_CONFLICT"
    );
    conflicting = input;
    conflicting.request_id = "stale".into();
    assert_eq!(
        dispatch::set_paused(&f.runtime, &conflicting)
            .unwrap_err()
            .to_string(),
        "OBJECT_TASK_DISPATCH_REVISION_CONFLICT"
    );
    assert_eq!(f.count("object_task_coarse_dispatch_receipt")?, 1);
    Ok(())
}

#[test]
fn corrupt_scope_policy_or_parent_fails_closed_without_claiming() -> Result<()> {
    for (kind, id, field, value) in [
        (
            "object_task_coarse_dispatch_control",
            "root",
            "/projectId",
            json!("other"),
        ),
        (
            "object_task_coarse_dispatch_control",
            "root",
            "/revision",
            json!(0),
        ),
        ("object_task", "head", "/parentTaskId", json!("missing")),
    ] {
        let f = fixture(false)?;
        dispatch::set_paused(&f.runtime, &request(&f, "pause", true)?)?;
        enqueue(&f, &["head", "next"])?;
        mutate(&f, kind, id, field, value)?;
        assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "worker").is_err());
        assert!(object_tasks::queue(&f.runtime, "project-1")?
            .iter()
            .all(|entry| entry.state == "queued"));
    }
    Ok(())
}
