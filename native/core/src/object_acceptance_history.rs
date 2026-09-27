//! Recover acceptance order from committed command evidence, not capture order.
use super::{digest, Receipt, KIND};
use crate::{
    object_catalog::{ObjectRecord, ObjectVersion},
    object_version_acceptance::AcceptanceRequest,
    object_version_manifest::{self, VersionStatus},
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;

pub(crate) fn latest_accepted(
    connection: &Connection,
    object: &ObjectRecord,
) -> Result<Option<String>> {
    let accepted = accepted_versions(object)?;
    let mut statement = connection.prepare(
        "SELECT id,value FROM entities WHERE kind=? AND json_extract(value,'$.result.object.id')=?",
    )?;
    let rows = statement.query_map(params![KIND, object.id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut history = BTreeMap::new();
    for row in rows {
        let (key, json) = row?;
        let receipt: Receipt = serde_json::from_str(&json)?;
        let result = receipt.result;
        ensure!(
            result.project_id == object.project_id
                && result.object.project_id == object.project_id
                && result.object.id == object.id
                && key == digest(&(&result.project_id, &result.request_id))?,
            "OBJECT_RECEIPT_IDENTITY_MISMATCH"
        );
        let Some(version_id) = &result.version_id else {
            continue;
        };
        let version = result
            .object
            .versions
            .iter()
            .find(|version| &version.version_id == version_id)
            .context("OBJECT_ACCEPTANCE_HISTORY_INVALID")?;
        if object_version_manifest::read_version(&result.object, version)?.status
            != VersionStatus::Accepted
        {
            continue;
        }
        let revision = result.object.revision;
        ensure!(
            revision > 0 && revision <= object.revision,
            "OBJECT_ACCEPTANCE_HISTORY_INVALID"
        );
        let request = AcceptanceRequest {
            project_id: result.project_id.clone(),
            request_id: result.request_id.clone(),
            object_id: object.id.clone(),
            version_id: version_id.clone(),
            expected_revision: revision - 1,
        };
        ensure!(
            receipt.input_digest == digest(&("accept", &request))?
                && accepted.get(version_id.as_str()).copied() == Some(version),
            "OBJECT_ACCEPTANCE_HISTORY_INVALID"
        );
        ensure!(
            history.insert(revision, result).is_none(),
            "OBJECT_ACCEPTANCE_HISTORY_INVALID"
        );
    }
    if let Some((_, latest)) = history.last_key_value() {
        ensure!(
            accepted_versions(&latest.object)? == accepted,
            "OBJECT_ACCEPTANCE_ORDER_AMBIGUOUS"
        );
        return Ok(latest.version_id.clone());
    }
    ensure!(accepted.len() <= 1, "OBJECT_ACCEPTANCE_ORDER_AMBIGUOUS");
    Ok(accepted.keys().next().map(|id| (*id).to_owned()))
}

fn accepted_versions(object: &ObjectRecord) -> Result<BTreeMap<&str, &ObjectVersion>> {
    let mut accepted = BTreeMap::new();
    let mut ids = std::collections::HashSet::new();
    for version in &object.versions {
        ensure!(
            ids.insert(&version.version_id),
            "OBJECT_ACCEPTANCE_HISTORY_INVALID"
        );
        if object_version_manifest::read_version(object, version)?.status == VersionStatus::Accepted
        {
            accepted.insert(version.version_id.as_str(), version);
        }
    }
    Ok(accepted)
}
