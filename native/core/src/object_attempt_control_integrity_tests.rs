use super::attempt_fixture::*;
use crate::{
    object_attempt::{self, State},
    object_attempt_control::{self as control, InterruptRequest},
};
use anyhow::Result;
use serde_json::json;
use std::sync::atomic::AtomicBool;

#[test]
fn stale_public_identity_or_revision_cannot_request_interruption() -> Result<()> {
    let fixture = fixture()?;
    let mut lease = start(&fixture)?;
    object_attempt::bind(&fixture.runtime, &mut lease, "thread", Some("turn"))?;
    let request = interrupt_request(&fixture, "head")?;
    let before = attempts(&fixture, "head")?;
    for (path, value) in [
        ("/projectId", json!("other-project")),
        ("/target/taskId", json!("next")),
        ("/target/fineTaskId", json!("fine-b")),
        ("/target/objectId", json!("rival")),
        ("/target/runId", json!("other-run")),
        ("/target/attemptId", json!("other-attempt")),
        ("/target/owner", json!("other-owner")),
        ("/target/claimToken", json!("other-token")),
        ("/target/generation", json!(99)),
        ("/target/threadId", json!("other-thread")),
        ("/target/turnId", json!("other-turn")),
        ("/target/threadId", json!(null)),
        ("/target/turnId", json!(null)),
        ("/expectedTaskRevision", json!(99)),
    ] {
        let mut value_request = serde_json::to_value(&request)?;
        *value_request.pointer_mut(path).unwrap() = value;
        let stale: InterruptRequest = serde_json::from_value(value_request)?;
        assert!(
            control::request(&fixture.runtime, &stale, true).is_err(),
            "{path}"
        );
        assert_eq!(fixture.count(control::KIND)?, 0, "{path}");
        assert_eq!(attempts(&fixture, "head")?, before);
    }
    assert!(control::request(&fixture.runtime, &request, true)?);
    let mut reused = request.clone();
    reused.expected_task_revision += 1;
    assert!(control::request(&fixture.runtime, &reused, true)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    Ok(())
}

#[test]
fn corrupted_frozen_receipt_is_not_returned_as_a_successful_interrupt() -> Result<()> {
    for (path, value) in [
        ("/result/projectId", json!("other-project")),
        ("/result/target/attemptId", json!("other-attempt")),
        ("/result/target/turnId", json!("forged")),
        ("/result/state", json!("running")),
        ("/result/outputCaptured", json!(false)),
    ] {
        let fixture = fixture()?;
        let lease = start(&fixture)?;
        let request = interrupt_request(&fixture, "head")?;
        control::request(&fixture.runtime, &request, true)?;
        object_attempt::finish(
            &fixture.runtime,
            lease,
            State::AwaitingGate,
            None,
            &AtomicBool::new(false),
        )?;
        mutate(&fixture, control::KIND, &request.request_id, path, value)?;
        assert!(
            control::request(&fixture.runtime, &request, false)
                .unwrap_err()
                .to_string()
                .contains("RECEIPT_MISMATCH"),
            "{path}"
        );
        assert!(control::result(&fixture.runtime, &request).is_err());
    }
    Ok(())
}

#[test]
fn pending_receipt_with_a_wrong_database_key_cannot_freeze_a_checkpoint() -> Result<()> {
    let fixture = fixture()?;
    let lease = start(&fixture)?;
    let request = interrupt_request(&fixture, "head")?;
    control::request(&fixture.runtime, &request, true)?;
    fixture.runtime.store().lock().unwrap().connection.execute(
        "UPDATE entities SET id='wrong-key' WHERE kind=? AND id=?",
        [control::KIND, &request.request_id],
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
    .contains("RECEIPT_MISMATCH"));
    assert_eq!(attempts(&fixture, "head")?[0].state, State::Running);
    assert_eq!(fixture.count(control::KIND)?, 1);
    Ok(())
}

#[test]
fn project_and_medium_ids_with_colons_keep_distinct_scheduler_ownership() {
    use crate::scheduler_work::object_key;
    assert_ne!(object_key("a:b", "c"), object_key("a", "b:c"));
    assert_ne!(
        object_key("project-1", "head"),
        object_key("project-2", "head")
    );
}
