//! Claim-time selection of accepted content and the independent publication comparison point.
use crate::{
    files::Snapshot,
    object_catalog::{self, ObjectRecord, ObjectVersion},
    object_command_receipt,
    object_framework::{Baseline, Identity},
    object_import_snapshot, object_task_storage,
    object_task_types::TaskRecord,
    object_version_manifest::VersionManifest,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenBaseline {
    pub policy: Baseline,
    pub resolved_version_id: Option<String>,
    pub accepted_version_id_at_claim: Option<String>,
    pub object_revision_at_claim: u64,
    pub versions: Vec<VersionManifest>,
    pub content_digest: String,
}

pub(crate) fn resolve(connection: &Connection, task: &TaskRecord) -> Result<FrozenBaseline> {
    let Identity::Medium {
        object_id,
        baseline,
        ..
    } = &task.identity
    else {
        anyhow::bail!("OBJECT_TASK_QUEUE_REQUIRES_MEDIUM");
    };
    let object = object_catalog::read(connection, object_id)?.context("OBJECT_NOT_FOUND")?;
    ensure!(
        object.project_id == task.project_id
            && task.object_id.as_deref() == Some(object_id.as_str()),
        "OBJECT_TASK_PROJECT_MISMATCH"
    );
    let accepted_version_id_at_claim =
        object_command_receipt::latest_accepted(connection, &object)?;
    let resolved_version_id = match baseline {
        Baseline::LatestAccepted {} => Some(
            accepted_version_id_at_claim
                .clone()
                .context("OBJECT_BASELINE_NO_ACCEPTED_VERSION")?,
        ),
        Baseline::PinnedVersion {
            selected_version_id,
        } => Some(selected_version_id.clone()),
        Baseline::Empty {} => None,
    };
    let versions = match &resolved_version_id {
        Some(version_id) => {
            let objects = catalog(connection, &task.project_id)?;
            object_import_snapshot::select(&objects, &task.project_id, object_id, version_id)?
                .versions
        }
        None => vec![],
    };
    for version in &versions {
        let saved = object_task_storage::read::<(String, ObjectVersion)>(
            connection,
            "object_version",
            &version.version_id,
        )?
        .context("OBJECT_VERSION_NOT_FOUND")?;
        ensure!(
            saved.0 == version.object_id
                && saved.1.version_id == version.version_id
                && serde_json::from_value::<VersionManifest>(saved.1.manifest)? == *version,
            "OBJECT_VERSION_MISMATCH"
        );
    }
    snapshot(&versions)?;
    Ok(FrozenBaseline {
        policy: baseline.clone(),
        resolved_version_id,
        accepted_version_id_at_claim,
        object_revision_at_claim: object.revision,
        content_digest: object_import_snapshot::digest(&versions)?,
        versions,
    })
}

fn catalog(connection: &Connection, project_id: &str) -> Result<Vec<ObjectRecord>> {
    let mut statement = connection.prepare(
        "SELECT id FROM entities WHERE kind='object' AND json_extract(value,'$.projectId')=? ORDER BY id",
    )?;
    let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
    rows.map(|row| object_catalog::read(connection, &row?)?.context("OBJECT_NOT_FOUND"))
        .collect()
}

pub(crate) fn snapshot(versions: &[VersionManifest]) -> Result<Snapshot> {
    let mut objects = BTreeMap::new();
    let mut paths = BTreeMap::new();
    let mut snapshot = Snapshot::new();
    for version in versions {
        ensure!(
            objects
                .insert(&version.object_id, &version.version_id)
                .is_none(),
            "OBJECT_BASELINE_REFERENCE_VERSION_CONFLICT"
        );
        for file in &version.files {
            ensure!(
                paths
                    .insert(file.path.to_lowercase(), &version.object_id)
                    .is_none(),
                "OBJECT_BASELINE_FILE_CONFLICT: {}",
                file.path
            );
            snapshot.insert(file.path.clone(), file.sha256.clone());
        }
    }
    Ok(snapshot)
}
