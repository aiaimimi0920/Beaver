use super::{
    attempt_fixture::*,
    object_recovery_verification::{frozen, request},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{
        self as recovery,
        disposition::{self, Choice, Outcome, Request},
    },
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::fs;

fn input(f: &Fixture, choice: Choice) -> Result<Request> {
    recovery::verify(&f.runtime, &request(f, "verify")?, false)?;
    Ok(Request {
        project_id: "project-1".into(),
        request_id: "dispose".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: "verify".into(),
        choice,
    })
}

#[test]
fn removal_preserves_home_blobs_other_workspaces_and_history() -> Result<()> {
    let f = frozen()?;
    let input = input(&f, Choice::CancelAndRemoveWorkspace)?;
    let attempt = attempts(&f, "head")?.remove(0);
    let root = workspace(&f, &attempt)?;
    let home = f.runtime.files().codex_home(&input.target.run_id)?;
    fs::create_dir_all(&home)?;
    fs::write(home.join("sentinel"), "home")?;
    let other = f.runtime.files().workspace("other-run")?;
    fs::create_dir_all(&other)?;
    fs::write(other.join("sentinel"), "other")?;
    let hash = attempt.output.as_ref().unwrap().get("partial.txt").unwrap();
    let blob = f.runtime.files().blob(hash)?;
    let result = disposition::dispose(&f.runtime, &input, false)?;
    assert!(matches!(
        result.result,
        Some(Outcome::CancelledAndWorkspaceRemoved { .. })
    ));
    assert!(!root.exists());
    assert_eq!(fs::read_to_string(home.join("sentinel"))?, "home");
    assert_eq!(fs::read_to_string(other.join("sentinel"))?, "other");
    assert_eq!(fs::read_to_string(blob)?, "keep");
    assert_eq!(task_record(&f, "head")?.status, "cancelled");
    assert_eq!(attempts(&f, "head")?.len(), 1);
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        recovery::get(&runtime, "project-1", "head")?
            .unwrap()
            .disposition,
        Some(result.clone())
    );
    fs::create_dir_all(&root)?;
    fs::write(root.join("new"), "do not delete")?;
    let history = crate::object_attempt_view::list_with_details(&runtime, &input.target.run_id)?;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].2.input, attempt.input);
    assert_eq!(history[0].2.output, attempt.output);
    assert!(!history[0].2.output.as_ref().unwrap().contains_key("new"));
    assert_eq!(disposition::dispose(&runtime, &input, false)?, result);
    assert!(root.join("new").exists());
    Ok(())
}

#[test]
fn deletion_reopens_after_files_before_metadata_commit() -> Result<()> {
    let f = frozen()?;
    let input = input(&f, Choice::CancelAndRemoveWorkspace)?;
    let root = workspace(&f, &attempts(&f, "head")?.remove(0))?;
    let error = disposition::dispose_with(
        &f.runtime,
        &input,
        false,
        || Ok(()),
        || anyhow::bail!("host stopped"),
    );
    assert_eq!(error.unwrap_err().to_string(), "host stopped");
    assert!(!root.exists());
    assert_eq!(task_record(&f, "head")?.status, "awaitingAcceptance");
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let result = disposition::dispose(&runtime, &input, false)?;
    assert!(matches!(
        result.result,
        Some(Outcome::CancelledAndWorkspaceRemoved { .. })
    ));
    assert_eq!(disposition::dispose(&runtime, &input, false)?, result);
    Ok(())
}

#[test]
fn live_drift_and_stale_records_prevent_removal() -> Result<()> {
    for reason in ["live", "drift", "stale"] {
        let f = frozen()?;
        let input = input(&f, Choice::CancelAndRemoveWorkspace)?;
        let root = workspace(&f, &attempts(&f, "head")?.remove(0))?;
        let result = disposition::dispose_with(
            &f.runtime,
            &input,
            reason == "live",
            || {
                if reason == "drift" {
                    fs::write(root.join("partial.txt"), "new edits")?;
                }
                if reason == "stale" {
                    mutate(
                        &f,
                        "object_task",
                        "head",
                        "/revision",
                        serde_json::json!(999),
                    )?;
                }
                Ok(())
            },
            || Ok(()),
        )?;
        assert!(matches!(result.result, Some(Outcome::Blocked { .. })));
        assert!(root.join("partial.txt").exists());
    }
    Ok(())
}

#[test]
fn keep_cancels_without_removing_workspace_and_replays() -> Result<()> {
    let f = frozen()?;
    let input = input(&f, Choice::CancelAndKeep)?;
    let root = workspace(&f, &attempts(&f, "head")?.remove(0))?;
    let result = disposition::dispose(&f.runtime, &input, false)?;
    assert!(matches!(
        result.result,
        Some(Outcome::CancelledAndRetained { .. })
    ));
    assert_eq!(fs::read_to_string(root.join("partial.txt"))?, "keep");
    assert_eq!(disposition::dispose(&f.runtime, &input, false)?, result);
    Ok(())
}

#[test]
fn completed_deletion_never_removes_a_recreated_matching_workspace() -> Result<()> {
    let f = frozen()?;
    let input = input(&f, Choice::CancelAndRemoveWorkspace)?;
    let root = workspace(&f, &attempts(&f, "head")?.remove(0))?;
    assert!(disposition::dispose_with(
        &f.runtime,
        &input,
        false,
        || Ok(()),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    fs::create_dir_all(&root)?;
    fs::write(root.join("partial.txt"), "keep")?;
    assert_eq!(
        disposition::dispose(&f.runtime, &input, false)
            .unwrap_err()
            .to_string(),
        "OBJECT_RECOVERY_WORKSPACE_RECREATED"
    );
    assert_eq!(fs::read_to_string(root.join("partial.txt"))?, "keep");
    Ok(())
}

#[test]
fn running_journal_with_absent_workspace_completes_after_reopen() -> Result<()> {
    use sha2::{Digest, Sha256};
    let f = frozen()?;
    let input = input(&f, Choice::CancelAndRemoveWorkspace)?;
    assert!(disposition::dispose_with(
        &f.runtime,
        &input,
        false,
        || Ok(()),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    // Model a crash after deletion but before the completed journal write.
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("project-1", "dispose"))?)
    );
    mutate(
        &f,
        "object_recovery_disposition",
        &key,
        "/resource/state",
        serde_json::json!("running"),
    )?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert!(matches!(
        disposition::dispose(&runtime, &input, false)?.result,
        Some(Outcome::CancelledAndWorkspaceRemoved { .. })
    ));
    Ok(())
}
