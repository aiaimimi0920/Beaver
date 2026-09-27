use super::{
    attempt_fixture::*,
    object_recovery_verification::{frozen, request},
    object_task_dispatch::request as pause_request,
};
use crate::{
    object_catalog_test_fixture::Fixture, object_run_recovery as recovery, object_task_dispatch,
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn pending_verification_is_retried_after_reopen_with_its_frozen_request() -> Result<()> {
    let f = frozen()?;
    let input = request(&f, "verify")?;
    let error = recovery::verify_with(&f.runtime, &input, false, || anyhow::bail!("host stopped"));
    assert_eq!(error.unwrap_err().to_string(), "host stopped");
    let pending = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(!pending.report_matches_records && !pending.can_dispose);
    assert_eq!(pending.operation.as_ref().unwrap().request, input);
    assert!(pending.operation.unwrap().result.is_none());
    assert_eq!(
        recovery::verify(&f.runtime, &request(&f, "competing")?, false)
            .unwrap_err()
            .to_string(),
        "OBJECT_RECOVERY_PENDING"
    );
    let mut conflicting = input.clone();
    conflicting.target.control_revision += 1;
    assert_eq!(
        recovery::verify(&f.runtime, &conflicting, false)
            .unwrap_err()
            .to_string(),
        "OBJECT_RECOVERY_REQUEST_CONFLICT"
    );
    object_task_dispatch::set_paused(&f.runtime, &pause_request(&f, "pause", true)?)?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let operation = recovery::verify(&runtime, &input, false)?;
    let report = operation.result.as_ref().unwrap();
    assert!(!report.records_current && !report.paused && !report.can_dispose());
    assert!(report
        .issues
        .iter()
        .any(|issue| issue == "OBJECT_RECOVERY_RECORDS_CHANGED"));
    assert_eq!(recovery::verify(&runtime, &input, false)?, operation);
    let view = recovery::get(&runtime, "project-1", "head")?.unwrap();
    assert!(!view.report_matches_records && !view.can_dispose && view.paused);
    let next = recovery::VerifyRequest {
        request_id: "next".into(),
        target: view.target,
        ..input
    };
    let report = recovery::verify(&runtime, &next, false)?.result.unwrap();
    assert!(report.records_current && report.paused);
    assert!(!report.can_dispose());
    Ok(())
}

#[test]
fn file_phase_releases_store_lock_but_keeps_one_verifier_across_runtime_clones() -> Result<()> {
    let f = frozen()?;
    let input = request(&f, "verify")?;
    let runtime = f.runtime.clone();
    let operation = recovery::verify_with(&f.runtime, &input, false, || {
        let handle = runtime.store();
        drop(
            handle
                .try_lock()
                .expect("verifier file work must not hold the Store lock"),
        );
        assert_eq!(
            recovery::verify(&runtime, &input, false)
                .unwrap_err()
                .to_string(),
            "OBJECT_RECOVERY_BUSY"
        );
        object_task_dispatch::set_paused(&runtime, &pause_request(&f, "pause", true)?)?;
        Ok(())
    })?;
    assert!(!operation.result.unwrap().records_current);
    assert_eq!(f.count("object_recovery_verification")?, 1);
    Ok(())
}

#[test]
fn damaged_records_during_verification_do_not_strand_the_pending_receipt() -> Result<()> {
    let f = frozen()?;
    let input = request(&f, "verify")?;
    let operation = recovery::verify_with(&f.runtime, &input, false, || {
        mutate(
            &f,
            "object_task",
            "head",
            "/revision",
            serde_json::json!(999),
        )
    })?;
    let report = operation.result.as_ref().unwrap();
    assert!(!report.records_current);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.starts_with("OBJECT_RECOVERY_RECORDS_INVALID:")));
    assert!(recovery::get(&f.runtime, "project-1", "head").is_err());
    assert_eq!(recovery::verify(&f.runtime, &input, false)?, operation);
    Ok(())
}
