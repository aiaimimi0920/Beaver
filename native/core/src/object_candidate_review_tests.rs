use super::{attempt_fixture::*, object_recovery_verification::request as verify_request};
use crate::{
    object_attempt::{self, State},
    object_attempt_checks as checks,
    object_attempt_view::Target,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{
        self as recovery,
        candidate::{self, Request},
        resume,
    },
    object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::{fs, sync::atomic::AtomicBool};

fn finish(f: &Fixture, lease: crate::object_attempt::Lease) -> Result<()> {
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )
}

pub(super) fn final_output(f: &Fixture) -> Result<Request> {
    final_output_with(f, |_| Ok(()))
}

pub(super) fn final_output_with(
    f: &Fixture,
    output: impl FnOnce(&std::path::Path) -> Result<()>,
) -> Result<Request> {
    let lease = start(f)?;
    fs::write(
        workspace(f, lease.record())?.join("hero.gd"),
        "extends Node\n",
    )?;
    finish(f, lease)?;
    let attempt = attempts(f, "head")?.remove(0);
    let early = Request {
        project_id: "project-1".into(),
        request_id: "candidate".into(),
        target: Target::from_record(&attempt),
        check_request_id: "check-a".into(),
    };
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: early.project_id.clone(),
            request_id: early.check_request_id.clone(),
            target: early.target.clone(),
        },
    )?;
    assert!(candidate::prepare(&f.runtime, &early)
        .unwrap_err()
        .to_string()
        .contains("FINAL_FINE_REQUIRED"));
    recovery::verify(&f.runtime, &verify_request(f, "verify-a")?, false)?;
    let advance = resume::Request {
        project_id: "project-1".into(),
        request_id: "advance-a".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-a".into(),
        rework: None,
        advance: Some(resume::advance::Approval {
            attempt_id: attempt.id,
            check_request_id: "check-a".into(),
            next_fine_task_id: "fine-b".into(),
            next_fine_revision: task_record(f, "fine-b")?.revision,
            acceptance_note: "Owner reviewed A".into(),
        }),
    };
    let (_, lease) = resume::execute(&f.runtime, &advance, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    fs::write(
        workspace(f, lease.record())?.join("hero.gd"),
        "extends Node\nvar speed = 3\n",
    )?;
    output(&workspace(f, lease.record())?)?;
    let target = Target::from_record(lease.record());
    finish(f, lease)?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-b".into(),
            target: target.clone(),
        },
    )?;
    Ok(Request {
        target,
        check_request_id: "check-b".into(),
        ..early
    })
}

#[test]
fn candidate_review_freezes_final_output_without_accepting_and_survives_cancel_reopen() -> Result<()>
{
    let f = fixture()?;
    let request = final_output(&f)?;
    let before = serde_json::to_value(object_tasks::snapshot(&f.runtime, "project-1")?)?;
    let queue = serde_json::to_value(object_tasks::queue(&f.runtime, "project-1")?)?;
    let report = candidate::prepare(&f.runtime, &request)?;
    assert_eq!(report.stages[0].status, "accepted");
    assert_eq!(report.stages[1].status, "awaitingAcceptance");
    let file = report
        .files
        .iter()
        .find(|file| file.path == "hero.gd")
        .unwrap();
    assert!(file.before.is_none()); // Compared to original run, not A's output.
    assert!(file.after.is_some());
    assert!(report
        .blockers
        .iter()
        .any(|b| b == "FILE_OWNERSHIP_REVIEW_REQUIRED: hero.gd"));
    assert!(report.rules.iter().all(|rule| rule.passed));
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&f.runtime, "project-1")?)?,
        before
    );
    assert_eq!(
        serde_json::to_value(object_tasks::queue(&f.runtime, "project-1")?)?,
        queue
    );
    recovery::verify(&f.runtime, &verify_request(&f, "verify-cancel")?, false)?;
    recovery::disposition::dispose(
        &f.runtime,
        &recovery::disposition::Request {
            project_id: "project-1".into(),
            request_id: "cancel".into(),
            target: recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .target,
            verification_request_id: "verify-cancel".into(),
            choice: recovery::disposition::Choice::CancelAndRemoveWorkspace,
        },
        false,
    )?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(candidate::prepare(&runtime, &request)?, report);
    assert_eq!(
        candidate::list(&runtime, "project-1", &request.target.attempt_id)?,
        vec![report]
    );
    assert!(candidate::prepare(
        &runtime,
        &Request {
            request_id: "new".into(),
            ..request
        }
    )
    .is_err());
    Ok(())
}

#[test]
fn candidate_review_reports_fresh_corruption_and_drift_without_rewriting_history() -> Result<()> {
    let f = fixture()?;
    let request = final_output(&f)?;
    let report = candidate::prepare(&f.runtime, &request)?;
    let file = report
        .files
        .iter()
        .find(|file| file.path == "hero.gd")
        .unwrap();
    fs::write(
        f.runtime.files().blob(file.after.as_ref().unwrap())?,
        "corrupt",
    )?;
    mutate(&f, "object", "hero", "/revision", serde_json::json!(1))?;
    let fresh = candidate::prepare(
        &f.runtime,
        &Request {
            request_id: "fresh".into(),
            ..request.clone()
        },
    )?;
    assert!(fresh.blockers.iter().any(|b| b == "TECHNICAL_CHECK_FAILED"));
    assert!(fresh.blockers.iter().any(|b| b == "OBJECT_REVISION_DRIFT"));
    assert_eq!(candidate::prepare(&f.runtime, &request)?, report);
    assert!(candidate::prepare(
        &f.runtime,
        &Request {
            check_request_id: "wrong".into(),
            ..request.clone()
        }
    )
    .unwrap_err()
    .to_string()
    .contains("REQUEST_CONFLICT"));
    assert!(candidate::prepare(
        &f.runtime,
        &Request {
            project_id: "other".into(),
            ..request.clone()
        }
    )
    .is_err());
    assert!(candidate::prepare(
        &f.runtime,
        &Request {
            request_id: "unverified".into(),
            check_request_id: "missing".into(),
            ..request
        }
    )
    .is_err());
    Ok(())
}
