//! Immutable, request-scoped receipts; source access is unnecessary for a retry.
use crate::{object_import_preparation::Baseline, object_version_manifest::VersionManifest};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub const PREPARATION_KIND: &str = "object_import_preparation";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMap {
    pub objects: BTreeMap<String, String>,
    pub components: BTreeMap<String, String>,
    pub versions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preparation {
    pub schema_version: u32,
    pub preparation_id: String,
    pub request_id: String,
    pub request_digest: String,
    pub target_project_id: String,
    pub source_path: PathBuf,
    pub source_project_id: String,
    pub source_object_id: String,
    pub baseline: Baseline,
    pub accepted_version_id: String,
    pub source_digest: String,
    pub versions: Vec<VersionManifest>,
    pub identity_map: IdentityMap,
    pub ready_to_commit: bool,
}

fn read(connection: &Connection, id: &str) -> Result<Option<Preparation>> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![PREPARATION_KIND, id],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| {
            let receipt: Preparation = serde_json::from_str(&value)
                .context("IMPORT_PREPARATION_UNSUPPORTED: prepare a new request")?;
            ensure!(
                receipt.schema_version == 1 && receipt.preparation_id == id,
                "IMPORT_PREPARATION_UNSUPPORTED"
            );
            Ok(receipt)
        })
        .transpose()
}

pub fn get(store: &crate::store::Store, id: &str) -> Result<Option<Preparation>> {
    read(&store.connection, id)
}

pub(crate) fn replay(
    store: &crate::store::Store,
    id: &str,
    request_digest: &str,
) -> Result<Option<Preparation>> {
    let receipt = get(store, id)?;
    if let Some(receipt) = &receipt {
        ensure!(
            receipt.request_digest == request_digest,
            "IMPORT_REQUEST_CONFLICT"
        );
    }
    Ok(receipt)
}

pub(crate) fn persist(
    store: &mut crate::store::Store,
    receipt: Preparation,
) -> Result<Preparation> {
    store.transaction(|connection| {
        if let Some(existing) = read(connection, &receipt.preparation_id)? {
            ensure!(
                existing.request_digest == receipt.request_digest,
                "IMPORT_REQUEST_CONFLICT"
            );
            return Ok(existing);
        }
        connection.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
            params![
                PREPARATION_KIND,
                receipt.preparation_id,
                serde_json::to_string(&receipt)?
            ],
        )?;
        Ok(receipt)
    })
}
