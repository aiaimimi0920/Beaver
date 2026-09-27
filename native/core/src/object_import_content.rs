//! Verify only content-addressed source blobs while the source reader owns its lease.
use crate::{files, object_version_manifest::VersionManifest};
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) fn verify(root: &Path, versions: &[VersionManifest]) -> Result<()> {
    let mut verified = BTreeMap::new();
    for version in versions {
        for file in &version.files {
            let label = format!(
                "{}/{}: {}",
                version.object_id, version.version_id, file.path
            );
            ensure!(
                file.bytes <= 512 * 1024 * 1024,
                "IMPORT_FILE_TOO_LARGE: {label}"
            );
            if let Some(bytes) = verified.get(&file.sha256) {
                ensure!(
                    *bytes == file.bytes,
                    "IMPORT_CONTENT_SIZE_MISMATCH: {label}"
                );
                continue;
            }
            let path = files::safe_path(root, &format!(".beaver/content/blobs/{}", file.sha256))
                .with_context(|| format!("IMPORT_UNSAFE_CONTENT_PATH: {label}"))?;
            let metadata = fs::symlink_metadata(&path)
                .with_context(|| format!("IMPORT_CONTENT_MISSING: {label}"))?;
            ensure!(
                metadata.is_file() && !files::linked(&metadata),
                "IMPORT_UNSAFE_CONTENT_PATH: {label}"
            );
            ensure!(
                metadata.len() == file.bytes,
                "IMPORT_CONTENT_SIZE_MISMATCH: {label}"
            );
            ensure!(
                files::file_hash_limited(&path, Some(file.bytes))
                    .with_context(|| format!("IMPORT_CONTENT_READ_FAILED: {label}"))?
                    .as_deref()
                    == Some(file.sha256.as_str()),
                "IMPORT_CONTENT_HASH_MISMATCH: {label}"
            );
            verified.insert(file.sha256.clone(), file.bytes);
        }
    }
    Ok(())
}
