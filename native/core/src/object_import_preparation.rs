//! Read-only preparation of an object import.
//!
//! Preparation records the exact source closure and hashes, but deliberately
//! does not mutate the target object catalog. F7 owns the eventual commit.
use crate::{
    files::file_hash,
    object_catalog::{ObjectRecord, ObjectReference},
    object_external_snapshot,
};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub const PREPARATION_KIND: &str = "object_import_preparation";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub target_project_id: String,
    pub source_path: PathBuf,
    pub source_project_id: String,
    pub object_id: String,
    pub baseline: Baseline,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind", content = "versionId")]
pub enum Baseline {
    LatestAccepted,
    PinnedVersion(String),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreparedFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preparation {
    pub preparation_id: String,
    pub target_project_id: String,
    pub source_project_id: String,
    pub source_object_id: String,
    pub baseline: Baseline,
    pub accepted_version_id: Option<String>,
    pub objects: Vec<String>,
    pub references: Vec<ObjectReference>,
    pub files: Vec<PreparedFile>,
    pub identity_map: BTreeMap<String, String>,
    pub source_record: Value,
    pub ready_to_commit: bool,
}

fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty() && !Path::new(relative).is_absolute() && !relative.contains(".."),
        "IMPORT_PATH_TRAVERSAL"
    );
    let root = root.canonicalize().context("source root missing")?;
    let path = root.join(relative);
    let canonical = path.canonicalize().context("import file missing")?;
    ensure!(canonical.starts_with(&root), "IMPORT_PATH_TRAVERSAL");
    Ok(canonical)
}

fn version(record: &ObjectRecord, baseline: &Baseline) -> Result<Option<String>> {
    match baseline {
        Baseline::Empty => Ok(None),
        Baseline::LatestAccepted => record
            .versions
            .last()
            .map(|v| Ok(Some(v.version_id.clone())))
            .unwrap_or_else(|| bail!("IMPORT_VERSION_UNSUPPORTED")),
        Baseline::PinnedVersion(id) => {
            ensure!(
                record.versions.iter().any(|v| v.version_id == *id),
                "IMPORT_VERSION_UNSUPPORTED"
            );
            Ok(Some(id.clone()))
        }
    }
}

pub fn prepare(store: &mut crate::store::Store, request: &Request) -> Result<Preparation> {
    ensure!(
        request.target_project_id != request.source_project_id,
        "IMPORT_TARGET_EQUALS_SOURCE"
    );
    let snapshot =
        object_external_snapshot::read(&request.source_path, &request.source_project_id, None)?;
    let root = request.source_path.canonicalize()?;
    let mut by_id = BTreeMap::new();
    for object in snapshot.objects {
        by_id.insert(object.id.clone(), object);
    }
    let source = by_id
        .get(&request.object_id)
        .context("IMPORT_OBJECT_MISSING")?;
    let accepted = version(source, &request.baseline)?;
    let mut pending = vec![source.id.clone()];
    let mut ids = BTreeSet::new();
    let mut refs = Vec::new();
    while let Some(id) = pending.pop() {
        if !ids.insert(id.clone()) {
            continue;
        }
        let object = by_id.get(&id).context("IMPORT_REFERENCE_MISSING")?;
        for reference in &object.references {
            ensure!(
                reference.project_id == request.source_project_id,
                "CROSS_PROJECT_OBJECT_REFERENCE"
            );
            refs.push(reference.clone());
            pending.push(reference.object_id.clone());
        }
    }
    let mut files = BTreeMap::new();
    for id in &ids {
        for file in &by_id[id].files {
            let path = safe_path(&root, &file.path)?;
            let metadata = std::fs::metadata(&path)?;
            let hash = file_hash(&path)?.context("IMPORT_FILE_MISSING")?;
            files.insert(
                file.path.clone(),
                PreparedFile {
                    path: file.path.clone(),
                    bytes: metadata.len(),
                    sha256: hash,
                },
            );
        }
    }
    for prepared in files.values() {
        let path = safe_path(&root, &prepared.path)?;
        ensure!(
            file_hash(&path)?.as_deref() == Some(prepared.sha256.as_str()),
            "IMPORT_SOURCE_CHANGED"
        );
    }
    let mut identity_map = BTreeMap::new();
    for id in &ids {
        let mut h = Sha256::new();
        h.update(request.target_project_id.as_bytes());
        h.update(id.as_bytes());
        identity_map.insert(id.clone(), format!("object-{:x}", h.finalize()));
    }
    let source_record = serde_json::to_value(source)?;
    let canonical = serde_json::to_vec(&(request, &ids, &files, &accepted))?;
    let preparation_id = format!("import-{:x}", Sha256::digest(canonical));
    let preparation = Preparation {
        preparation_id: preparation_id.clone(),
        target_project_id: request.target_project_id.clone(),
        source_project_id: request.source_project_id.clone(),
        source_object_id: request.object_id.clone(),
        baseline: request.baseline.clone(),
        accepted_version_id: accepted,
        objects: ids.into_iter().collect(),
        references: refs,
        files: files.into_values().collect(),
        identity_map,
        source_record,
        ready_to_commit: false,
    };
    store.put(PREPARATION_KIND, &preparation_id, &preparation)?;
    Ok(preparation)
}

pub fn get(store: &crate::store::Store, id: &str) -> Result<Option<Preparation>> {
    Ok(store.get(PREPARATION_KIND, id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_wire_format_is_explicit_and_stable() {
        assert_eq!(
            serde_json::to_value(Baseline::LatestAccepted).unwrap(),
            serde_json::json!({"kind":"latestAccepted"})
        );
        assert_eq!(
            serde_json::to_value(Baseline::PinnedVersion("v1".into())).unwrap(),
            serde_json::json!({"kind":"pinnedVersion","versionId":"v1"})
        );
    }

    #[test]
    fn rejects_absolute_and_parent_paths() {
        let root = tempfile::tempdir().unwrap();
        assert!(safe_path(root.path(), "../outside.txt").is_err());
        assert!(safe_path(root.path(), "C:/outside.txt").is_err());
    }
}
