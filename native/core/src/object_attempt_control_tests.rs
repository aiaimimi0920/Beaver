use super::attempt_fixture::*;
use crate::{
    object_attempt::{self, State},
    object_attempt_control as control, object_task_queue,
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

#[test]
fn durable_interrupt_wins_checkpoint_race_and_replays_after_completion() -> Result<()> {
    let fixture = fixture()?;
    let mut lease = start(&fixture)?;
    object_attempt::bind(&fixture.runtime, &mut lease, "thread", Some("turn"))?;
    let request = interrupt_request(&fixture, "head")?;
    let queue_before = object_task_queue::list(&fixture.runtime, "project-1")?;
    assert!(control::request(&fixture.runtime, &request, true)?);
    assert!(control::request(&fixture.runtime, &request, true)?);
    assert_eq!(fixture.count(control::KIND)?, 1);
    assert!(control::result(&fixture.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("INTERRUPT_PENDING"));
    std::fs::write(
        workspace(&fixture, lease.record())?.join("partial.txt"),
        "keep",
    )?;
    object_attempt::finish(
        &fixture.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let receipt = control::result(&fixture.runtime, &request)?;
    assert_eq!(receipt.result.state, State::Interrupted);
    assert!(receipt.result.output_captured);
    assert_eq!(
        receipt.result.task_revision,
        request.expected_task_revision + 1
    );
    assert_eq!(receipt.result.target, request.target);
    assert!(!control::request(&fixture.runtime, &request, false)?);
    assert_eq!(control::result(&fixture.runtime, &request)?, receipt);
    let mut conflicting = request;
    conflicting.target.turn_id = Some("different-turn".into());
    assert!(control::request(&fixture.runtime, &conflicting, false)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    let queue_after = object_task_queue::list(&fixture.runtime, "project-1")?;
    assert_eq!(queue_after[0].owner, queue_before[0].owner);
    assert_eq!(queue_after[0].claim_token, queue_before[0].claim_token);
    assert_eq!(task_record(&fixture, "next")?.status, "planned");
    Ok(())
}

#[test]
fn interruption_before_binding_keeps_the_requested_identity_frozen() -> Result<()> {
    for thread_bound in [false, true] {
        let fixture = fixture()?;
        let mut lease = start(&fixture)?;
        if thread_bound {
            object_attempt::bind(&fixture.runtime, &mut lease, "thread", None)?;
        }
        let request = interrupt_request(&fixture, "head")?;
        assert!(control::request(&fixture.runtime, &request, true)?);
        assert!(
            object_attempt::bind(&fixture.runtime, &mut lease, "thread", Some("turn"))
                .unwrap_err()
                .to_string()
                .contains("OBJECT_ATTEMPT_INTERRUPTED")
        );
        object_attempt::finish(
            &fixture.runtime,
            lease,
            State::Failed,
            Some("bind failed".into()),
            &AtomicBool::new(false),
        )?;
        let receipt = control::result(&fixture.runtime, &request)?;
        assert_eq!(receipt.result.state, State::Interrupted);
        assert_eq!(receipt.result.target, request.target);
    }
    Ok(())
}

#[test]
fn late_request_reports_the_actual_frozen_outcome_without_rewriting_it() -> Result<()> {
    for state in [State::AwaitingGate, State::Failed, State::Interrupted] {
        let fixture = fixture()?;
        let lease = start(&fixture)?;
        let request = interrupt_request(&fixture, "head")?;
        object_attempt::finish(
            &fixture.runtime,
            lease,
            state.clone(),
            None,
            &AtomicBool::new(false),
        )?;
        let before = attempts(&fixture, "head")?;
        assert!(!control::request(&fixture.runtime, &request, false)?);
        assert_eq!(
            control::result(&fixture.runtime, &request)?.result.state,
            state
        );
        assert_eq!(attempts(&fixture, "head")?, before);
    }
    Ok(())
}

#[test]
fn nonlive_running_attempt_requires_recovery_and_keeps_ownership() -> Result<()> {
    let fixture = fixture()?;
    let _lease = start(&fixture)?;
    let request = interrupt_request(&fixture, "head")?;
    let before = attempts(&fixture, "head")?;
    let queue = object_task_queue::list(&fixture.runtime, "project-1")?;
    assert!(control::request(&fixture.runtime, &request, false)
        .unwrap_err()
        .to_string()
        .contains("RECOVERY_REQUIRED"));
    assert_eq!(fixture.count(control::KIND)?, 0);
    assert_eq!(attempts(&fixture, "head")?, before);
    assert_eq!(
        object_task_queue::list(&fixture.runtime, "project-1")?,
        queue
    );
    Ok(())
}

#[test]
fn checkpoint_and_interrupt_receipt_roll_back_together() -> Result<()> {
    let fixture = fixture()?;
    let lease = start(&fixture)?;
    let request = interrupt_request(&fixture, "head")?;
    control::request(&fixture.runtime, &request, true)?;
    let before = attempts(&fixture, "head")?;
    let queue = object_task_queue::list(&fixture.runtime, "project-1")?;
    fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_receipt BEFORE UPDATE ON entities
         WHEN NEW.kind='object_attempt_interrupt_receipt'
         BEGIN SELECT RAISE(ABORT,'receipt unavailable'); END;",
        )?;
    assert!(object_attempt::finish(
        &fixture.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )
    .unwrap_err()
    .to_string()
    .contains("receipt unavailable"));
    assert_eq!(attempts(&fixture, "head")?, before);
    assert_eq!(
        object_task_queue::list(&fixture.runtime, "project-1")?,
        queue
    );
    assert_eq!(task_record(&fixture, "fine-a")?.status, "running");
    assert!(control::result(&fixture.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("INTERRUPT_PENDING"));
    Ok(())
}
