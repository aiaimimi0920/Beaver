use crate::{
    asset_task,
    files::{file_hash_limited, safe_path, Files, Snapshot},
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub task_id: String,
    pub stage_id: String,
    pub template_id: String,
    pub template_version: u32,
    pub input_candidates: Vec<String>,
    #[serde(default)]
    pub attempt_ids: Vec<String>,
    pub files: Snapshot,
    pub summary: String,
    pub thread_id: String,
    pub turn_id: String,
    pub session_id: Option<String>,
    pub created_at: String,
}

pub fn kind(id: &str) -> String {
    format!("asset-delivery/{id}")
}

pub fn get(store: &Store, task: &str, candidate: &str) -> Result<Candidate> {
    asset_task::validate_id(candidate)?;
    store
        .get(&kind(task), candidate)?
        .context("Candidate does not belong to this task")
}

pub fn capture(files: &Files, workspace: &Path, paths: &[String]) -> Result<Snapshot> {
    ensure!(
        (1..=32).contains(&paths.len()),
        "Submit 1 to 32 actual files"
    );
    let mut seen = std::collections::HashSet::new();
    let mut total = 0_u64;
    for path in paths {
        ensure!(
            path.len() <= 2000 && seen.insert(path),
            "Invalid or duplicate file path"
        );
        ensure!(
            !path.split('/').any(|p| matches!(
                p.to_ascii_lowercase().as_str(),
                ".git"
                    | ".godot"
                    | ".beaver"
                    | ".beaver-context"
                    | "node_modules"
                    | "target"
                    | "release"
                    | "exports"
            ) || p.to_ascii_lowercase().starts_with(".beaver-write-")),
            "Internal workspace files cannot be submitted"
        );
        let metadata = fs::metadata(safe_path(workspace, path)?)?;
        ensure!(
            metadata.is_file() && metadata.len() > 0 && metadata.len() <= MAX_FILE_BYTES,
            "Candidate files must be nonempty regular files of at most 256 MiB"
        );
        total += metadata.len();
    }
    ensure!(total <= MAX_TOTAL_BYTES, "Candidate exceeds 512 MiB");
    let snapshot = files.capture_paths(workspace, paths.to_vec(), Some(MAX_FILE_BYTES))?;
    verify(files, &snapshot)?;
    Ok(snapshot)
}

pub fn verify(files: &Files, snapshot: &Snapshot) -> Result<()> {
    let mut total = 0;
    for hash in snapshot.values() {
        let path = files.blob(hash)?;
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !crate::files::linked(&metadata)
                && metadata.is_file()
                && metadata.len() > 0
                && metadata.len() <= MAX_FILE_BYTES,
            "Frozen candidate file is missing, linked or invalid"
        );
        ensure!(
            file_hash_limited(&path, Some(MAX_FILE_BYTES))?.as_ref() == Some(hash),
            "Frozen candidate hash mismatch"
        );
        total += metadata.len();
    }
    ensure!(
        !snapshot.is_empty() && total <= MAX_TOTAL_BYTES,
        "Invalid candidate size"
    );
    Ok(())
}

pub fn read(files: &Files, candidate: &Candidate, path: &str) -> Result<Vec<u8>> {
    let hash = candidate
        .files
        .get(path)
        .context("File is not in this candidate")?;
    let blob = files.blob(hash)?;
    let metadata = fs::symlink_metadata(&blob)?;
    ensure!(
        !crate::files::linked(&metadata)
            && metadata.is_file()
            && metadata.len() <= 16 * 1024 * 1024,
        "Inline viewing is limited to 16 MiB; use candidate export for larger files"
    );
    let mut bytes = Vec::new();
    fs::File::open(blob)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "Inline file grew beyond 16 MiB"
    );
    use sha2::{Digest, Sha256};
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == *hash,
        "Frozen candidate hash mismatch"
    );
    Ok(bytes)
}

pub fn verify_final(
    store: &Store,
    files: &Files,
    task: &serde_json::Value,
    changes: &[crate::files::Change],
) -> Result<()> {
    let id = task["id"].as_str().context("Missing task ID")?;
    let Some(state) = store.get::<asset_task::State>("asset-task", id)? else {
        return Ok(());
    };
    let Some(flow) = &state.delivery else {
        return Ok(());
    };
    ensure!(
        crate::asset_delivery::complete(&state),
        "Delivery approvals are incomplete"
    );
    let expected = approved_files(store, id, &flow.approved)?;
    for change in changes {
        ensure!(change.after.is_some() && expected.get(&change.path) == change.after.as_ref(), "Unapproved output in final changes: {}; submit it before final delivery (deletions are not supported)", change.path);
    }
    let workspace = files.resolve_workspace(
        id,
        Path::new(task["workspace"].as_str().context("Missing workspace")?),
    )?;
    crate::framework_operations::idle(store, id)?;
    crate::framework_inputs::verify_closure(store, files, &workspace, &state)?;
    verify_workspace(files, &workspace, &expected)
}

pub fn approved_files(store: &Store, id: &str, approved: &[String]) -> Result<Snapshot> {
    let mut expected = Snapshot::new();
    for candidate in approved {
        expected.extend(get(store, id, candidate)?.files);
    }
    Ok(expected)
}

pub fn verify_workspace(files: &Files, workspace: &Path, expected: &Snapshot) -> Result<()> {
    // The union may span many stages; per-file limits still apply.
    for (path, hash) in expected {
        verify(files, &Snapshot::from([(path.clone(), hash.clone())]))?;
        ensure!(
            file_hash_limited(&safe_path(workspace, path)?, Some(MAX_FILE_BYTES))?.as_ref()
                == Some(hash),
            "Approved output changed: {path}; restore frozen inputs or submit a new candidate"
        );
    }
    Ok(())
}

pub fn export(files: &Files, candidate: &Candidate) -> Result<std::path::PathBuf> {
    export_snapshot(files, &candidate.files)
}

pub fn export_snapshot(files: &Files, snapshot: &Snapshot) -> Result<std::path::PathBuf> {
    // Bound the union as well as each retained file.
    verify(files, snapshot)?;
    let root = files.delivery_exports()?;
    fs::create_dir_all(&root)?;
    let directory = tempfile::Builder::new()
        .prefix("candidate-")
        .tempdir_in(root)?;
    for (path, hash) in snapshot {
        let target = safe_path(directory.path(), path)?;
        fs::create_dir_all(target.parent().context("Invalid export path")?)?;
        let copied = io::copy(
            &mut fs::File::open(files.blob(hash)?)?.take(MAX_FILE_BYTES + 1),
            &mut fs::File::create(&target)?,
        )?;
        ensure!(
            copied <= MAX_FILE_BYTES,
            "Export file grew beyond size limit"
        );
        ensure!(
            file_hash_limited(&target, Some(MAX_FILE_BYTES))?.as_ref() == Some(hash),
            "Export hash mismatch"
        );
    }
    Ok(directory.keep())
}
