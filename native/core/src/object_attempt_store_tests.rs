use super::{attempt_fixture::*, queue_fixture};
use crate::{
    object_attempt::{self, State},
    object_run_preparation, object_task_queue,
};
use anyhow::Result;
use serde_json::json;
use std::sync::atomic::AtomicBool;

#[test]
fn frozen_first_fine_checkpoint_retains_object_ownership() -> Result<()> {
    let fixture = fixture()?;
    let mut lease = start(&fixture)?;
    let frozen = lease.record().clone();
    assert_eq!(frozen.fine.id, "fine-a");
    assert!(frozen.input.is_empty());
    assert_eq!(
        task_record(&fixture, "fine-a")?.revision,
        frozen.fine.revision + 1
    );
    assert_eq!(task_record(&fixture, "head")?.status, "running");
    object_attempt::bind(&fixture.runtime, &mut lease, "thread", None)?;
    assert!(object_attempt::bind(&fixture.runtime, &mut lease, "other", Some("turn")).is_err());
    object_attempt::bind(&fixture.runtime, &mut lease, "thread", Some("turn"))?;
    std::fs::write(
        workspace(&fixture, &frozen)?.join("result.txt"),
        "fine output",
    )?;
    object_attempt::finish(
        &fixture.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let record = attempts(&fixture, "head")?.remove(0);
    assert_eq!(record.state, State::AwaitingGate);
    assert!(record.output.unwrap().contains_key("result.txt"));
    assert_eq!(record.fine, frozen.fine);
    assert_eq!(task_record(&fixture, "fine-b")?.status, "planned");
    assert_eq!(task_record(&fixture, "head")?.status, "awaitingAcceptance");
    let queue = object_task_queue::list(&fixture.runtime, "project-1")?;
    assert_eq!(
        queue[0].claim_token.as_deref(),
        Some(frozen.preparation.claim_token.as_str())
    );
    assert_eq!(queue[0].owner.as_deref(), Some("worker"));
    assert!(object_run_preparation::claim_next(&fixture.runtime, "project-1", "later")?.is_none());
    assert_eq!(fixture.count("task")?, 0);
    assert_eq!(fixture.count("object_version")?, 0);
    Ok(())
}

#[test]
fn stale_callbacks_cannot_freeze_output_or_release_claim() -> Result<()> {
    for (kind, id, pointer, value) in [
        (
            "object_task_queue",
            "project-1:head",
            "/generation",
            json!(99),
        ),
        (
            "object_task_queue",
            "project-1:head",
            "/owner",
            json!("other"),
        ),
        (
            "object_task_queue",
            "project-1:head",
            "/claimToken",
            json!("other"),
        ),
        ("object_task", "fine-a", "/revision", json!(99)),
        ("object_task", "fine-a", "/prompt", json!("changed")),
        ("object_task", "fine-a", "/objectId", json!("rival")),
        ("object_task", "head", "/revision", json!(99)),
        ("object_run", "run", "/revision", json!(99)),
        ("object_attempt", "attempt", "/turnId", json!("forged")),
    ] {
        let fixture = fixture()?;
        let lease = start(&fixture)?;
        let id = match id {
            "run" => lease.record().preparation.run.id.clone(),
            "attempt" => lease.record().id.clone(),
            id => id.into(),
        };
        mutate(&fixture, kind, &id, pointer, value)?;
        assert!(
            object_attempt::finish(
                &fixture.runtime,
                lease,
                State::AwaitingGate,
                None,
                &AtomicBool::new(false)
            )
            .is_err(),
            "{kind} {pointer}"
        );
        let record = attempts(&fixture, "head")?.remove(0);
        assert_eq!(record.state, State::Running);
        assert!(record.output.is_none());
        assert_eq!(task_record(&fixture, "head")?.status, "running");
    }
    Ok(())
}

#[test]
fn final_checkpoint_observes_cancellation_and_keeps_workspace() -> Result<()> {
    let fixture = fixture()?;
    let lease = start(&fixture)?;
    let path = workspace(&fixture, lease.record())?;
    std::fs::write(path.join("partial.txt"), "keep me")?;
    object_attempt::finish(
        &fixture.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(true),
    )?;
    let record = attempts(&fixture, "head")?.remove(0);
    assert_eq!(record.state, State::Interrupted);
    assert!(record.output.unwrap().contains_key("partial.txt"));
    assert!(path.join("partial.txt").exists());
    assert!(
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "restart")?.is_none()
    );
    Ok(())
}

#[test]
fn unmet_first_fine_dependency_retains_preparation_without_execution() -> Result<()> {
    let fixture = fixture()?;
    mutate(
        &fixture,
        "object_task",
        "fine-a",
        "/dependsOn",
        json!(["independent"]),
    )?;
    queue_fixture::enqueue(&fixture, &["head"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    assert!(object_attempt::start(&fixture.runtime, claim)?.is_none());
    assert!(attempts(&fixture, "head")?.is_empty());
    assert!(
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "restart")?.is_none()
    );
    Ok(())
}

#[test]
fn persisted_attempt_key_must_match_record_identity() -> Result<()> {
    let fixture = fixture()?;
    let lease = start(&fixture)?;
    mutate(
        &fixture,
        "object_attempt",
        &lease.record().id,
        "/id",
        json!("forged"),
    )?;
    assert!(attempts(&fixture, "head")
        .unwrap_err()
        .to_string()
        .contains("IDENTITY_MISMATCH"));
    Ok(())
}
