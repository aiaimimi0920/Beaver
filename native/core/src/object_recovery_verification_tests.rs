use super::{attempt_fixture::*, object_task_dispatch::request as pause_request};
use crate::{
    object_attempt::{self, State},
    object_catalog_test_fixture::{update_request, Fixture},
    object_registration, object_run_preparation,
    object_run_recovery::{
        self as recovery, ContentStatus, VerifyRequest, WorkspaceStatus, WriterStatus,
    },
    object_task_dispatch, object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::{fs, sync::atomic::AtomicBool};

pub(super) fn frozen() -> Result<Fixture> {
    let f = fixture()?;
    let lease = start(&f)?;
    fs::write(workspace(&f, lease.record())?.join("partial.txt"), "keep")?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    Ok(f)
}

pub(super) fn request(f: &Fixture, id: &str) -> Result<VerifyRequest> {
    Ok(VerifyRequest {
        project_id: "project-1".into(),
        request_id: id.into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
    })
}

#[test]
fn frozen_report_survives_reopen_without_claiming_or_reexecuting() -> Result<()> {
    let f = frozen()?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let queue = object_tasks::queue(&f.runtime, "project-1")?;
    let input = request(&f, "verify")?;
    assert_eq!(f.count("object_recovery_verification")?, 0);
    let operation = recovery::verify(&f.runtime, &input, false)?;
    assert!(operation.result.as_ref().unwrap().can_dispose());
    let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(view.report_matches_records && view.can_dispose);
    assert_eq!(view.target.recovery_generation, 1);
    assert_eq!(view.operation, Some(operation.clone()));
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(recovery::get(&reopened, "project-1", "head")?, Some(view));
    assert_eq!(recovery::verify(&reopened, &input, false)?, operation);
    assert_eq!(object_tasks::snapshot(&reopened, "project-1")?, before);
    assert_eq!(object_tasks::queue(&reopened, "project-1")?, queue);
    assert!(object_run_preparation::claim_next(&reopened, "project-1", "restart")?.is_none());
    Ok(())
}

#[test]
fn pause_and_unpause_expire_eligibility_without_rewriting_the_report() -> Result<()> {
    let f = frozen()?;
    let input = request(&f, "verify")?;
    let operation = recovery::verify(&f.runtime, &input, false)?;
    for (id, paused) in [("pause", true), ("unpause", false)] {
        object_task_dispatch::set_paused(&f.runtime, &pause_request(&f, id, paused)?)?;
        let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
        assert_eq!(view.paused, paused);
        assert!(!view.report_matches_records && !view.can_dispose);
        assert_eq!(view.operation, Some(operation.clone()));
        assert_eq!(recovery::verify(&f.runtime, &input, false)?, operation);
    }
    let next = request(&f, "verify-again")?;
    let verified = recovery::verify(&f.runtime, &next, false)?;
    assert_eq!(verified.generation, 2);
    assert!(
        recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .can_dispose
    );
    assert_eq!(task_record(&f, "next")?.status, "planned");
    Ok(())
}

#[test]
fn object_metadata_changes_require_new_verification() -> Result<()> {
    let f = frozen()?;
    let input = request(&f, "verify")?;
    let operation = recovery::verify(&f.runtime, &input, false)?;
    let mut update = update_request(&f.object("hero")?, "rename");
    update.name = "Renamed".into();
    object_registration::update(&f.runtime, &update)?;
    let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(!view.report_matches_records && !view.can_dispose);
    assert_eq!(view.operation, Some(operation));
    let mut stale = input;
    stale.request_id = "stale".into();
    assert_eq!(
        recovery::verify(&f.runtime, &stale, false)
            .unwrap_err()
            .to_string(),
        "OBJECT_RECOVERY_STALE_TARGET"
    );
    assert_eq!(f.count("object_recovery_verification")?, 1);
    recovery::verify(&f.runtime, &request(&f, "current")?, false)?;
    assert!(
        recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .can_dispose
    );
    Ok(())
}

#[test]
fn file_changes_are_found_by_explicit_verification_and_old_receipts_stay_frozen() -> Result<()> {
    for damage in ["drift", "missing", "blob"] {
        let f = frozen()?;
        let input = request(&f, "verify")?;
        let original = recovery::verify(&f.runtime, &input, false)?;
        let attempt = attempts(&f, "head")?.remove(0);
        let root = workspace(&f, &attempt)?;
        match damage {
            "drift" => fs::write(root.join("partial.txt"), "changed")?,
            "missing" => fs::rename(&root, root.with_extension("retained"))?,
            _ => {
                let hash = attempt.output.as_ref().unwrap().get("partial.txt").unwrap();
                fs::write(f.runtime.files().blob(hash)?, "bad content")?;
            }
        }
        // The query and exact replay deliberately do not scan or mutate files.
        assert!(
            recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .report_matches_records
        );
        assert_eq!(recovery::verify(&f.runtime, &input, false)?, original);
        let next = recovery::verify(&f.runtime, &request(&f, "check-again")?, false)?;
        let report = next.result.unwrap();
        assert!(!report.can_dispose());
        match damage {
            "drift" => assert_eq!(report.workspace_status, WorkspaceStatus::Drifted),
            "missing" => assert_eq!(report.workspace_status, WorkspaceStatus::Missing),
            _ => {
                assert_eq!(report.content_status, ContentStatus::Invalid);
                assert!(report
                    .issues
                    .iter()
                    .any(|issue| issue.contains("CONTENT_HASH_MISMATCH")));
            }
        }
        assert_eq!(task_record(&f, "head")?.status, "awaitingAcceptance");
    }
    Ok(())
}

#[test]
fn live_and_orphaned_writers_never_prove_stop() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    fs::write(workspace(&f, lease.record())?.join("unfrozen.txt"), "keep")?;
    let active = recovery::verify(&f.runtime, &request(&f, "active")?, true)?
        .result
        .unwrap();
    assert_eq!(active.writer_status, WriterStatus::Active);
    assert_eq!(active.content_status, ContentStatus::Unchecked);
    assert_eq!(active.workspace_status, WorkspaceStatus::Unchecked);
    let orphaned = recovery::verify(&f.runtime, &request(&f, "orphaned")?, false)?
        .result
        .unwrap();
    assert_eq!(orphaned.writer_status, WriterStatus::Unconfirmed);
    assert_eq!(orphaned.workspace_status, WorkspaceStatus::Drifted);
    assert!(!orphaned.can_dispose());
    assert_eq!(attempts(&f, "head")?.len(), 1);
    assert_eq!(task_record(&f, "head")?.status, "running");
    Ok(())
}
