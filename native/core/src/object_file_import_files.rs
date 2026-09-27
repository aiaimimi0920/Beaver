//! All source reads finish before the journal starts; retries only consume target blobs.
use super::{Operation, State};
use crate::{
    files::{file_hash_limited, linked, safe_path},
    object_catalog::ObjectRecord,
    object_file_import_preparation::Preparation,
    object_version_manifest::{self, VersionFile},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use std::{fs, path::PathBuf};

pub(super) fn freeze(runtime: &ProjectRuntime, receipt: &Preparation) -> Result<()> {
    let roots: Vec<PathBuf> = receipt
        .source
        .source
        .paths
        .iter()
        .map(PathBuf::from)
        .collect();
    let target = fs::canonicalize(runtime.project_root())?;
    for root in &roots {
        let source = fs::canonicalize(root)?;
        ensure!(
            !target.starts_with(&source) && !source.starts_with(&target),
            "IMPORT_SOURCE_TARGET_OVERLAP"
        );
    }
    ensure!(
        crate::object_import_file_source::inspect(&roots)? == receipt.source,
        "IMPORT_SOURCE_CHANGED"
    );
    let files = runtime.files();
    for file in &receipt.source.files {
        let path = PathBuf::from(&file.path);
        let parent = path.parent().context("IMPORT_SOURCE_PARENT_MISSING")?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("IMPORT_SOURCE_NAME_INVALID")?;
        let snapshot = files.capture_paths(parent, vec![name.into()], Some(file.bytes))?;
        ensure!(
            snapshot.get(name) == Some(&file.sha256),
            "IMPORT_SOURCE_CHANGED"
        );
        ensure!(
            fs::metadata(files.blob(&file.sha256)?)?.len() == file.bytes,
            "IMPORT_SOURCE_CHANGED"
        );
    }
    ensure!(
        crate::object_import_file_source::inspect(&roots)? == receipt.source,
        "IMPORT_SOURCE_CHANGED"
    );
    Ok(())
}

fn frozen(objects: &[ObjectRecord]) -> Result<Vec<VersionFile>> {
    let mut files = Vec::new();
    for object in objects {
        files.extend(object_version_manifest::read_version(object, &object.versions[0])?.files);
    }
    Ok(files)
}

fn current(runtime: &ProjectRuntime, file: &VersionFile) -> Result<Option<String>> {
    let path = safe_path(runtime.project_root(), &file.path)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !linked(&metadata),
                "IMPORT_TARGET_CONFLICT: {}",
                file.path
            );
            // Oversized external replacements are conflicts without hashing arbitrary content.
            if metadata.len() != file.bytes {
                return Ok(Some("different-size".into()));
            }
            file_hash_limited(&path, Some(file.bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn check_blob(runtime: &ProjectRuntime, file: &VersionFile) -> Result<PathBuf> {
    let blob = runtime.files().blob(&file.sha256)?;
    let metadata = fs::symlink_metadata(&blob)?;
    ensure!(
        metadata.is_file()
            && !linked(&metadata)
            && metadata.len() == file.bytes
            && file_hash_limited(&blob, Some(file.bytes))?.as_ref() == Some(&file.sha256),
        "IMPORT_BLOB_CORRUPT"
    );
    Ok(blob)
}

pub(super) fn apply(
    runtime: &ProjectRuntime,
    objects: &[ObjectRecord],
    operation: &mut Operation,
    checkpoint: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<()> {
    ensure!(operation.state == State::Applying, "IMPORT_NOT_APPLYING");
    let files = frozen(objects)?;
    for file in &files {
        check_blob(runtime, file)?;
        let hash = current(runtime, file)?;
        ensure!(
            hash.is_none()
                || (operation.writes.contains(&file.path) && hash.as_ref() == Some(&file.sha256)),
            "IMPORT_TARGET_CONFLICT: {}",
            file.path
        );
    }
    for file in &files {
        if operation.writes.contains(&file.path)
            && current(runtime, file)?.as_ref() == Some(&file.sha256)
        {
            continue;
        }
        ensure!(
            current(runtime, file)?.is_none(),
            "IMPORT_TARGET_CONFLICT: {}",
            file.path
        );
        if !operation.writes.contains(&file.path) {
            operation.writes.push(file.path.clone());
            super::save(runtime, operation)?;
        }
        checkpoint("intent")?;
        let blob = check_blob(runtime, file)?;
        let path = safe_path(runtime.project_root(), &file.path)?;
        let parent = path.parent().context("IMPORT_TARGET_PARENT_MISSING")?;
        fs::create_dir_all(parent)?;
        let path = safe_path(runtime.project_root(), &file.path)?;
        let temporary = tempfile::NamedTempFile::new_in(parent)?;
        fs::copy(blob, temporary.path())?;
        temporary.as_file().sync_all()?;
        ensure!(
            file_hash_limited(temporary.path(), Some(file.bytes))?.as_ref() == Some(&file.sha256),
            "IMPORT_BLOB_CORRUPT"
        );
        temporary
            .persist_noclobber(path)
            .map_err(|error| error.error)?;
        checkpoint("written")?;
    }
    verify(runtime, objects)
}

pub(super) fn verify(runtime: &ProjectRuntime, objects: &[ObjectRecord]) -> Result<()> {
    for file in frozen(objects)? {
        ensure!(
            current(runtime, &file)?.as_ref() == Some(&file.sha256),
            "IMPORT_TARGET_CONFLICT: {}",
            file.path
        );
    }
    Ok(())
}

pub(super) fn reverse(
    runtime: &ProjectRuntime,
    objects: &[ObjectRecord],
    operation: &Operation,
) -> Result<()> {
    let mut conflicts = Vec::new();
    for file in frozen(objects)?
        .iter()
        .rev()
        .filter(|file| operation.writes.contains(&file.path))
    {
        match current(runtime, file) {
            Ok(None) => (),
            Ok(Some(hash)) if hash == file.sha256 => {
                fs::remove_file(safe_path(runtime.project_root(), &file.path)?)?;
            }
            _ => conflicts.push(file.path.clone()),
        }
    }
    ensure!(
        conflicts.is_empty(),
        "IMPORT_ABORT_EXTERNAL_CHANGES: {}",
        conflicts.join(", ")
    );
    Ok(())
}
