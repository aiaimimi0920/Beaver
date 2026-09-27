use super::attempt_fixture::{attempts, fixture, start, workspace};
use crate::{
    object_attempt::{self, State},
    object_attempt_checks::{self as checks, Request},
    object_attempt_view::Target,
    object_tasks,
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

#[test]
fn frozen_checks_ignore_workspace_and_preserve_ownership_and_replay() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    let path = workspace(&f, lease.record())?;
    std::fs::write(path.join("hero.gd"), "extends Node\n")?;
    let running = Request {
        project_id: "project-1".into(),
        request_id: "check-1".into(),
        target: Target::from_record(lease.record()),
    };
    assert!(checks::run(&f.runtime, &running)
        .unwrap_err()
        .to_string()
        .contains("OUTPUT_REQUIRED"));
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    let request = Request {
        target: Target::from_record(&attempt),
        ..running
    };
    let snapshot = serde_json::to_value(object_tasks::snapshot(&f.runtime, "project-1")?)?;
    let queue = serde_json::to_value(object_tasks::queue(&f.runtime, "project-1")?)?;
    std::fs::write(path.join("hero.gd"), "changed workspace".repeat(1000))?;
    let report = checks::run(&f.runtime, &request)?;
    assert!(report.passed, "{report:?}");
    assert_eq!(report.rules[1].files_checked, 1);
    let blob = f
        .runtime
        .files()
        .blob(&attempt.output.as_ref().unwrap()["hero.gd"])?;
    std::fs::write(&blob, "tampered")?;
    assert_eq!(checks::run(&f.runtime, &request)?, report);
    assert_eq!(
        checks::list(&f.runtime, "project-1", &attempt.id)?,
        vec![report]
    );
    let fresh = Request {
        request_id: "check-2".into(),
        ..request.clone()
    };
    let failed = checks::run(&f.runtime, &fresh)?;
    assert!(!failed.passed);
    assert!(failed.rules[0].issues[0].contains("HASH_MISMATCH"));
    std::fs::remove_file(&blob)?;
    let missing = checks::run(
        &f.runtime,
        &Request {
            request_id: "check-3".into(),
            ..request.clone()
        },
    )?;
    assert!(!missing.passed);
    assert!(missing.rules[0].issues[0].contains("CONTENT_MISSING"));
    let mut foreign = request.clone();
    foreign.project_id = "foreign".into();
    assert!(checks::run(&f.runtime, &foreign).is_err());
    let mut stale = request;
    stale.target.generation += 1;
    assert!(checks::run(&f.runtime, &stale).is_err());
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&f.runtime, "project-1")?)?,
        snapshot
    );
    assert_eq!(
        serde_json::to_value(object_tasks::queue(&f.runtime, "project-1")?)?,
        queue
    );
    assert_eq!(attempts(&f, "head")?, vec![attempt]);
    Ok(())
}

#[test]
fn oversized_output_fails_and_request_ids_cannot_be_rebound() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    std::fs::write(
        workspace(&f, lease.record())?.join("hero.gd"),
        "var value = 1\n".repeat(701),
    )?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::Failed,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    let request = Request {
        project_id: "project-1".into(),
        request_id: "check".into(),
        target: Target::from_record(&attempt),
    };
    let report = checks::run(&f.runtime, &request)?;
    assert!(report.rules[0].passed);
    assert!(!report.rules[1].passed);
    assert!(!report.rules[1].issues.is_empty());
    object_tasks::enqueue(&f.runtime, "project-1", &["independent".into()])?;
    let claim =
        crate::object_run_preparation::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    let other = object_attempt::start(&f.runtime, claim)?.unwrap();
    object_attempt::finish(
        &f.runtime,
        other,
        State::Failed,
        None,
        &AtomicBool::new(false),
    )?;
    let other = attempts(&f, "independent")?.remove(0);
    let conflict = Request {
        target: Target::from_record(&other),
        ..request
    };
    assert!(checks::run(&f.runtime, &conflict)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    Ok(())
}
