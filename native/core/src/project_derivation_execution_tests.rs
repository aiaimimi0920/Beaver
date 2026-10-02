use crate::{
    data_backup,
    object_attempt::{self, State},
    object_attempt_checks as checks, object_attempt_control as control,
    object_attempt_trace as trace,
    object_run_recovery::{self as recovery, resume},
    object_tasks, project_derivation_assembly as assembly, project_derivation_copy as copy,
    project_derivation_validation_records::Rewrite,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

#[path = "project_derivation_execution_fixture.rs"]
mod fixture;
#[path = "project_derivation_execution_inventory_tests.rs"]
mod inventory;
#[path = "project_derivation_object_fixture.rs"]
mod objects;
#[path = "project_derivation_execution_rejection_tests.rs"]
mod rejection;
#[path = "project_derivation_retry_tests.rs"]
mod retry;
use fixture::{pause, ExecutionFixture};

#[test]
fn project_derivation_execution_stopped_history_replays_and_only_fresh_resume_creates_lease(
) -> Result<()> {
    for (state, baseline) in [
        (State::Failed, "empty"),
        (State::Failed, "latest"),
        (State::Interrupted, "pinned"),
    ] {
        let f = ExecutionFixture::new(state.clone(), baseline)?;
        let source_before = data_backup::inventory(&f.base.source)?;
        let preparation = f.base.temp.path().join("prepared");
        let prepared = copy::prepare(f.request_with_reordered_closure()?, &preparation)?;
        let map = |kind: &str, id: &str| -> Result<String> {
            Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
        };
        let medium = map("object_task", "build")?;
        let next = map("object_task", "next")?;
        let run = map("object_run", &f.attempt.preparation.run.id)?;
        let id = map("object_attempt", &f.attempt.id)?;
        let prepared_before = data_backup::inventory(&preparation)?;
        let target = f.base.temp.path().join("target");
        assembly::create(&preparation, &target)?;
        assembly::activate(&preparation, &target)?;
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        let attempt = object_attempt::list(&runtime, &run)?.remove(0);
        assert_eq!(attempt.id, id);
        assert_eq!(attempt.state, state);
        assert_eq!(attempt.input, f.attempt.input);
        assert_eq!(attempt.output, f.attempt.output);
        assert_eq!(attempt.fine.prompt, f.attempt.fine.prompt);
        assert_eq!(attempt.thread_id, f.attempt.thread_id);
        assert_eq!(attempt.turn_id, f.attempt.turn_id);
        assert_ne!(attempt.preparation.owner, f.attempt.preparation.owner);
        assert_ne!(
            attempt.preparation.claim_token,
            f.attempt.preparation.claim_token
        );
        let queue = object_tasks::queue(&runtime, "derived")?;
        let owner = queue.iter().find(|q| q.task_id == medium).unwrap();
        assert_eq!(owner.owner.as_ref(), Some(&attempt.preparation.owner));
        assert_eq!(
            owner.claim_token.as_ref(),
            Some(&attempt.preparation.claim_token)
        );
        let view = crate::object_task_queue_view::get(&runtime, "derived")?;
        assert!(view
            .items
            .iter()
            .find(|i| i.task_id == next)
            .unwrap()
            .blockers
            .contains(&"objectHeld".into()));
        assert!(recovery::get(&runtime, "derived", &medium)?.unwrap().paused);
        assert!(
            crate::object_run_preparation::claim_next(&runtime, "derived", "cannot-reclaim")?
                .is_none()
        );
        let reports = checks::list(&runtime, "derived", &id)?;
        assert_eq!(reports.len(), 1);
        assert_ne!(
            reports[0].attempt_digest,
            crate::framework_checks::digest(&f.attempt)?
        );
        assert_eq!(checks::run(&runtime, &reports[0].request)?, reports[0]);
        assert!(
            checks::run(
                &runtime,
                &checks::Request {
                    project_id: "derived".into(),
                    request_id: "fresh-check".into(),
                    target: reports[0].request.target.clone()
                }
            )?
            .passed
        );
        let copied_trace = trace::read(
            &runtime,
            &trace::Request {
                project_id: "derived".into(),
                run_id: run.clone(),
                attempt_id: id.clone(),
            },
        )?;
        assert_eq!(copied_trace.entries[0].operation, "codex");
        if state == State::Interrupted {
            let receipt: control::Pending = runtime
                .store()
                .lock()
                .unwrap()
                .get(control::KIND, &map(control::KIND, "interrupt")?)?
                .unwrap();
            assert_eq!(
                Some(control::result(&runtime, &receipt.request)?.result),
                receipt.result
            );
            control::request(&runtime, &receipt.request, false)?;
        }
        let receipt_key = crate::project_derivation_queue_records::receipt_key(
            "original",
            &f.unpaused.request.request_id,
        )?;
        let historical: crate::object_task_dispatch::Receipt = runtime
            .store()
            .lock()
            .unwrap()
            .get(
                "object_task_dispatch_receipt",
                &map("object_task_dispatch_receipt", &receipt_key)?,
            )?
            .unwrap();
        assert_eq!(
            crate::object_task_dispatch::set_paused(&runtime, &historical.request)?,
            historical
        );
        assert!(recovery::get(&runtime, "derived", &medium)?.unwrap().paused);
        drop(runtime);
        let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
        assert!(recovery::get(&runtime, "derived", &medium)?.unwrap().paused);
        // Verification while paused cannot authorize a resume after control changes.
        let before = recovery::get(&runtime, "derived", &medium)?.unwrap();
        recovery::verify(
            &runtime,
            &recovery::VerifyRequest {
                project_id: "derived".into(),
                request_id: "paused-verify".into(),
                target: before.target,
            },
            false,
        )?;
        let stale = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(!stale.can_resume);
        pause(&runtime, &medium, "explicit-unpause", false)?;
        assert!(resume::execute(
            &runtime,
            &resume::Request {
                project_id: "derived".into(),
                request_id: "stale-resume".into(),
                verification_request_id: "paused-verify".into(),
                target: stale.target,
                rework: None,
                advance: None
            },
            &AtomicBool::new(false)
        )
        .is_err());
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
                request_id: "fresh-verify".into(),
                target: current.target,
            },
            false,
        )?;
        let verified = recovery::get(&runtime, "derived", &medium)?.unwrap();
        assert!(verified.can_resume);
        let input = resume::Request {
            project_id: "derived".into(),
            request_id: "fresh-resume".into(),
            verification_request_id: "fresh-verify".into(),
            target: verified.target,
            rework: None,
            advance: None,
        };
        let (receipt, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
        let lease = lease.unwrap();
        assert_ne!(lease.record().id, id);
        assert!(lease.record().thread_id.is_none() && lease.record().turn_id.is_none());
        assert_eq!(lease.record().input, attempt.output.clone().unwrap());
        object_attempt::finish(
            &runtime,
            lease,
            State::Failed,
            None,
            &AtomicBool::new(false),
        )?;
        let (replayed, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
        assert_eq!(replayed, receipt);
        assert!(lease.is_none());
        let history = object_attempt::list(&runtime, &run)?;
        assert_eq!(history.len(), 2);
        assert!(history.contains(&attempt));
        assert_eq!(data_backup::inventory(&f.base.source)?, source_before);
        assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    }
    Ok(())
}

#[test]
fn project_derivation_execution_accepts_legitimate_object_history_drift_after_claim() -> Result<()>
{
    let f = ExecutionFixture::new(State::Failed, "latest")?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let mut child = f.base.child.clone();
    child.name = "Renamed after failure".into();
    let updated = objects::update(&runtime, &child, "post-stop-metadata")?.object;
    let captured = objects::capture(&runtime, &updated, "post-stop-capture")?;
    objects::accept(
        &runtime,
        &captured.object,
        captured.version_id.as_deref().unwrap(),
        "post-stop-accept",
    )?;
    drop(runtime);
    copy::prepare(f.base.request(), &f.base.temp.path().join("drift-prepared"))?;
    Ok(())
}
