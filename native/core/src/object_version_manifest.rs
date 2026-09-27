//! Version manifests describe frozen blobs; only accepted versions are importable.
use crate::object_catalog::{ObjectComponent, ObjectRecord, ObjectReference, ObjectVersion};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionFile {
    pub path: String,
    pub role: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VersionStatus {
    Captured,
    ImportedPendingValidation,
    Accepted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub object_id: String,
    pub version_id: String,
    pub status: VersionStatus,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub parent_object_id: Option<String>,
    pub components: Vec<ObjectComponent>,
    pub files: Vec<VersionFile>,
    pub references: Vec<ObjectReference>,
}

fn default_category() -> String {
    "其他".to_owned()
}

pub(crate) fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn valid_asset_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 2_000
        && !path
            .chars()
            .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
        && path.split('/').all(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let device = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|suffix| {
                        matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                    })
                });
            !part.is_empty()
                && part != "."
                && part != ".."
                && part.trim() == part
                && !part.ends_with('.')
                && !device
                && ![".beaver", ".beaver-context", ".git", ".godot"]
                    .contains(&part.to_ascii_lowercase().as_str())
        })
}

pub fn read(record: &ObjectRecord, version: &ObjectVersion) -> Result<VersionManifest> {
    ensure!(
        version.manifest["schemaVersion"] == 1,
        "IMPORT_VERSION_UNSUPPORTED: {}/{}",
        record.id,
        version.version_id
    );
    ensure!(
        version.manifest["status"] == "accepted",
        "IMPORT_VERSION_NOT_ACCEPTED: {}/{}",
        record.id,
        version.version_id
    );
    read_version(record, version)
}

pub fn read_version(record: &ObjectRecord, version: &ObjectVersion) -> Result<VersionManifest> {
    let manifest = read_frozen_version(record, version)?;
    ensure!(
        manifest.category == record.category
            && manifest.tags == record.tags
            && manifest.thumbnail_path == record.thumbnail_path
            && manifest.parent_object_id == record.parent_object_id,
        "IMPORT_VERSION_METADATA_MISMATCH: {}/{}",
        record.id,
        version.version_id
    );
    Ok(manifest)
}

/// Historical viewing validates frozen metadata without requiring today's registration.
pub fn read_frozen_version(
    record: &ObjectRecord,
    version: &ObjectVersion,
) -> Result<VersionManifest> {
    let label = format!("{}/{}", record.id, version.version_id);
    ensure!(
        version.manifest["schemaVersion"] == 1,
        "IMPORT_VERSION_UNSUPPORTED: {label}"
    );
    let manifest: VersionManifest = serde_json::from_value(version.manifest.clone())
        .map_err(|error| anyhow::anyhow!("IMPORT_VERSION_UNSUPPORTED: {label}: {error}"))?;
    ensure!(
        manifest.project_id == record.project_id
            && manifest.object_id == record.id
            && manifest.version_id == version.version_id
            && valid_id(&manifest.project_id)
            && valid_id(&manifest.object_id)
            && valid_id(&manifest.version_id),
        "IMPORT_VERSION_IDENTITY_MISMATCH: {label}"
    );
    ensure!(
        !manifest.name.trim().is_empty(),
        "IMPORT_VERSION_INVALID_NAME: {label}"
    );
    crate::object_catalog::validate_metadata(
        &manifest.object_id,
        &manifest.category,
        &manifest.tags,
        manifest.thumbnail_path.as_deref(),
        manifest.parent_object_id.as_deref(),
    )?;
    let mut components = BTreeSet::new();
    for component in &manifest.components {
        ensure!(
            valid_id(&component.id)
                && components.insert(&component.id)
                && !component.kind.trim().is_empty()
                && !component.name.trim().is_empty(),
            "IMPORT_VERSION_INVALID_COMPONENT: {label}: {}",
            component.id
        );
    }
    let mut paths = BTreeSet::new();
    for file in &manifest.files {
        ensure!(
            valid_asset_path(&file.path),
            "IMPORT_PATH_TRAVERSAL: {label}: {}",
            file.path
        );
        ensure!(
            paths.insert(file.path.to_lowercase())
                && !file.role.trim().is_empty()
                && valid_digest(&file.sha256),
            "IMPORT_VERSION_INVALID_FILE: {label}: {}",
            file.path
        );
    }
    let mut references = BTreeSet::new();
    for reference in &manifest.references {
        ensure!(
            reference.project_id == manifest.project_id,
            "CROSS_PROJECT_OBJECT_REFERENCE: {label}: {}",
            reference.object_id
        );
        ensure!(
            valid_id(&reference.object_id)
                && reference.version_id.as_deref().is_some_and(valid_id)
                && references.insert((&reference.object_id, &reference.version_id)),
            "IMPORT_REFERENCE_NOT_PINNED: {label}: {}",
            reference.object_id
        );
    }
    Ok(manifest)
}
