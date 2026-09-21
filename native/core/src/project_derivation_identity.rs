//! Stable database identities for an offline, pending independent copy.
use crate::{
    project_derivation_copy::Request, project_derivation_identity_keys as keys,
    project_derivation_validation, project_migration_ownership::Entity,
};
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Key {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    Remap { key: Key },
    // Receipt hashes omit the request input. Keep originals as history, never target replay state.
    Archive,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityIdentity {
    pub source: Key,
    pub target: Target,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMap {
    pub format: String,
    pub entities: Vec<EntityIdentity>,
    pub calls: BTreeMap<String, String>,
}

/// UUIDv8 derived from explicit provenance, independent of source/destination filesystem paths.
pub(crate) fn generated(request: &Request, kind: &str, id: &str) -> Result<String> {
    let digest = Sha256::digest(serde_json::to_vec(&[
        "beaver-project-derivation-identity-v1",
        &request.source_project_id,
        &request.target_project_id,
        &request.request_id,
        kind,
        id,
    ])?);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(uuid::Uuid::from_bytes(bytes).to_string())
}

fn entity_map(entities: &[Entity], request: &Request) -> Result<Vec<EntityIdentity>> {
    let source_keys: BTreeSet<_> = entities
        .iter()
        .map(|entity| (entity.kind.as_str(), entity.id.as_str()))
        .collect();
    let flows = keys::flows(entities, request)?;
    let mut targets = BTreeSet::new();
    let mut result = Vec::new();
    for entity in entities {
        let target = keys::target(entity, request, &flows)?;
        if let Target::Remap { key } = &target {
            ensure!(
                !source_keys.contains(&(key.kind.as_str(), key.id.as_str())),
                "derivation target collides with source {}/{}",
                key.kind,
                key.id
            );
            ensure!(
                targets.insert((key.kind.clone(), key.id.clone())),
                "duplicate derivation target {}/{}",
                key.kind,
                key.id
            );
        }
        result.push(EntityIdentity {
            source: Key {
                kind: entity.kind.clone(),
                id: entity.id.clone(),
            },
            target,
        });
    }
    Ok(result)
}

pub(crate) fn build(connection: &Connection, request: &Request) -> Result<IdentityMap> {
    let entities = project_derivation_validation::validate(connection, &request.source_project_id)?;
    let entities = entity_map(&entities, request)?;
    let mut statement = connection.prepare("SELECT id FROM calls ORDER BY id")?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    let mut targets = BTreeSet::new();
    let mut calls = BTreeMap::new();
    for id in &ids {
        let target = generated(request, "calls", id)?;
        ensure!(
            !ids.contains(&target) && targets.insert(target.clone()),
            "derivation call identity collision"
        );
        calls.insert(id.clone(), target);
    }
    Ok(IdentityMap {
        format: "beaver-project-derivation-identities-v1".into(),
        entities,
        calls,
    })
}

#[cfg(test)]
#[path = "project_derivation_identity_tests.rs"]
mod tests;
