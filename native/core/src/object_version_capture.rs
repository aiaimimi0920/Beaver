//! Freeze registered files outside SQLite locks, then compare-and-swap the catalog projection.
use crate::{
    files::{file_hash_limited, linked, safe_path},
    object_catalog::{self, ObjectRecord, ObjectVersion},
    object_command_receipt::{Command, CommandResult},
    object_registration,
    object_version_manifest::{VersionFile, VersionManifest, VersionStatus},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;

const MAX_CAPTURE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRequest {
    pub project_id: String,
    pub request_id: String,
    pub object_id: String,
    pub expected_revision: u64,
}

enum Start {
    Replay(CommandResult),
    Capture(ObjectRecord),
}

fn start(runtime: &ProjectRuntime, request: &CaptureRequest, command: &Command) -> Result<Start> {
    let store = runtime.store();
    let mut store = store
        .lock()
        .map_err(|_| anyhow::anyhow!("object store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = command.replay(connection)? {
            return Ok(Start::Replay(receipt));
        }
        ensure!(
            !crate::object_run_recovery::candidate::publication::blocked(
                connection,
                &request.project_id
            )?,
            "OBJECT_PUBLICATION_PENDING"
        );
        let object = object_registration::current(
            connection,
            &request.project_id,
            &request.object_id,
            request.expected_revision,
        )?;
        object_catalog::validate_write(connection, &object)?;
        Ok(Start::Capture(object))
    })
}

fn freeze(runtime: &ProjectRuntime, object: &ObjectRecord) -> Result<ObjectVersion> {
    let files = runtime.files();
    let mut remaining = MAX_CAPTURE_BYTES;
    let mut frozen = Vec::new();
    for file in &object.files {
        let source = safe_path(runtime.project_root(), &file.path)?;
        let metadata = fs::symlink_metadata(&source)?;
        ensure!(
            metadata.is_file() && !linked(&metadata),
            "OBJECT_CAPTURE_NOT_FILE: {}",
            file.path
        );
        ensure!(metadata.len() <= remaining, "OBJECT_CAPTURE_TOO_LARGE");
        let snapshot = files.capture_paths(
            runtime.project_root(),
            vec![file.path.clone()],
            Some(remaining),
        )?;
        let sha256 = snapshot
            .get(&file.path)
            .context("OBJECT_CAPTURE_FILE_MISSING")?
            .clone();
        let metadata = fs::symlink_metadata(files.blob(&sha256)?)?;
        ensure!(
            metadata.is_file() && !linked(&metadata),
            "OBJECT_CAPTURE_INVALID_BLOB"
        );
        let bytes = metadata.len();
        remaining = remaining
            .checked_sub(bytes)
            .context("OBJECT_CAPTURE_TOO_LARGE")?;
        frozen.push(VersionFile {
            path: file.path.clone(),
            role: file.role.clone(),
            bytes,
            sha256,
        });
    }
    // Detect edits to earlier files during a multi-file capture without retaining a Store lock.
    for file in &frozen {
        let path = safe_path(runtime.project_root(), &file.path)?;
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            metadata.is_file()
                && !linked(&metadata)
                && metadata.len() == file.bytes
                && file_hash_limited(&path, Some(file.bytes))?.as_ref() == Some(&file.sha256),
            "OBJECT_CAPTURE_SOURCE_CHANGED: {}",
            file.path
        );
    }
    let version_id = format!("version-{}", uuid::Uuid::new_v4());
    let manifest = VersionManifest {
        schema_version: 1,
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        version_id: version_id.clone(),
        status: VersionStatus::Captured,
        name: object.name.clone(),
        category: object.category.clone(),
        tags: object.tags.clone(),
        thumbnail_path: object.thumbnail_path.clone(),
        parent_object_id: object.parent_object_id.clone(),
        components: object.components.clone(),
        files: frozen,
        references: object.references.clone(),
    };
    Ok(ObjectVersion {
        version_id,
        manifest: serde_json::to_value(manifest)?,
    })
}

fn finish(
    runtime: &ProjectRuntime,
    request: &CaptureRequest,
    command: &Command,
    baseline: &ObjectRecord,
    version: ObjectVersion,
) -> Result<CommandResult> {
    let store = runtime.store();
    let mut store = store
        .lock()
        .map_err(|_| anyhow::anyhow!("object store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = command.replay(connection)? {
            return Ok(receipt);
        }
        let mut object = object_registration::current(
            connection,
            &request.project_id,
            &request.object_id,
            request.expected_revision,
        )?;
        ensure!(&object == baseline, "OBJECT_REVISION_CONFLICT");
        object_catalog::validate_write(connection, &object)?;
        connection.execute(
            "INSERT INTO entities(kind,id,value) VALUES('object_version',?,?)",
            params![
                version.version_id,
                serde_json::to_string(&(object.id.clone(), &version))?
            ],
        )?;
        let version_id = version.version_id.clone();
        object.versions.push(version);
        object.revision += 1;
        object_catalog::update_projection(connection, &object)?;
        command.save(connection, object, Some(version_id))
    })
}

pub fn capture(runtime: &ProjectRuntime, request: &CaptureRequest) -> Result<CommandResult> {
    let command = Command::new(
        runtime,
        &request.project_id,
        &request.request_id,
        "capture",
        request,
    )?;
    {
        let handle = runtime.store();
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("object store lock poisoned"))?;
        if let Some(result) = command.replay(&store.connection)? {
            return Ok(result);
        }
    }
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_PUBLICATION_BUSY"))?;
    match start(runtime, request, &command)? {
        Start::Replay(result) => Ok(result),
        Start::Capture(baseline) => {
            let version = freeze(runtime, &baseline)?;
            finish(runtime, request, &command, &baseline, version)
        }
    }
}

#[cfg(test)]
#[path = "object_version_capture_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "object_version_capture_concurrency_tests.rs"]
mod concurrency_tests;
