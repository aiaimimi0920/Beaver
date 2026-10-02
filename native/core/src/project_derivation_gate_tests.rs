use super::*;

#[path = "project_derivation_gate_rejection_tests.rs"]
mod rejection;

#[test]
fn project_derivation_gate_preserves_retry_history_and_requires_new_advance() -> Result<()> {
    for baseline in ["empty", "latest", "pinned"] {
        let f = RetryFixture::with_terminal(baseline, State::AwaitingGate)?;
        let source_before = data_backup::inventory(&f.first.base.source)?;
        let preparation = f.first.base.temp.path().join("gate-prepared");
        let prepared = copy::prepare(f.first.request_with_reordered_closure()?, &preparation)?;
        let map = |kind: &str, id: &str| -> Result<String> {
            Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
        };
        let medium = map("object_task", "build")?;
        let run = map("object_run", &f.first.attempt.preparation.run.id)?;
        let target = f.first.base.temp.path().join("gate-target");
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
        let candidate = expected.last().unwrap();
        assert_eq!(candidate.state, State::AwaitingGate);
        let current = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(current.paused && !current.can_resume && !current.can_dispose);
        for (kind, requests) in [
            ("object_recovery_verification", &f.verifications),
            ("object_recovery_resume", &f.resumes),
        ] {
            for id in requests {
                let saved: Value = runtime
                    .store()
                    .lock()
                    .unwrap()
                    .get(kind, &map(kind, &RetryFixture::key(id)?)?)?
                    .unwrap();
                if kind == "object_recovery_verification" {
                    let operation: recovery::Operation =
                        serde_json::from_value(saved["operation"].clone())?;
                    assert_eq!(
                        recovery::verify(&runtime, &operation.request, false)?,
                        operation
                    );
                } else {
                    let operation: resume::Operation =
                        serde_json::from_value(saved["operation"].clone())?;
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
        }
        pause(&runtime, &medium, "gate-unpause", false)?;
        assert!(
            crate::object_run_preparation::claim_next(&runtime, "derived", "no-auto-start")?
                .is_none()
        );
        let current = recovery::get(&runtime, "derived", &medium)?.unwrap();
        recovery::verify(
            &runtime,
            &recovery::VerifyRequest {
                project_id: "derived".into(),
                request_id: "gate-fresh-verify".into(),
                target: current.target,
            },
            false,
        )?;
        let verified = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(verified.can_dispose && !verified.can_resume);
        let mut request = resume::Request {
            project_id: "derived".into(),
            request_id: "gate-plain-retry".into(),
            target: verified.target,
            verification_request_id: "gate-fresh-verify".into(),
            advance: None,
            rework: None,
        };
        assert_eq!(
            resume::execute(&runtime, &request, &AtomicBool::new(false))
                .err()
                .unwrap()
                .to_string(),
            "OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED"
        );
        let next_id = map("object_task", "later-fine")?;
        let snapshot = object_tasks::snapshot(&runtime, "derived")?;
        let next = snapshot.tasks.iter().find(|t| t.id == next_id).unwrap();
        request.request_id = "gate-explicit-advance".into();
        request.advance = Some(resume::advance::Approval {
            attempt_id: candidate.id.clone(),
            check_request_id: map("object_attempt_check_report", "retry-check-1")?,
            next_fine_task_id: next_id.clone(),
            next_fine_revision: next.revision,
            acceptance_note: "Reviewed the copied candidate".into(),
        });
        let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert_eq!(lease.record().fine.id, next_id);
        assert_eq!(Some(&lease.record().input), candidate.output.as_ref());
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
        let snapshot = object_tasks::snapshot(&runtime, "derived")?;
        assert_eq!(
            snapshot
                .tasks
                .iter()
                .find(|t| t.id == candidate.fine.id)
                .unwrap()
                .status,
            "accepted"
        );
        assert_eq!(object_attempt::list(&runtime, &run)?.len(), 4);
        drop(runtime);
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        assert_eq!(
            resume::execute(&runtime, &request, &AtomicBool::new(false))?.0,
            receipt
        );
        assert_eq!(object_attempt::list(&runtime, &run)?.len(), 4);
        assert_eq!(data_backup::inventory(&f.first.base.source)?, source_before);
        assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    }
    Ok(())
}

#[test]
fn project_derivation_gate_first_success_and_rederivation_keep_safe_pause() -> Result<()> {
    let f = ExecutionFixture::new(State::AwaitingGate, "empty")?;
    let preparation = f.base.temp.path().join("prepared");
    let prepared = copy::prepare(f.base.request(), &preparation)?;
    let target = f.base.temp.path().join("target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let medium = Rewrite(&prepared.identities)
        .key("object_task", "build")?
        .id;
    let current = recovery::get(&runtime, "derived", &medium)?.unwrap();
    assert!(current.paused);
    let before = object_tasks::snapshot(&runtime, "derived")?.dispatch_controls;
    drop(runtime);
    let second_prepared = f.base.temp.path().join("prepared-again");
    copy::prepare(
        copy::Request {
            request_id: "derive-gate-again".into(),
            source: target.join("project"),
            source_project_id: "derived".into(),
            target_project_id: "again".into(),
        },
        &second_prepared,
    )?;
    let second = f.base.temp.path().join("second");
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
