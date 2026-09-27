use super::{attempt_fixture::*, object_recovery_verification::request as verification};
use crate::{
    object_attempt::{self, State},
    object_attempt_checks::{self as checks, Request},
    object_attempt_view::Target,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{self as recovery, disposition, resume},
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

#[test]
fn retry_cannot_grandfather_oversized_output_and_checks_survive_disposal_and_reopen() -> Result<()>
{
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
    recovery::verify(&f.runtime, &verification(&f, "verify-retry")?, false)?;
    let input = resume::Request {
        rework: None,
        advance: None,
        project_id: "project-1".into(),
        request_id: "retry".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-retry".into(),
    };
    let (_, lease) = resume::execute(&f.runtime, &input, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    let id = lease.record().id.clone();
    assert!(lease.record().input.contains_key("hero.gd"));
    object_attempt::finish(
        &f.runtime,
        lease,
        State::Failed,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?
        .into_iter()
        .find(|value| value.id == id)
        .unwrap();
    assert_eq!(attempt.input, *attempt.output.as_ref().unwrap());
    let check = Request {
        project_id: "project-1".into(),
        request_id: "check".into(),
        target: Target::from_record(&attempt),
    };
    let report = checks::run(&f.runtime, &check)?;
    assert!(report.rules[0].passed);
    assert!(
        !report.rules[1].passed,
        "retry input must not become structural baseline"
    );
    recovery::verify(&f.runtime, &verification(&f, "verify-dispose")?, false)?;
    let dispose = disposition::Request {
        project_id: "project-1".into(),
        request_id: "dispose".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify-dispose".into(),
        choice: disposition::Choice::CancelAndRemoveWorkspace,
    };
    let workspace_root = workspace(&f, &attempt)?;
    let result = disposition::dispose(&f.runtime, &dispose, false)?;
    assert!(matches!(
        result.result,
        Some(disposition::Outcome::CancelledAndWorkspaceRemoved { .. })
    ));
    assert!(!workspace_root.exists());
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        checks::list(&runtime, "project-1", &id)?,
        vec![report.clone()]
    );
    assert_eq!(checks::run(&runtime, &check)?, report);
    let fresh = checks::run(
        &runtime,
        &Request {
            request_id: "after-reopen".into(),
            ..check
        },
    )?;
    assert!(fresh.rules[0].passed);
    assert!(!fresh.rules[1].passed);
    Ok(())
}
