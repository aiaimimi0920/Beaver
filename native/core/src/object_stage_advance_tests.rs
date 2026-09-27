use super::{attempt_fixture::*, object_recovery_verification::request};
use crate::{
    object_attempt::{self, State},
    object_attempt_checks as checks, object_attempt_view,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{
        self as recovery,
        resume::{self, advance::Approval, Outcome, Request},
    },
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::{fs, sync::atomic::AtomicBool};

fn ready() -> Result<Fixture> {
    let f = fixture()?;
    let lease = start(&f)?;
    fs::write(workspace(&f, lease.record())?.join("output.txt"), "stage A")?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    Ok(f)
}

fn approve(f: &Fixture) -> Result<Request> {
    let attempt = attempts(f, "head")?.remove(0);
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-a".into(),
            target: object_attempt_view::Target::from_record(&attempt),
        },
    )?;
    recovery::verify(&f.runtime, &request(f, "verify-a")?, false)?;
    Ok(Request {
        project_id: "project-1".into(),
        request_id: "advance-a".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-a".into(),
        rework: None,
        advance: Some(Approval {
            attempt_id: attempt.id,
            check_request_id: "check-a".into(),
            next_fine_task_id: "fine-b".into(),
            next_fine_revision: task_record(f, "fine-b")?.revision,
            acceptance_note: "Reviewed stage A output against its acceptance criteria".into(),
        }),
    })
}

#[test]
fn advances_then_retries_successor_and_retains_history_after_cancel_and_reopen() -> Result<()> {
    let f = ready()?;
    let original = attempts(&f, "head")?.remove(0);
    let input = approve(&f)?;
    let (receipt, lease) = resume::execute(&f.runtime, &input, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    assert_eq!(lease.record().fine.id, "fine-b");
    assert_eq!(Some(&lease.record().input), original.output.as_ref());
    assert_eq!(task_record(&f, "fine-a")?.status, "accepted");
    assert_eq!(task_record(&f, "head")?.status, "running");
    assert_eq!(task_record(&f, "next")?.status, "planned");
    assert!(matches!(receipt.result, Some(Outcome::Started { .. })));
    assert_eq!(
        object_attempt_view::list(&f.runtime, &input.target.run_id)?.len(),
        2
    );
    fs::write(
        workspace(&f, lease.record())?.join("output.txt"),
        "stage B partial",
    )?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::Failed,
        None,
        &AtomicBool::new(false),
    )?;
    recovery::verify(&f.runtime, &request(&f, "verify-b")?, false)?;
    let retry = Request {
        project_id: "project-1".into(),
        request_id: "retry-b".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-b".into(),
        rework: None,
        advance: None,
    };
    let (_, retry_lease) = resume::execute(&f.runtime, &retry, &AtomicBool::new(false))?;
    let retry_lease = retry_lease.unwrap();
    assert_eq!(retry_lease.record().fine.id, "fine-b");
    object_attempt::finish(
        &f.runtime,
        retry_lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    recovery::verify(&f.runtime, &request(&f, "verify-cancel")?, false)?;
    recovery::disposition::dispose(
        &f.runtime,
        &recovery::disposition::Request {
            project_id: "project-1".into(),
            request_id: "cancel".into(),
            target: recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .target,
            verification_request_id: "verify-cancel".into(),
            choice: recovery::disposition::Choice::CancelAndKeep,
        },
        false,
    )?;
    assert_eq!(task_record(&f, "fine-a")?.status, "accepted");
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (replayed, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
    assert_eq!(replayed, receipt);
    assert!(lease.is_none());
    assert_eq!(
        object_attempt_view::list(&runtime, &input.target.run_id)?.len(),
        3
    );
    assert_eq!(
        object_attempt::list(&runtime, &input.target.run_id)?
            .iter()
            .find(|a| a.id == original.id),
        Some(&original)
    );
    Ok(())
}

#[test]
fn rejects_missing_report_wrong_successor_and_stale_revision_without_accepting() -> Result<()> {
    for reason in ["report", "successor", "revision", "note"] {
        let f = ready()?;
        let mut input = approve(&f)?;
        let approval = input.advance.as_mut().unwrap();
        match reason {
            "report" => approval.check_request_id = "missing".into(),
            "successor" => approval.next_fine_task_id = "fine-a".into(),
            "revision" => approval.next_fine_revision += 1,
            _ => approval.acceptance_note = " ".into(),
        }
        assert!(
            resume::execute(&f.runtime, &input, &AtomicBool::new(false)).is_err(),
            "{reason}"
        );
        assert_eq!(task_record(&f, "fine-a")?.status, "awaitingAcceptance");
        assert_eq!(attempts(&f, "head")?.len(), 1);
    }
    Ok(())
}

#[test]
fn fresh_integrity_and_definition_recheck_block_approval() -> Result<()> {
    for reason in ["content", "definition", "predecessor"] {
        let f = ready()?;
        let input = approve(&f)?;
        if reason == "content" {
            let attempt = attempts(&f, "head")?.remove(0);
            let hash = attempt.output.unwrap()["output.txt"].clone();
            fs::write(f.runtime.files().blob(&hash)?, "tampered")?;
        }
        let (receipt, lease) =
            resume::execute_with(&f.runtime, &input, &AtomicBool::new(false), || {
                if reason == "definition" {
                    mutate(
                        &f,
                        "object_task",
                        "fine-b",
                        "/prompt",
                        serde_json::json!("changed without revision"),
                    )?;
                }
                if reason == "predecessor" {
                    mutate(
                        &f,
                        "object_task",
                        "fine-a",
                        "/prompt",
                        serde_json::json!("changed without revision"),
                    )?;
                }
                Ok(())
            })?;
        assert!(
            matches!(receipt.result, Some(Outcome::Blocked { .. })),
            "{reason}"
        );
        assert!(lease.is_none());
        assert_eq!(task_record(&f, "fine-a")?.status, "awaitingAcceptance");
        assert_eq!(resume::replay(&f.runtime, &input)?, Some(receipt));
    }
    Ok(())
}

#[test]
fn pending_approval_reopens_and_committed_approval_never_relaunches() -> Result<()> {
    let f = ready()?;
    let input = approve(&f)?;
    assert!(resume::execute_with(
        &f.runtime,
        &input,
        &AtomicBool::new(false),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    assert_eq!(task_record(&f, "fine-a")?.status, "awaitingAcceptance");
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let pending = recovery::get(&runtime, "project-1", "head")?.unwrap();
    assert!(pending.resume.unwrap().result.is_none());
    assert!(!pending.can_dispose);
    assert_eq!(
        object_attempt::list(&runtime, &input.target.run_id)?.len(),
        1
    );
    let mut changed = input.clone();
    changed.advance.as_mut().unwrap().acceptance_note = "different approval".into();
    assert!(resume::execute(&runtime, &changed, &AtomicBool::new(false)).is_err());
    let (receipt, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
    drop(lease); // Commit survived; the process died before starting the worker.
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (replayed, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
    assert_eq!(receipt, replayed);
    assert!(lease.is_none());
    assert_eq!(
        object_attempt::list(&runtime, &input.target.run_id)?.len(),
        2
    );
    Ok(())
}
