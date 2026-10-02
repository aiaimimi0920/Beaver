use super::*;

#[path = "project_derivation_stage_rejection_tests.rs"]
mod rejection;
#[path = "project_derivation_stage_fixture.rs"]
pub(super) mod stage_fixture;
use stage_fixture::{verify, StageFixture};

#[test]
fn project_derivation_stage_history_preserves_accepted_fines_and_replays_without_leases(
) -> Result<()> {
    for (advances, state) in [
        (1, State::Failed),
        (2, State::Interrupted),
        (1, State::AwaitingGate),
    ] {
        let stage = StageFixture::new(advances, state.clone())?;
        let f = &stage.retry;
        let base = &f.first.base;
        let before = data_backup::inventory(&base.source)?;
        // The first stage's retry revision is higher than the actual chain tail's revision.
        assert!(f.attempts[2].fine.revision > f.attempts.last().unwrap().fine.revision);
        let preparation = base.temp.path().join("stage-prepared");
        let prepared = copy::prepare(base.request(), &preparation)?;
        let map = |kind: &str, id: &str| -> Result<String> {
            Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
        };
        let target = base.temp.path().join("stage-target");
        assembly::create(&preparation, &target)?;
        assembly::activate(&preparation, &target)?;
        let prepared_before = data_backup::inventory(&preparation)?;
        let project = "derived";
        let run = map("object_run", &f.first.attempt.preparation.run.id)?;
        let medium = map("object_task", "build")?;
        for _ in 0..2 {
            let runtime = ProjectStore::open(&target.join("project"), project)?.into_runtime();
            let history = object_attempt::list(&runtime, &run)?;
            assert_eq!(history.len(), f.attempts.len());
            for original in &f.attempts {
                let copied = history
                    .iter()
                    .find(|a| a.id == map("object_attempt", &original.id).unwrap())
                    .unwrap();
                assert_eq!(copied.input, original.input);
                assert_eq!(copied.output, original.output);
                assert_eq!(copied.fine.prompt, original.fine.prompt);
            }
            let snapshot = object_tasks::snapshot(&runtime, project)?;
            for previous in f
                .attempts
                .windows(2)
                .filter(|p| p[0].fine.id != p[1].fine.id)
            {
                let mut accepted = resume::advance::accepted(&previous[0])?;
                crate::project_derivation_plan_rewrite::PlanIds(&prepared.request)
                    .task(&mut accepted)?;
                assert!(snapshot.tasks.contains(&accepted));
            }
            for request in &f.resumes {
                let saved: Value = runtime
                    .store()
                    .lock()
                    .unwrap()
                    .get(
                        "object_recovery_resume",
                        &map("object_recovery_resume", &RetryFixture::key(request)?)?,
                    )?
                    .unwrap();
                let op: resume::Operation = serde_json::from_value(saved["operation"].clone())?;
                let (actual, lease) =
                    resume::execute(&runtime, &op.request, &AtomicBool::new(false))?;
                assert_eq!(actual, op);
                assert!(lease.is_none());
                if let Some(approval) = &op.request.advance {
                    assert!(approval
                        .acceptance_note
                        .starts_with("Reviewed source stage"));
                    assert!(checks::list(&runtime, project, &approval.attempt_id)?
                        .iter()
                        .any(|c| c.request.request_id == approval.check_request_id));
                }
            }
            assert!(recovery::get(&runtime, project, &medium)?.unwrap().paused);
            assert!(
                crate::object_run_preparation::claim_next(&runtime, project, "no-auto-start")?
                    .is_none()
            );
        }
        let runtime = ProjectStore::open(&target.join("project"), project)?.into_runtime();
        pause(&runtime, &medium, "stage-unpause", false)?;
        let fresh = verify(&runtime, "fresh-stage-verify")?;
        let mut request = resume::Request {
            project_id: project.into(),
            request_id: "fresh-stage-action".into(),
            target: fresh.target,
            verification_request_id: "fresh-stage-verify".into(),
            advance: None,
            rework: None,
        };
        if state == State::AwaitingGate {
            let current = f.attempts.last().unwrap();
            let id = map("object_attempt", &current.id)?;
            let attempt = object_attempt::list(&runtime, &run)?
                .into_iter()
                .find(|a| a.id == id)
                .unwrap();
            let report = checks::run(
                &runtime,
                &checks::Request {
                    project_id: project.into(),
                    request_id: "fresh-stage-check".into(),
                    target: crate::object_attempt_view::Target::from_record(&attempt),
                },
            )?;
            assert!(report.passed);
            request.advance = Some(resume::advance::Approval {
                attempt_id: id,
                check_request_id: report.request.request_id,
                next_fine_task_id: map("object_task", "final-fine")?,
                next_fine_revision: 0,
                acceptance_note: "Explicit copied stage approval".into(),
            });
        }
        let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert_eq!(
            Some(&lease.record().input),
            f.attempts.last().unwrap().output.as_ref()
        );
        object_attempt::finish(
            &runtime,
            lease,
            State::Failed,
            None,
            &AtomicBool::new(false),
        )?;
        let (replay, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
        assert_eq!(replay, receipt);
        assert!(lease.is_none());
        assert_eq!(
            object_attempt::list(&runtime, &run)?.len(),
            f.attempts.len() + 1
        );
        assert_eq!(data_backup::inventory(&base.source)?, before);
        assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    }
    Ok(())
}
