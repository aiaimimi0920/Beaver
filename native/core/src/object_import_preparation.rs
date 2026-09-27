//! Pin and verify accepted source versions without modifying the target catalog.
//! F7 owns copying verified content and committing pending-validation objects.
use crate::{
    object_catalog::ObjectRecord,
    object_import_content, object_import_receipt as receipt, object_import_snapshot,
    object_version_manifest::{valid_digest, valid_id, VersionManifest},
    project_storage_router::ProjectStorageRouter,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Mutex};

pub use receipt::{get, IdentityMap, Preparation, PREPARATION_KIND};

#[path = "object_import_history.rs"]
pub mod history;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub target_project_id: String,
    pub source_path: PathBuf,
    pub source_project_id: String,
    pub object_id: String,
    pub baseline: Baseline,
    pub source_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    content = "versionId",
    deny_unknown_fields
)]
pub enum Baseline {
    LatestAccepted,
    PinnedVersion(String),
}

fn version(record: &ObjectRecord, baseline: &Baseline) -> Result<String> {
    match baseline {
        Baseline::LatestAccepted => record
            .versions
            .iter()
            .rev()
            .find(|version| version.manifest["status"] == "accepted")
            .map(|version| version.version_id.clone())
            .context("IMPORT_NO_ACCEPTED_VERSION"),
        Baseline::PinnedVersion(id) => {
            ensure!(valid_id(id), "IMPORT_INVALID_VERSION_ID");
            Ok(id.clone())
        }
    }
}

pub(crate) fn identities(request: &Request, versions: &[VersionManifest]) -> Result<IdentityMap> {
    let mut identities = IdentityMap::default();
    let remap = |kind: &str, id: &str| -> Result<String> {
        let digest = object_import_snapshot::digest(&(
            1,
            kind,
            &request.target_project_id,
            &request.request_id,
            &request.source_project_id,
            id,
        ))?;
        Ok(format!("{kind}-{digest}"))
    };
    for version in versions {
        identities.objects.insert(
            version.object_id.clone(),
            remap("object", &version.object_id)?,
        );
        identities.versions.insert(
            version.version_id.clone(),
            remap("version", &version.version_id)?,
        );
        for component in &version.components {
            identities
                .components
                .insert(component.id.clone(), remap("component", &component.id)?);
        }
    }
    Ok(identities)
}

pub fn prepare(
    target: &Mutex<Store>,
    sources: &ProjectStorageRouter,
    request: &Request,
) -> Result<Preparation> {
    ensure!(
        [
            &request.request_id,
            &request.target_project_id,
            &request.source_project_id,
            &request.object_id
        ]
        .into_iter()
        .all(|id| valid_id(id)),
        "IMPORT_INVALID_ID"
    );
    ensure!(
        valid_digest(&request.source_digest),
        "IMPORT_INVALID_SOURCE_DIGEST"
    );
    ensure!(
        request.source_path.is_absolute(),
        "IMPORT_SOURCE_PATH_NOT_ABSOLUTE"
    );
    ensure!(
        request.target_project_id != request.source_project_id,
        "IMPORT_TARGET_EQUALS_SOURCE"
    );
    let preparation_id = format!(
        "import-{}",
        object_import_snapshot::digest(&(1, &request.target_project_id, &request.request_id))?
    );
    let request_digest = object_import_snapshot::digest(request)?;
    {
        let store = target
            .lock()
            .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
        validate_target(&store, &request.target_project_id)?;
        if let Some(receipt) = receipt::replay(&store, &preparation_id, &request_digest)? {
            return Ok(receipt);
        }
    }
    // Never hold the target database while resolving or reading the source database.
    let source = sources.object_import_source(&request.source_path, &request.source_project_id)?;
    let snapshot = source.snapshot()?;
    let object = snapshot
        .objects
        .iter()
        .find(|object| object.id == request.object_id)
        .context("IMPORT_OBJECT_MISSING")?;
    let accepted_version_id = version(object, &request.baseline)?;
    let selection = object_import_snapshot::select(
        &snapshot.objects,
        &request.source_project_id,
        &request.object_id,
        &accepted_version_id,
    )?;
    ensure!(
        selection.digest == request.source_digest,
        "IMPORT_SOURCE_CHANGED: inspect again"
    );
    object_import_content::verify(source.root(), &selection.versions)?;
    let prepared = Preparation {
        schema_version: 1,
        preparation_id,
        request_id: request.request_id.clone(),
        request_digest,
        target_project_id: request.target_project_id.clone(),
        source_path: source.root().to_path_buf(),
        source_project_id: request.source_project_id.clone(),
        source_object_id: request.object_id.clone(),
        baseline: request.baseline.clone(),
        accepted_version_id,
        source_digest: selection.digest,
        identity_map: identities(request, &selection.versions)?,
        versions: selection.versions,
        ready_to_commit: false,
    };
    let mut store = target
        .lock()
        .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
    validate_target(&store, &request.target_project_id)?;
    receipt::persist(&mut store, prepared)
}

fn validate_target(store: &Store, project_id: &str) -> Result<()> {
    let project = store.get::<Value>("project", project_id)?;
    ensure!(
        project.is_some_and(|project| project["id"] == project_id),
        "UNKNOWN_IMPORT_TARGET_PROJECT"
    );
    Ok(())
}

#[cfg(test)]
#[path = "object_import_preparation_tests.rs"]
mod tests;
