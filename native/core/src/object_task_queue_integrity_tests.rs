use super::queue_fixture::{enqueue, fixture};
use crate::{object_catalog_test_fixture::Fixture, object_task_queue as queue, object_tasks};
use anyhow::Result;
use serde_json::{json, Value};

fn records(fixture: &Fixture) -> Result<Vec<(String, String, String)>> {
    let handle = fixture.runtime.store();
    let store = handle.lock().unwrap();
    let mut statement = store
        .connection
        .prepare("SELECT kind,id,value FROM entities ORDER BY kind,id")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn change(fixture: &Fixture, kind: &str, id: &str, path: &str, value: Value) -> Result<()> {
    fixture.runtime.store().lock().unwrap().connection.execute(
        "UPDATE entities SET value=json_set(value,?,json(?)) WHERE kind=? AND id=?",
        rusqlite::params![path, value.to_string(), kind, id],
    )?;
    Ok(())
}

#[test]
fn stale_credentials_cannot_finish_or_cancel_any_owned_entity() -> Result<()> {
    let fixture = fixture(false)?;
    enqueue(&fixture, &["head", "next"])?;
    let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let before = records(&fixture)?;
    for (project, owner, token, generation) in [
        (
            "project-2",
            "worker",
            claim.claim_token.as_str(),
            claim.generation,
        ),
        (
            "project-1",
            "other",
            claim.claim_token.as_str(),
            claim.generation,
        ),
        ("project-1", "worker", "wrong", claim.generation),
        (
            "project-1",
            "worker",
            claim.claim_token.as_str(),
            claim.generation + 1,
        ),
    ] {
        assert!(queue::finish_claim(
            &fixture.runtime,
            project,
            "head",
            owner,
            token,
            generation,
            true
        )
        .is_err());
        assert!(
            queue::cancel_claim(&fixture.runtime, project, "head", owner, token, generation)
                .is_err()
        );
        assert_eq!(records(&fixture)?, before);
    }
    Ok(())
}

#[test]
fn mismatched_persisted_identity_cannot_redirect_a_callback() -> Result<()> {
    for (kind, path, value) in [
        ("object_task", "$.id", json!("next")),
        ("object_task", "$.projectId", json!("project-2")),
        ("object_task", "$.identity.objectId", json!("rival")),
        ("object_run", "$.id", json!("other-run")),
        ("object_run", "$.projectId", json!("project-2")),
        ("object_run", "$.mediumTaskId", json!("next")),
        ("object_run", "$.objectId", json!("rival")),
        ("object_task_queue", "$.id", json!("project-1:next")),
        ("object_task_queue", "$.taskId", json!("next")),
    ] {
        let fixture = fixture(false)?;
        enqueue(&fixture, &["head"])?;
        let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
        let id = match kind {
            "object_run" => claim.task.run_id.as_deref().unwrap(),
            "object_task_queue" => "project-1:head",
            _ => "head",
        };
        change(&fixture, kind, id, path, value)?;
        let before = records(&fixture)?;
        assert!(
            queue::finish_claim(
                &fixture.runtime,
                "project-1",
                "head",
                "worker",
                &claim.claim_token,
                claim.generation,
                true,
            )
            .is_err(),
            "{kind} {path}"
        );
        assert!(
            queue::cancel_claim(
                &fixture.runtime,
                "project-1",
                "head",
                "worker",
                &claim.claim_token,
                claim.generation,
            )
            .is_err(),
            "{kind} {path}"
        );
        assert_eq!(records(&fixture)?, before);
    }
    Ok(())
}

#[test]
fn exhausted_revisions_or_generation_leave_an_unclaimed_queue_unchanged() -> Result<()> {
    for (kind, path) in [
        ("object_task", "$.revision"),
        ("object_run", "$.revision"),
        ("object_task_queue", "$.generation"),
    ] {
        let fixture = fixture(false)?;
        enqueue(&fixture, &["head"])?;
        let task = object_tasks::get_task(&fixture.runtime, "project-1", "head")?.unwrap();
        let id = match kind {
            "object_run" => task.run_id.as_deref().unwrap(),
            "object_task_queue" => "project-1:head",
            _ => "head",
        };
        change(&fixture, kind, id, path, json!(i64::MAX))?;
        let before = records(&fixture)?;
        assert!(queue::claim_next(&fixture.runtime, "project-1", "worker").is_err());
        assert_eq!(records(&fixture)?, before);
    }
    Ok(())
}

#[test]
fn cancellation_revision_exhaustion_preserves_the_claim_and_its_subtree() -> Result<()> {
    for kind in ["object_task", "object_run", "object_task_plan_state"] {
        let fixture = fixture(false)?;
        enqueue(&fixture, &["head"])?;
        let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
        let id = match kind {
            "object_run" => claim.task.run_id.as_deref().unwrap(),
            "object_task_plan_state" => "project-1",
            _ => "fine",
        };
        change(&fixture, kind, id, "$.revision", json!(i64::MAX))?;
        let before = records(&fixture)?;
        assert!(queue::cancel_claim(
            &fixture.runtime,
            "project-1",
            "head",
            "worker",
            &claim.claim_token,
            claim.generation,
        )
        .is_err());
        assert_eq!(records(&fixture)?, before);
        if kind == "object_run" {
            assert!(queue::finish_claim(
                &fixture.runtime,
                "project-1",
                "head",
                "worker",
                &claim.claim_token,
                claim.generation,
                true,
            )
            .is_err());
            assert_eq!(records(&fixture)?, before);
        }
    }
    Ok(())
}

#[test]
fn run_write_failure_rolls_back_claim_finish_and_cancellation() -> Result<()> {
    let fixture = fixture(false)?;
    enqueue(&fixture, &["head"])?;
    let trigger = "CREATE TEMP TRIGGER reject_run_update BEFORE UPDATE ON entities
        WHEN NEW.kind='object_run' BEGIN SELECT RAISE(ABORT,'injected run write failure'); END;";
    fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch(trigger)?;
    let before = records(&fixture)?;
    assert!(queue::claim_next(&fixture.runtime, "project-1", "worker").is_err());
    assert_eq!(records(&fixture)?, before);
    fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER reject_run_update;")?;
    let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch(trigger)?;
    let before = records(&fixture)?;
    assert!(queue::finish_claim(
        &fixture.runtime,
        "project-1",
        "head",
        "worker",
        &claim.claim_token,
        claim.generation,
        true,
    )
    .is_err());
    assert_eq!(records(&fixture)?, before);
    assert!(queue::cancel_claim(
        &fixture.runtime,
        "project-1",
        "head",
        "worker",
        &claim.claim_token,
        claim.generation,
    )
    .is_err());
    assert_eq!(records(&fixture)?, before);
    Ok(())
}
