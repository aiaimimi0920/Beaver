use super::*;
use crate::object_run_recovery::candidate;

#[path = "project_derivation_candidate_rejection_tests.rs"]
mod rejection;
#[path = "project_derivation_candidate_fixture.rs"]
mod review_fixture;
#[path = "project_derivation_rework_tests.rs"]
mod rework;
use review_fixture::CandidateFixture;
const KIND: &str = "object_candidate_review";

#[test]
fn project_derivation_candidate_maps_frozen_v1_v2_and_only_fresh_review_authorizes_rework(
) -> Result<()> {
    for (retry, baseline, schema, staged) in [
        (false, "empty", 1, false),
        (false, "pinned", 2, false),
        (true, "pinned", 1, false),
        (true, "empty", 2, false),
        (true, "empty", 1, true),
        (true, "empty", 2, true),
    ] {
        let f = if staged {
            CandidateFixture::staged(schema)?
        } else {
            CandidateFixture::new(retry, baseline, schema)?
        };
        let base = &f.execution.base;
        // Legitimate metadata changes after review must not replace its original snapshot.
        let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
        let attempt_count = object_attempt::list(&runtime, &f.terminal.preparation.run.id)?.len();
        let mut child = runtime
            .store()
            .lock()
            .unwrap()
            .get::<crate::object_catalog::ObjectRecord>("object", &base.child.id)?
            .unwrap();
        child.name = "Renamed AFTER frozen review: original source-review".into();
        objects::update(&runtime, &child, "post-review-metadata")?;
        drop(runtime);
        let source_before = data_backup::inventory(&base.source)?;
        let preparation = base.temp.path().join("review-prepared");
        let prepared = copy::prepare(f.execution.request_with_reordered_closure()?, &preparation)?;
        let map = |kind: &str, id: &str| -> Result<String> {
            Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
        };
        let medium = map("object_task", "build")?;
        let run = map("object_run", &f.terminal.preparation.run.id)?;
        let id = map("object_attempt", &f.terminal.id)?;
        let prepared_before = data_backup::inventory(&preparation)?;
        let target = base.temp.path().join("review-target");
        assembly::create(&preparation, &target)?;
        assembly::activate(&preparation, &target)?;
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        let reports = candidate::list(&runtime, "derived", &id)?;
        assert_eq!(reports.len(), 1);
        let old = &reports[0];
        assert_eq!(old.schema_version, schema);
        assert_eq!(old.time, f.review.time);
        assert_eq!(old.output_digest, f.review.output_digest);
        assert_ne!(old.source_digest, f.review.source_digest);
        assert_eq!(old.rules, f.review.rules);
        assert_eq!(old.blockers, f.review.blockers);
        assert_eq!(
            old.object_revision_at_review,
            f.review.object_revision_at_review
        );
        assert_eq!(old.request.request_id, map(KIND, "source-review")?);
        assert_eq!(
            old.request.check_request_id,
            map(
                "object_attempt_check_report",
                &f.review.request.check_request_id
            )?
        );
        assert_eq!(old.stages[0].task_id, map("object_task", "fine")?);
        assert_eq!(old.stages[0].title, f.review.stages[0].title);
        assert_eq!(
            old.baseline_version_id,
            f.review
                .baseline_version_id
                .as_ref()
                .map(|v| map("object_version", v))
                .transpose()?
        );
        for (source, copied) in f.review.files.iter().zip(&old.files) {
            assert_eq!(
                (
                    &source.path,
                    &source.before,
                    &source.after,
                    source.reference
                ),
                (
                    &copied.path,
                    &copied.before,
                    &copied.after,
                    copied.reference
                )
            );
            let mut owners = source
                .owners
                .iter()
                .map(|id| map("object", id))
                .collect::<Result<Vec<_>>>()?;
            owners.sort();
            assert_eq!(copied.owners, owners);
        }
        let mut refs = f
            .review
            .references
            .iter()
            .map(|r| {
                Ok(candidate::Reference {
                    object_id: map("object", &r.object_id)?,
                    version_id: map("object_version", &r.version_id)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        refs.sort_by(|a, b| (&a.object_id, &a.version_id).cmp(&(&b.object_id, &b.version_id)));
        assert_eq!(old.references, refs);
        let before = object_tasks::snapshot(&runtime, "derived")?;
        assert_eq!(candidate::prepare(&runtime, &old.request)?, *old);
        assert_eq!(object_tasks::snapshot(&runtime, "derived")?, before);
        assert!(recovery::get(&runtime, "derived", &medium)?.unwrap().paused);
        drop(runtime);
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        assert_eq!(candidate::list(&runtime, "derived", &id)?, reports);
        pause(&runtime, &medium, "review-unpause", false)?;
        assert!(
            crate::object_run_preparation::claim_next(&runtime, "derived", "no-auto-start")?
                .is_none()
        );
        let verified = review_fixture::verify(&runtime, &medium)?;
        assert!(verified.can_dispose && !verified.can_resume);
        let mut request: resume::Request = serde_json::from_value(json!({
            "projectId":"derived", "requestId":"old-review-cannot-authorize", "target":verified.target,
            "verificationRequestId":"fresh-review-verify", "rework":{
                "reviewRequestId":old.request.request_id, "attemptId":id,
                "fineTaskId":old.request.target.fine_task_id, "feedback":"Reduce movement speed"
            }
        }))?;
        assert_eq!(
            resume::execute(&runtime, &request, &AtomicBool::new(false))
                .err()
                .unwrap()
                .to_string(),
            "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH"
        );
        let check = checks::run(
            &runtime,
            &checks::Request {
                project_id: "derived".into(),
                request_id: "fresh-review-check".into(),
                target: old.request.target.clone(),
            },
        )?;
        assert!(check.passed);
        let fresh = candidate::prepare(
            &runtime,
            &candidate::Request {
                project_id: "derived".into(),
                request_id: "fresh-review".into(),
                target: old.request.target.clone(),
                check_request_id: check.request.request_id,
            },
        )?;
        assert!(fresh.object_revision_at_review > old.object_revision_at_review);
        let both = candidate::list(&runtime, "derived", &id)?;
        assert_eq!(both.len(), 2);
        assert!(both.contains(old) && both.contains(&fresh));
        assert!(crate::object_run_preparation::claim_next(
            &runtime,
            "derived",
            "still-no-auto-start"
        )?
        .is_none());
        request.request_id = "explicit-fresh-rework".into();
        request.rework.as_mut().unwrap().review_request_id = fresh.request.request_id;
        let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert_eq!(Some(&lease.record().input), f.terminal.output.as_ref());
        assert!(lease.record().thread_id.is_none() && lease.record().turn_id.is_none());
        assert!(lease.record().fine.prompt.contains("Reduce movement speed"));
        object_attempt::finish(
            &runtime,
            lease,
            State::Failed,
            None,
            &AtomicBool::new(false),
        )?;
        let (replayed, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        assert_eq!(replayed, receipt);
        assert!(lease.is_none());
        assert_eq!(
            object_attempt::list(&runtime, &run)?.len(),
            attempt_count + 1
        );
        assert_eq!(candidate::prepare(&runtime, &old.request)?, *old);
        assert_eq!(data_backup::inventory(&base.source)?, source_before);
        assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
        if staged {
            drop(runtime);
            let preparation = base.temp.path().join("supported-rework-history");
            let prepared = copy::prepare(
                copy::Request {
                    request_id: "derive-after-rework".into(),
                    source: target.join("project"),
                    source_project_id: "derived".into(),
                    target_project_id: "rework-copy".into(),
                },
                &preparation,
            )?;
            let destination = base.temp.path().join("rework-history-target");
            assembly::create(&preparation, &destination)?;
            assembly::activate(&preparation, &destination)?;
            let runtime =
                ProjectStore::open(&destination.join("project"), "rework-copy")?.into_runtime();
            let mapped_run = Rewrite(&prepared.identities).key("object_run", &run)?.id;
            assert_eq!(
                object_attempt::list(&runtime, &mapped_run)?.len(),
                attempt_count + 1
            );
            let mapped_attempt = Rewrite(&prepared.identities).key("object_attempt", &id)?.id;
            assert_eq!(
                candidate::list(&runtime, "rework-copy", &mapped_attempt)?.len(),
                2
            );
        }
    }
    Ok(())
}

#[test]
fn project_derivation_candidate_rederivation_preserves_history_and_pause_revision() -> Result<()> {
    let f = CandidateFixture::staged(2)?;
    let base = &f.execution.base;
    let mut source = base.source.clone();
    let mut source_id = "original".to_string();
    let mut medium = "build".to_string();
    let mut attempt = f.terminal.id.clone();
    let mut previous_control = None;
    for index in 0..2 {
        let target_id = format!("derived-{index}");
        let preparation = base.temp.path().join(format!("prepared-{index}"));
        let prepared = copy::prepare(
            copy::Request {
                source,
                source_project_id: source_id,
                target_project_id: target_id.clone(),
                request_id: format!("again-{index}"),
            },
            &preparation,
        )?;
        medium = Rewrite(&prepared.identities)
            .key("object_task", &medium)?
            .id;
        attempt = Rewrite(&prepared.identities)
            .key("object_attempt", &attempt)?
            .id;
        let target = base.temp.path().join(format!("target-{index}"));
        assembly::create(&preparation, &target)?;
        assembly::activate(&preparation, &target)?;
        source = target.join("project");
        source_id = target_id;
        let runtime = ProjectStore::open(&source, &source_id)?.into_runtime();
        let view = recovery::get(&runtime, &source_id, &medium)?.unwrap();
        assert!(view.paused);
        if let Some(revision) = previous_control {
            assert_eq!(view.target.control_revision, revision);
        }
        previous_control = Some(view.target.control_revision);
        let reviews = candidate::list(&runtime, &source_id, &attempt)?;
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].time, f.review.time);
        assert_eq!(reviews[0].rules, f.review.rules);
        assert_eq!(
            candidate::prepare(&runtime, &reviews[0].request)?,
            reviews[0]
        );
        assert!(
            crate::object_run_preparation::claim_next(&runtime, &source_id, "no-auto-start")?
                .is_none()
        );
    }
    Ok(())
}
