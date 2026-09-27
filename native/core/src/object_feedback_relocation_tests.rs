use super::*;
use crate::{
    object_attempt::{self, State as AttemptState},
    object_attempt_callback as callback, object_attempt_checks as checks,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{self as recovery, candidate, resume},
};
use resume::rework::relocation;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

#[path = "object_feedback_relocation_fixture.rs"]
mod fixture;
use fixture::*;

#[path = "object_publication_evidence_tests.rs"]
mod publication_evidence;

#[test]
fn feedback_relocation_rejects_incomplete_foreign_and_stale_without_mutation() -> Result<()> {
    let f = fixture()?;
    let (_, request, _, _) = historical(&f)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let preview = publication::preview(&f.runtime, &request.review)?;
    for mutation in 0..8 {
        let mut bad = request.clone();
        let mapping = bad.relocation.as_mut().unwrap();
        match mutation {
            0 => {
                mapping.regions.pop();
            }
            1 => {
                mapping.regions[0] = relocation::Region::Matched {
                    source_region: 0,
                    target_region: 1,
                }
            }
            2 => mapping.source_attempt_id = request.review.target.attempt_id.clone(),
            3 => mapping.source_frame = request.preview_frame.clone().unwrap(),
            4 => mapping.confirmed = false,
            5 => bad.preview_frame = Some(mapping.source_frame.clone()),
            6 => bad.preview_frame = None,
            _ => bad.review.target.attempt_id = mapping.source_attempt_id.clone(),
        }
        assert!(
            deferred::create(&f.runtime, &bad).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(publication::preview(&f.runtime, &request.review)?, preview);
        assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    }
    let mut foreign = request.clone();
    foreign.project_id = "another-project".into();
    assert!(deferred::create(&f.runtime, &foreign).is_err());
    f.runtime.store().lock().unwrap().transaction(|db| {
        let target =
            crate::object_attempt_view::read(db, "project-1", &request.review.target.attempt_id)?;
        for other_run in [true, false] {
            let mut foreign = target.clone();
            if other_run {
                foreign.preparation.run.id = "another-run".into();
            } else {
                foreign.preparation.run.object_id = "another-object".into();
            }
            assert!(relocation::resolve(
                db,
                &foreign,
                request.preview_frame.as_ref().unwrap(),
                request.relocation.as_ref().unwrap()
            )
            .err()
            .unwrap()
            .to_string()
            .contains("SOURCE_MISMATCH"));
        }
        Ok(())
    })?;
    Ok(())
}

#[test]
fn feedback_relocation_deferred_reopens_publishes_and_delivers_both_original_frames() -> Result<()>
{
    let f = fixture()?;
    let (mut publish, feedback, source, target) = historical(&f)?;
    deferred::create(&f.runtime, &feedback)?;
    let f = reopen(f)?;
    assert_eq!(deferred::create(&f.runtime, &feedback)?, feedback);
    let mut changed = feedback.clone();
    changed.relocation.as_mut().unwrap().regions[1] = relocation::Region::Absent {
        source_region: 1,
        note: "Different owner intent".into(),
    };
    assert!(deferred::create(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_ID_CONFLICT"));
    approve(&f, &mut publish)?;
    let op = publication::publish(&f.runtime, &publish)?;
    assert_eq!(op.state, State::Published);
    let deferred = op
        .preview
        .feedback
        .iter()
        .filter(|item| item.later.is_some())
        .collect::<Vec<_>>();
    assert_eq!(deferred.len(), 1);
    assert_eq!(deferred[0].relocation, feedback.relocation);
    assert_eq!(deferred[0].preview_frame, feedback.preview_frame);
    assert_eq!(deferred[0].attempt_id, feedback.review.target.attempt_id);
    let receipt = followup::list(&f.runtime, "project-1", "publish")?.remove(0);
    let f = reopen(f)?;
    let mut lease = super::delivery::start_followup(&f, &receipt)?;
    let binding = assert_delivery(&f, &mut lease, &feedback, &source, &target)?;
    assert_eq!(binding["versionId"], op.version_id);
    assert!(lease.record().fine.prompt.contains("re-localize"));
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::Interrupted,
        Some("test complete".into()),
        &AtomicBool::new(false),
    )?;
    assert_eq!(deferred::create(&f.runtime, &feedback)?, feedback);
    let mut late = feedback.clone();
    late.request_id = "late-relocation".into();
    assert!(deferred::create(&f.runtime, &late).is_err());
    Ok(())
}

#[test]
fn feedback_relocation_rework_recovers_receipt_and_plain_retry_keeps_correspondence() -> Result<()>
{
    let f = fixture()?;
    let (_, feedback, source, target) = historical(&f)?;
    let request = rework(&f, &feedback, "relocated-rework", true)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut bad = request.clone();
    bad.rework
        .as_mut()
        .unwrap()
        .relocation
        .as_mut()
        .unwrap()
        .regions
        .pop();
    assert!(resume::execute(&f.runtime, &bad, &AtomicBool::new(false)).is_err());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert!(resume::execute_with(
        &f.runtime,
        &request,
        &AtomicBool::new(false),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    let f = reopen(f)?;
    let (receipt, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    assert_eq!(
        resume::execute(&f.runtime, &request, &AtomicBool::new(false))?.0,
        receipt
    );
    let mut lease = lease.unwrap();
    let binding = assert_delivery(&f, &mut lease, &feedback, &source, &target)?;
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::Interrupted,
        Some("interrupted".into()),
        &AtomicBool::new(false),
    )?;
    let mut retry = rework(&f, &feedback, "plain-retry", true)?;
    retry.rework = None;
    let (_, lease) = resume::execute(&f.runtime, &retry, &AtomicBool::new(false))?;
    let mut lease = lease.unwrap();
    assert_eq!(
        assert_delivery(&f, &mut lease, &feedback, &source, &target)?,
        binding
    );
    let mut publish = finish_candidate(&f, lease, "relocated-output")?;
    approve(&f, &mut publish)?;
    let op = publication::publish(&f.runtime, &publish)?;
    let saved = op
        .preview
        .feedback
        .iter()
        .find(|v| v.request_id == request.request_id)
        .unwrap();
    assert_eq!(saved.relocation, feedback.relocation);
    assert_eq!(saved.preview_frame, feedback.preview_frame);
    assert_eq!(saved.attempt_id, feedback.review.target.attempt_id);
    Ok(())
}
