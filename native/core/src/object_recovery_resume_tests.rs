use super::{attempt_fixture::*, object_recovery_verification::request};
use crate::{
    object_attempt::{self, State},
    object_attempt_view,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{
        self as recovery,
        resume::{self, Outcome, Request},
    },
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::{fs, sync::atomic::AtomicBool};

fn stopped(state: State) -> Result<Fixture> {
    let f = fixture()?;
    let lease = start(&f)?;
    fs::write(
        workspace(&f, lease.record())?.join("partial.txt"),
        "first output",
    )?;
    object_attempt::finish(&f.runtime, lease, state, None, &AtomicBool::new(false))?;
    Ok(f)
}

fn authorize(f: &Fixture, id: &str) -> Result<Request> {
    recovery::verify(&f.runtime, &request(f, id)?, false)?;
    let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    Ok(Request {
        project_id: "project-1".into(),
        request_id: format!("resume-{id}"),
        target: view.target,
        verification_request_id: id.into(),
        rework: None,
        advance: None,
    })
}

#[test]
fn retries_chain_checkpoints_and_keep_immutable_history_after_disposition() -> Result<()> {
    let f = stopped(State::Interrupted)?;
    let original = attempts(&f, "head")?.remove(0);
    let old_view = object_attempt_view::list(&f.runtime, &original.preparation.run.id)?.remove(0);
    for index in 0..2 {
        let input = authorize(&f, &format!("verify-{index}"))?;
        assert!(
            recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .can_resume
        );
        let (receipt, lease) = resume::execute(&f.runtime, &input, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert!(matches!(receipt.result, Some(Outcome::Started { .. })));
        assert_eq!(lease.record().fine.id, original.fine.id);
        assert_eq!(
            lease.record().input,
            f.runtime.files().capture(&workspace(&f, lease.record())?)?
        );
        assert!(
            !recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .can_resume
        );
        fs::write(
            workspace(&f, lease.record())?.join("partial.txt"),
            format!("output-{index}"),
        )?;
        object_attempt::finish(
            &f.runtime,
            lease,
            State::Failed,
            None,
            &AtomicBool::new(false),
        )?;
        let (replayed, lease) = resume::execute(&f.runtime, &input, &AtomicBool::new(false))?;
        assert_eq!(replayed, receipt);
        assert!(lease.is_none());
        assert_eq!(attempts(&f, "head")?.len(), index + 2);
        let views = object_attempt_view::list(&f.runtime, &original.preparation.run.id)?;
        assert!(views.contains(&old_view));
    }
    let input = authorize(&f, "dispose-verify")?;
    let receipt = recovery::disposition::dispose(
        &f.runtime,
        &recovery::disposition::Request {
            project_id: input.project_id,
            request_id: "dispose".into(),
            target: input.target,
            verification_request_id: input.verification_request_id,
            choice: recovery::disposition::Choice::CancelAndKeep,
        },
        false,
    )?;
    assert!(receipt.result.is_some());
    let views = object_attempt_view::list(&f.runtime, &original.preparation.run.id)?;
    assert_eq!(views.len(), 3);
    assert!(views.contains(&old_view));
    let definitions =
        object_attempt_view::list_with_details(&f.runtime, &original.preparation.run.id)?;
    assert_eq!(definitions.len(), 3);
    for (view, definition, checkpoints) in definitions {
        let record = attempts(&f, "head")?
            .into_iter()
            .find(|record| record.id == view.target.attempt_id)
            .unwrap();
        assert_eq!(definition.prompt, record.fine.prompt);
        assert_eq!(definition.acceptance, record.fine.acceptance);
        assert_eq!(definition.revision, record.fine.revision);
        assert_eq!(checkpoints.input, record.input);
        assert_eq!(checkpoints.output, record.output);
    }
    Ok(())
}

#[test]
fn pending_retry_reopens_and_only_exact_request_can_finish_it() -> Result<()> {
    let f = stopped(State::Failed)?;
    let input = authorize(&f, "verify")?;
    assert!(resume::execute_with(
        &f.runtime,
        &input,
        &AtomicBool::new(false),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    let pending = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(pending.resume.unwrap().result.is_none());
    assert!(!pending.can_resume && !pending.can_dispose);
    assert_eq!(
        recovery::verify(&f.runtime, &request(&f, "new-verify")?, false)
            .unwrap_err()
            .to_string(),
        "OBJECT_RECOVERY_RESUME_PENDING"
    );
    let mut changed = input.clone();
    changed.target.control_revision += 1;
    assert_eq!(
        resume::execute(&f.runtime, &changed, &AtomicBool::new(false))
            .err()
            .unwrap()
            .to_string(),
        "OBJECT_RECOVERY_RESUME_REQUEST_CONFLICT"
    );
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (receipt, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
    assert!(lease.is_some());
    drop(lease); // Simulate a crash after commit but before the worker starts.
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (replayed, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
    assert_eq!(replayed, receipt);
    assert!(lease.is_none());
    assert_eq!(
        object_attempt::list(&runtime, &input.target.run_id)?.len(),
        2
    );
    assert!(
        !recovery::get(&runtime, "project-1", "head")?
            .unwrap()
            .can_resume
    );
    Ok(())
}

#[test]
fn file_drift_shutdown_and_record_change_block_attempt_creation() -> Result<()> {
    for reason in ["drift", "shutdown", "records"] {
        let f = stopped(State::Failed)?;
        let input = authorize(&f, "verify")?;
        if reason == "drift" {
            fs::write(
                workspace(&f, &attempts(&f, "head")?[0])?.join("partial.txt"),
                "external edit",
            )?;
        }
        let (receipt, lease) = resume::execute_with(
            &f.runtime,
            &input,
            &AtomicBool::new(reason == "shutdown"),
            || {
                if reason == "records" {
                    mutate(
                        &f,
                        "object_task",
                        "head",
                        "/revision",
                        serde_json::json!(999),
                    )?;
                }
                Ok(())
            },
        )?;
        assert!(
            matches!(receipt.result, Some(Outcome::Blocked { .. })),
            "{reason}"
        );
        assert!(lease.is_none());
        assert_eq!(attempts(&f, "head")?.len(), 1);
        assert_eq!(resume::replay(&f.runtime, &input)?, Some(receipt));
    }
    Ok(())
}

#[test]
fn awaiting_acceptance_cannot_be_retried() -> Result<()> {
    let f = stopped(State::AwaitingGate)?;
    let input = authorize(&f, "verify")?;
    assert!(
        !recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .can_resume
    );
    assert_eq!(
        resume::execute(&f.runtime, &input, &AtomicBool::new(false))
            .err()
            .unwrap()
            .to_string(),
        "OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED"
    );
    assert_eq!(attempts(&f, "head")?.len(), 1);
    Ok(())
}
