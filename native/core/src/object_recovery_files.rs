//! Read-only evidence collection. A matching checkpoint never grants execution rights.
use super::{records::Records, ContentStatus, Report, WorkspaceStatus, WriterStatus};
use crate::{
    files::{self, Snapshot},
    object_import_content, object_import_snapshot, object_run_baseline,
    object_version_manifest::{valid_asset_path, valid_digest},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeSet, fs, io::ErrorKind};

const FILE_LIMIT: u64 = 512 * 1024 * 1024;

pub(super) fn verify(runtime: &ProjectRuntime, records: &Records, live: bool) -> Report {
    let mut report = Report {
        records_current: true,
        writer_status: if live {
            WriterStatus::Active
        } else if records
            .attempt
            .as_ref()
            .is_some_and(|attempt| attempt.output.is_some())
        {
            WriterStatus::StopRecorded
        } else {
            WriterStatus::Unconfirmed
        },
        content_status: ContentStatus::Unchecked,
        workspace_status: WorkspaceStatus::Unchecked,
        paused: records.control.paused,
        issues: vec![],
    };
    if live {
        report.issues.push("OBJECT_RECOVERY_WRITER_ACTIVE".into());
        return report;
    }
    if report.writer_status == WriterStatus::Unconfirmed {
        report
            .issues
            .push("OBJECT_RECOVERY_STOP_UNCONFIRMED".into());
    }
    let checkpoint = match content(runtime, records) {
        Ok(snapshot) => {
            report.content_status = ContentStatus::Verified;
            snapshot
        }
        Err(error) => {
            report.content_status = ContentStatus::Invalid;
            report.issues.push(format!("{error:#}"));
            return report;
        }
    };
    match workspace(runtime, records, &checkpoint) {
        Ok(status) => report.workspace_status = status,
        Err(error) => {
            report.workspace_status = WorkspaceStatus::Invalid;
            report.issues.push(format!("{error:#}"));
        }
    }
    report
}

fn valid_snapshot(snapshot: &Snapshot) -> Result<()> {
    let mut paths = BTreeSet::new();
    ensure!(
        snapshot.iter().all(|(path, hash)| valid_asset_path(path)
            && valid_digest(hash)
            && paths.insert(path.to_lowercase())),
        "OBJECT_RECOVERY_INVALID_CHECKPOINT"
    );
    Ok(())
}

fn content(runtime: &ProjectRuntime, records: &Records) -> Result<Snapshot> {
    let baseline = records
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_RUN_BASELINE_MISSING")?;
    let crate::object_framework::Identity::Medium {
        baseline: policy, ..
    } = &records.preparation.medium.identity
    else {
        anyhow::bail!("OBJECT_RECOVERY_BASELINE_POLICY_MISMATCH");
    };
    ensure!(
        policy == &baseline.policy
            && records.preparation.run.baseline_version_id == baseline.resolved_version_id,
        "OBJECT_RECOVERY_BASELINE_POLICY_MISMATCH"
    );
    if let Some(version) = &baseline.resolved_version_id {
        let selected = object_import_snapshot::select(
            &records.baseline_sources,
            runtime.project_id(),
            &records.object.id,
            version,
        )?;
        ensure!(
            selected.versions == baseline.versions,
            "OBJECT_RECOVERY_BASELINE_CHANGED"
        );
    } else {
        ensure!(
            matches!(policy, crate::object_framework::Baseline::Empty {})
                && baseline.versions.is_empty(),
            "OBJECT_RECOVERY_BASELINE_POLICY_MISMATCH"
        );
    }
    ensure!(
        object_import_snapshot::digest(&baseline.versions)? == baseline.content_digest,
        "OBJECT_RECOVERY_BASELINE_DIGEST_MISMATCH"
    );
    let mut input = object_run_baseline::snapshot(&baseline.versions)?;
    valid_snapshot(&input)?;
    object_import_content::verify(runtime.project_root(), &baseline.versions)?;
    for attempt in records.history.iter().chain(records.attempt.iter()) {
        ensure!(attempt.input == input, "OBJECT_RECOVERY_INPUT_MISMATCH");
        let checkpoint = attempt.output.as_ref().unwrap_or(&attempt.input);
        verify_checkpoint(runtime, checkpoint)?;
        input = checkpoint.clone();
    }
    Ok(input)
}

fn verify_checkpoint(runtime: &ProjectRuntime, checkpoint: &Snapshot) -> Result<()> {
    valid_snapshot(checkpoint)?;
    for hash in checkpoint.values().collect::<BTreeSet<_>>() {
        let path = runtime.files().blob(hash)?;
        let metadata = fs::symlink_metadata(&path).context("OBJECT_RECOVERY_CONTENT_MISSING")?;
        ensure!(
            metadata.is_file() && !files::linked(&metadata),
            "OBJECT_RECOVERY_INVALID_CONTENT_PATH"
        );
        ensure!(
            files::file_hash_limited(&path, Some(FILE_LIMIT))?.as_deref() == Some(hash),
            "OBJECT_RECOVERY_CONTENT_HASH_MISMATCH"
        );
    }
    Ok(())
}

fn workspace(
    runtime: &ProjectRuntime,
    records: &Records,
    checkpoint: &Snapshot,
) -> Result<WorkspaceStatus> {
    let prepared = &records.preparation;
    ensure!(
        runtime.files().workspace_location(&prepared.run.id)? == prepared.workspace,
        "OBJECT_RUN_WORKSPACE_MISMATCH"
    );
    let root = runtime.files().workspace(&prepared.run.id)?;
    let metadata = match fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(WorkspaceStatus::Missing),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.is_dir() && !files::linked(&metadata),
        "OBJECT_RECOVERY_INVALID_WORKSPACE"
    );
    let paths = files::list_files(&root)?;
    if paths.len() != checkpoint.len() {
        return Ok(WorkspaceStatus::Drifted);
    }
    for path in paths {
        let Some(expected) = checkpoint.get(&path) else {
            return Ok(WorkspaceStatus::Drifted);
        };
        let file = files::safe_path(&root, &path)?;
        if files::file_hash_limited(&file, Some(FILE_LIMIT))?.as_ref() != Some(expected) {
            return Ok(WorkspaceStatus::Drifted);
        }
    }
    Ok(WorkspaceStatus::MatchesCheckpoint)
}
