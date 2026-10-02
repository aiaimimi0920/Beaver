use super::*;

#[path = "project_derivation_candidate_tests.rs"]
mod candidate;
#[path = "project_derivation_retry_fixture.rs"]
mod fixture;
#[path = "project_derivation_gate_tests.rs"]
mod gate;
#[path = "project_derivation_retry_rejection_tests.rs"]
mod rejection;
#[path = "project_derivation_stage_history_tests.rs"]
mod stages;
use fixture::RetryFixture;

#[test]
fn project_derivation_retry_preserves_complete_history_and_replays_without_leases() -> Result<()> {
    for baseline in ["empty", "latest", "pinned"] {
        let f = RetryFixture::new(baseline)?;
        let source_before = data_backup::inventory(&f.first.base.source)?;
        let preparation = f.first.base.temp.path().join("retry-prepared");
        let prepared = copy::prepare(f.first.request_with_reordered_closure()?, &preparation)?;
        let map = |kind: &str, id: &str| -> Result<String> {
            Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
        };
        let medium = map("object_task", "build")?;
        let run = map("object_run", &f.first.attempt.preparation.run.id)?;
        let target = f.first.base.temp.path().join("retry-target");
        let prepared_before = data_backup::inventory(&preparation)?;
        assembly::create(&preparation, &target)?;
        assembly::activate(&preparation, &target)?;
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        let mut expected = f.attempts.clone();
        for attempt in &mut expected {
            crate::project_derivation_execution_records::attempt(
                &prepared.identities,
                &prepared.request,
                &Default::default(),
                attempt,
            )?;
        }
        let history = object_attempt::list(&runtime, &run)?;
        assert_eq!(history.len(), 3);
        assert!(expected.iter().all(|a| history.contains(a)));
        assert!(recovery::get(&runtime, "derived", &medium)?.unwrap().paused);
        assert!(
            crate::object_run_preparation::claim_next(&runtime, "derived", "no-auto-start")?
                .is_none()
        );
        for (kind, requests) in [
            ("object_recovery_verification", &f.verifications),
            ("object_recovery_resume", &f.resumes),
        ] {
            for id in requests {
                let value: Value = runtime
                    .store()
                    .lock()
                    .unwrap()
                    .get(kind, &map(kind, &RetryFixture::key(id)?)?)?
                    .unwrap();
                let op = value["operation"].clone();
                if kind == "object_recovery_verification" {
                    let operation: recovery::Operation = serde_json::from_value(op)?;
                    assert_eq!(
                        recovery::verify(&runtime, &operation.request, false)?,
                        operation
                    );
                } else {
                    let operation: resume::Operation = serde_json::from_value(op)?;
                    let (replayed, lease) =
                        resume::execute(&runtime, &operation.request, &AtomicBool::new(false))?;
                    assert_eq!(replayed, operation);
                    assert!(lease.is_none());
                }
            }
        }
        for attempt in &expected {
            let reports = checks::list(&runtime, "derived", &attempt.id)?;
            assert_eq!(reports.len(), 1);
            assert_eq!(checks::run(&runtime, &reports[0].request)?, reports[0]);
            let trace = trace::read(
                &runtime,
                &trace::Request {
                    project_id: "derived".into(),
                    run_id: run.clone(),
                    attempt_id: attempt.id.clone(),
                },
            )?;
            assert_eq!(trace.entries.len(), 1);
        }
        let stale = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(!stale.report_matches_records && !stale.can_resume);
        pause(&runtime, &medium, "retry-copy-unpause", false)?;
        assert!(crate::object_run_preparation::claim_next(
            &runtime,
            "derived",
            "still-no-auto-start"
        )?
        .is_none());
        let current = recovery::get(&runtime, "derived", &medium)?.unwrap();
        recovery::verify(
            &runtime,
            &recovery::VerifyRequest {
                project_id: "derived".into(),
                request_id: "new-verify".into(),
                target: current.target,
            },
            false,
        )?;
        let verified = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(verified.can_resume);
        let request = resume::Request {
            project_id: "derived".into(),
            request_id: "new-resume".into(),
            target: verified.target,
            verification_request_id: "new-verify".into(),
            advance: None,
            rework: None,
        };
        let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert_eq!(
            lease.record().input,
            expected.last().unwrap().output.clone().unwrap()
        );
        assert!(!expected.iter().any(|a| a.id == lease.record().id));
        assert!(lease.record().thread_id.is_none() && lease.record().turn_id.is_none());
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
        let history = object_attempt::list(&runtime, &run)?;
        assert_eq!(history.len(), 4);
        assert!(expected.iter().all(|a| history.contains(a)));
        assert_eq!(data_backup::inventory(&f.first.base.source)?, source_before);
        assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    }
    Ok(())
}

#[test]
fn project_derivation_retry_rederiving_a_paused_copy_preserves_control_revisions() -> Result<()> {
    let f = RetryFixture::new("latest")?;
    let preparation = f.first.base.temp.path().join("prepared");
    copy::prepare(f.first.base.request(), &preparation)?;
    let target = f.first.base.temp.path().join("target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let before = object_tasks::snapshot(&runtime, "derived")?.dispatch_controls;
    drop(runtime);
    let second_prepared = f.first.base.temp.path().join("second-prepared");
    copy::prepare(
        copy::Request {
            request_id: "derive-retry-again".into(),
            source: target.join("project"),
            source_project_id: "derived".into(),
            target_project_id: "again".into(),
        },
        &second_prepared,
    )?;
    let second = f.first.base.temp.path().join("second");
    assembly::create(&second_prepared, &second)?;
    assembly::activate(&second_prepared, &second)?;
    let runtime = ProjectStore::open(&second.join("project"), "again")?.into_runtime();
    let after = object_tasks::snapshot(&runtime, "again")?.dispatch_controls;
    let revisions = |controls: &[crate::object_task_dispatch::Control]| {
        let mut revisions: Vec<_> = controls.iter().map(|c| c.revision).collect();
        revisions.sort();
        revisions
    };
    assert_eq!(revisions(&before), revisions(&after));
    assert!(after.iter().all(|c| c.paused));
    assert!(
        crate::object_run_preparation::claim_next(&runtime, "again", "no-auto-start")?.is_none()
    );
    Ok(())
}
