//! Recoverable materialization of exactly the frozen object's owned paths.
use super::{Preview, Source, Stored};
use crate::{
    files::{file_hash, file_hash_limited, linked, safe_path},
    object_version_manifest::{VersionFile, VersionManifest, VersionStatus},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use std::fs;

const MAX_BYTES: u64 = 512 * 1024 * 1024;

fn blob_bytes(runtime: &ProjectRuntime, hash: &str, remaining: u64) -> Result<u64> {
    let path = runtime.files().blob(hash)?;
    let metadata = fs::symlink_metadata(&path)?;
    ensure!(
        metadata.is_file() && !linked(&metadata) && metadata.len() <= remaining,
        "OBJECT_PUBLICATION_INVALID_BLOB"
    );
    ensure!(
        file_hash_limited(&path, Some(remaining))?.as_deref() == Some(hash),
        "OBJECT_PUBLICATION_BLOB_CHANGED"
    );
    Ok(metadata.len())
}

pub(super) fn freeze(
    runtime: &ProjectRuntime,
    source: &Source,
    preview: &Preview,
) -> Result<VersionManifest> {
    let object = &source.records.object;
    let mut remaining = MAX_BYTES;
    let mut files = Vec::new();
    for file in &preview.files {
        let hash = preview
            .paths
            .iter()
            .find(|c| c.path == file.path)
            .and_then(|c| c.after.as_ref())
            .context("OBJECT_PUBLICATION_FILE_MISSING")?;
        let bytes = blob_bytes(runtime, hash, remaining)?;
        remaining -= bytes;
        files.push(VersionFile {
            path: file.path.clone(),
            role: file.role.clone(),
            bytes,
            sha256: hash.clone(),
        });
    }
    for change in &preview.paths {
        ensure!(
            file_hash(&safe_path(runtime.project_root(), &change.path)?)? == change.before,
            "OBJECT_PUBLICATION_FILE_DRIFT: {}",
            change.path
        );
        if let Some(hash) = &change.before {
            blob_bytes(runtime, hash, MAX_BYTES)?;
        }
    }
    Ok(VersionManifest {
        schema_version: 1,
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        version_id: format!("version-{}", uuid::Uuid::new_v4()),
        status: VersionStatus::Accepted,
        name: object.name.clone(),
        category: object.category.clone(),
        tags: object.tags.clone(),
        thumbnail_path: object.thumbnail_path.clone(),
        parent_object_id: object.parent_object_id.clone(),
        components: object.components.clone(),
        files,
        references: object.references.clone(),
    })
}

fn current(runtime: &ProjectRuntime, path: &str) -> Result<Option<String>> {
    file_hash(&safe_path(runtime.project_root(), path)?)
}

pub(super) fn apply(
    runtime: &ProjectRuntime,
    saved: &mut Stored,
    checkpoint: &impl Fn(&str) -> Result<()>,
) -> Result<()> {
    for file in &saved.manifest.files {
        ensure!(
            blob_bytes(runtime, &file.sha256, file.bytes)? == file.bytes,
            "OBJECT_PUBLICATION_INVALID_BLOB"
        );
    }
    // Check every path and blob before any write; an interrupted intent may be at either endpoint.
    for change in &saved.operation.preview.paths {
        let actual = current(runtime, &change.path)?;
        ensure!(
            actual == change.before
                || (saved.writes.contains(&change.path) && actual == change.after),
            "OBJECT_PUBLICATION_FILE_DRIFT: {}",
            change.path
        );
        if let Some(hash) = &change.after {
            blob_bytes(runtime, hash, MAX_BYTES)?;
        }
        if let Some(hash) = &change.before {
            blob_bytes(runtime, hash, MAX_BYTES)?;
        }
    }
    for change in saved.operation.preview.paths.clone() {
        if change.before == change.after {
            continue;
        }
        let actual = current(runtime, &change.path)?;
        if saved.writes.contains(&change.path) && actual == change.after {
            continue;
        }
        ensure!(
            actual == change.before,
            "OBJECT_PUBLICATION_FILE_DRIFT: {}",
            change.path
        );
        if !saved.writes.contains(&change.path) {
            saved.writes.push(change.path.clone());
            super::transact(runtime, |db| super::storage::save(db, saved))?;
        }
        ensure!(
            current(runtime, &change.path)? == change.before,
            "OBJECT_PUBLICATION_FILE_DRIFT: {}",
            change.path
        );
        runtime.files().write(
            runtime.project_root(),
            &change.path,
            change.after.as_deref(),
        )?;
        checkpoint(&format!("applied:{}", change.path))?;
    }
    Ok(())
}

pub(super) fn verify_applied(runtime: &ProjectRuntime, saved: &Stored) -> Result<()> {
    for change in &saved.operation.preview.paths {
        ensure!(
            current(runtime, &change.path)? == change.after,
            "OBJECT_PUBLICATION_FILE_DRIFT: {}",
            change.path
        );
    }
    Ok(())
}

pub(super) fn reverse(runtime: &ProjectRuntime, saved: &Stored) -> Result<Vec<String>> {
    let mut conflicts = Vec::new();
    for path in saved.writes.iter().rev() {
        let change = saved
            .operation
            .preview
            .paths
            .iter()
            .find(|c| &c.path == path)
            .context("OBJECT_PUBLICATION_RECEIPT_MISMATCH")?;
        let actual = current(runtime, path)?;
        if actual == change.before {
            continue;
        }
        if actual != change.after {
            conflicts.push(path.clone());
            continue;
        }
        if let Some(hash) = &change.before {
            blob_bytes(runtime, hash, MAX_BYTES)?;
        }
        ensure!(
            current(runtime, path)? == change.after,
            "OBJECT_PUBLICATION_FILE_DRIFT: {path}"
        );
        runtime
            .files()
            .write(runtime.project_root(), path, change.before.as_deref())?;
    }
    Ok(conflicts)
}
