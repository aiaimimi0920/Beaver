use super::{
    attempt_fixture::*, object_candidate_review::final_output,
    object_recovery_verification::request as verify_request,
};
use crate::{
    object_attempt::{self, State},
    object_attempt_view,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{self as recovery, candidate, resume},
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

fn authorize(f: &Fixture) -> Result<resume::Request> {
    authorize_review(f, final_output(f)?)
}

fn authorize_review(f: &Fixture, review: candidate::Request) -> Result<resume::Request> {
    candidate::prepare(&f.runtime, &review)?;
    recovery::verify(&f.runtime, &verify_request(f, "verify-rework")?, false)?;
    Ok(resume::Request {
        project_id: "project-1".into(),
        request_id: "rework".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-rework".into(),
        advance: None,
        rework: Some(resume::rework::Approval {
            review_request_id: review.request_id,
            attempt_id: review.target.attempt_id,
            fine_task_id: "fine-b".into(),
            feedback: "Reduce movement speed".into(),
            image: None,
            preview_frame: None,
            relocation: None,
        }),
    })
}

#[path = "object_rework_image_tests.rs"]
mod image;

#[test]
fn candidate_rework_preserves_stages_feedback_history_and_exact_replay() -> Result<()> {
    let f = fixture()?;
    let request = authorize(&f)?;
    let previous = attempts(&f, "head")?;
    let original = previous.iter().find(|a| a.fine.id == "fine-b").unwrap();
    let accepted = task_record(&f, "fine-a")?;
    let reports = candidate::list(&f.runtime, "project-1", &original.id)?;
    let (receipt, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    assert_eq!(lease.record().input, original.output.clone().unwrap());
    assert!(lease
        .record()
        .fine
        .prompt
        .ends_with("Reduce movement speed"));
    assert_eq!(lease.record().fine.id, "fine-b");
    assert_eq!(task_record(&f, "fine-a")?, accepted);
    assert_eq!(
        candidate::list(&f.runtime, "project-1", &original.id)?,
        reports
    );
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let views = object_attempt_view::list_with_details(&f.runtime, &request.target.run_id)?;
    assert_eq!(views.len(), 3);
    assert!(attempts(&f, "head")?.contains(original));
    recovery::verify(&f.runtime, &verify_request(&f, "verify-after")?, false)?;
    let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(view.can_dispose && !view.can_resume);
    assert_eq!(view.target.claim_token, request.target.claim_token);
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (replayed, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
    assert_eq!(replayed, receipt);
    assert!(lease.is_none());
    recovery::disposition::dispose(
        &runtime,
        &recovery::disposition::Request {
            project_id: "project-1".into(),
            request_id: "cancel".into(),
            target: view.target,
            verification_request_id: "verify-after".into(),
            choice: recovery::disposition::Choice::CancelAndKeep,
        },
        false,
    )?;
    assert_eq!(
        object_attempt_view::list_with_details(&runtime, &request.target.run_id)?.len(),
        3
    );
    Ok(())
}

#[test]
fn candidate_rework_pending_reopens_and_rejects_changed_feedback() -> Result<()> {
    let f = fixture()?;
    let request = authorize(&f)?;
    assert!(resume::execute_with(
        &f.runtime,
        &request,
        &AtomicBool::new(false),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    let mut changed = request.clone();
    changed.rework.as_mut().unwrap().feedback = "Different request".into();
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
    assert!(recovery::get(&runtime, "project-1", "head")?
        .unwrap()
        .resume
        .unwrap()
        .result
        .is_none());
    let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
    assert!(lease.is_some());
    let (replayed, duplicate) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
    assert_eq!(replayed, receipt);
    assert!(duplicate.is_none());
    Ok(())
}

#[test]
fn candidate_rework_rejects_stale_review_and_blocks_post_authorization_drift() -> Result<()> {
    for reason in ["review", "files", "catalog"] {
        let f = fixture()?;
        let mut request = authorize(&f)?;
        if reason == "review" {
            request.rework.as_mut().unwrap().review_request_id = "missing".into();
            assert!(resume::execute(&f.runtime, &request, &AtomicBool::new(false)).is_err());
        } else {
            if reason == "files" {
                let attempt = attempts(&f, "head")?
                    .into_iter()
                    .find(|a| a.fine.id == "fine-b")
                    .unwrap();
                std::fs::write(workspace(&f, &attempt)?.join("hero.gd"), "external change")?;
            }
            let (receipt, lease) =
                resume::execute_with(&f.runtime, &request, &AtomicBool::new(false), || {
                    if reason == "catalog" {
                        mutate(&f, "object", "hero", "/revision", serde_json::json!(9))?;
                    }
                    Ok(())
                })?;
            assert!(matches!(
                receipt.result,
                Some(resume::Outcome::Blocked { .. })
            ));
            assert!(lease.is_none());
        }
    }
    Ok(())
}
