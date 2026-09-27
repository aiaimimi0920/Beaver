//! Prepare ordinary file sources without copying or committing their content.
use crate::{
    object_import_file_source::{self, FileSourceSnapshot},
    object_import_snapshot,
    object_version_manifest::valid_id,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Mutex,
};

pub const PREPARATION_KIND: &str = "file_object_import_preparation";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileImportGroup {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub target_project_id: String,
    pub snapshot: FileSourceSnapshot,
    pub groups: Vec<FileImportGroup>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMap {
    pub files: BTreeMap<String, String>,
    pub groups: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preparation {
    pub schema_version: u32,
    pub preparation_id: String,
    pub request_id: String,
    pub request_digest: String,
    pub target_project_id: String,
    pub source: FileSourceSnapshot,
    pub groups: Vec<FileImportGroup>,
    pub identity_map: IdentityMap,
    pub ready_to_commit: bool,
}

pub fn prepare(target: &Mutex<Store>, request: &Request) -> Result<Preparation> {
    validate_request(request)?;
    let preparation_id = format!(
        "file-import-{}",
        object_import_snapshot::digest(&(1, &request.target_project_id, &request.request_id))?
    );
    let request_digest = object_import_snapshot::digest(request)?;
    {
        let store = target
            .lock()
            .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
        validate_target(&store, &request.target_project_id)?;
        if let Some(receipt) = replay(&store, &preparation_id, &request_digest)? {
            return Ok(receipt);
        }
    }
    let paths: Vec<PathBuf> = request
        .snapshot
        .source
        .paths
        .iter()
        .map(PathBuf::from)
        .collect();
    let current = object_import_file_source::inspect(&paths)
        .context("IMPORT_SOURCE_CHANGED: inspect again")?;
    ensure!(
        current == request.snapshot,
        "IMPORT_SOURCE_CHANGED: inspect again"
    );
    let identity_map = identities(request, &request.snapshot)?;
    let prepared = Preparation {
        schema_version: 1,
        preparation_id,
        request_id: request.request_id.clone(),
        request_digest,
        target_project_id: request.target_project_id.clone(),
        source: request.snapshot.clone(),
        groups: request.groups.clone(),
        identity_map,
        ready_to_commit: false,
    };
    let mut store = target
        .lock()
        .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
    validate_target(&store, &request.target_project_id)?;
    persist(&mut store, prepared)
}

fn validate_request(request: &Request) -> Result<()> {
    ensure!(
        valid_id(&request.request_id) && valid_id(&request.target_project_id),
        "IMPORT_INVALID_ID"
    );
    ensure!(
        request.snapshot.source.kind == "files",
        "IMPORT_INVALID_SOURCE_KIND"
    );
    ensure!(!request.snapshot.files.is_empty(), "IMPORT_FILES_EMPTY");
    ensure!(
        request
            .snapshot
            .source
            .paths
            .iter()
            .all(|path| PathBuf::from(path).is_absolute()),
        "IMPORT_SOURCE_PATH_NOT_ABSOLUTE"
    );
    let known: BTreeSet<_> = request
        .snapshot
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let mut assigned = BTreeSet::new();
    let mut group_ids = BTreeSet::new();
    for group in &request.groups {
        ensure!(
            valid_id(&group.id) && group_ids.insert(&group.id),
            "IMPORT_INVALID_GROUP_ID"
        );
        ensure!(
            !group.name.trim().is_empty() && group.name.len() <= 200,
            "IMPORT_INVALID_GROUP_NAME"
        );
        ensure!(!group.paths.is_empty(), "IMPORT_EMPTY_GROUP");
        for path in &group.paths {
            ensure!(
                known.contains(path.as_str()) && assigned.insert(path),
                "IMPORT_INVALID_GROUP_PATH"
            );
        }
    }
    Ok(())
}

fn identities(request: &Request, snapshot: &FileSourceSnapshot) -> Result<IdentityMap> {
    let mut map = IdentityMap::default();
    for file in &snapshot.files {
        map.files.insert(
            file.path.clone(),
            format!(
                "file-{}",
                object_import_snapshot::digest(&(
                    1,
                    &request.target_project_id,
                    &request.request_id,
                    &file.path
                ))?
            ),
        );
    }
    for group in &request.groups {
        map.groups.insert(
            group.id.clone(),
            format!(
                "group-{}",
                object_import_snapshot::digest(&(
                    1,
                    &request.target_project_id,
                    &request.request_id,
                    &group.id
                ))?
            ),
        );
    }
    Ok(map)
}

fn validate_target(store: &Store, project_id: &str) -> Result<()> {
    let project = store.get::<Value>("project", project_id)?;
    ensure!(
        project.is_some_and(|project| project["id"] == project_id),
        "UNKNOWN_IMPORT_TARGET_PROJECT"
    );
    Ok(())
}

fn read(connection: &rusqlite::Connection, id: &str) -> Result<Option<Preparation>> {
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
            validate_receipt(&receipt, id)?;
            Ok(receipt)
        })
        .transpose()
}

fn validate_receipt(receipt: &Preparation, id: &str) -> Result<()> {
    ensure!(
        receipt.schema_version == 1 && !receipt.ready_to_commit && receipt.preparation_id == id,
        "IMPORT_PREPARATION_UNSUPPORTED"
    );
    let request = Request {
        request_id: receipt.request_id.clone(),
        target_project_id: receipt.target_project_id.clone(),
        snapshot: receipt.source.clone(),
        groups: receipt.groups.clone(),
    };
    validate_request(&request)?;
    ensure!(
        id == format!(
            "file-import-{}",
            object_import_snapshot::digest(&(1, &request.target_project_id, &request.request_id))?
        ) && receipt.request_digest == object_import_snapshot::digest(&request)?
            && receipt.source.digest
                == object_import_snapshot::digest(&(
                    &receipt.source.source,
                    &receipt.source.files
                ))?
            && receipt.identity_map == identities(&request, &receipt.source)?,
        "IMPORT_PREPARATION_CORRUPT"
    );
    Ok(())
}

fn replay(store: &Store, id: &str, digest: &str) -> Result<Option<Preparation>> {
    let receipt = read(&store.connection, id)?;
    if let Some(receipt) = &receipt {
        ensure!(receipt.request_digest == digest, "IMPORT_REQUEST_CONFLICT");
    }
    Ok(receipt)
}

fn persist(store: &mut Store, receipt: Preparation) -> Result<Preparation> {
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

pub fn get(store: &Store, id: &str) -> Result<Option<Preparation>> {
    read(&store.connection, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn prepares_and_replays_without_writing_source_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("asset.txt");
        fs::write(&file, b"asset")?;
        let snapshot = object_import_file_source::inspect(std::slice::from_ref(&file))?;
        let store = Mutex::new(Store::open(&temp.path().join("target"))?);
        store
            .lock()
            .unwrap()
            .put("project", "target-1", &serde_json::json!({"id":"target-1"}))?;
        let request = Request {
            request_id: "request-1".into(),
            target_project_id: "target-1".into(),
            snapshot,
            groups: vec![],
        };
        let first = prepare(&store, &request)?;
        assert!(!first.ready_to_commit);
        assert_eq!(prepare(&store, &request)?, first);
        assert_eq!(fs::read(&file)?, b"asset");
        let store_guard = store.lock().unwrap();
        let object_count: i64 = store_guard.connection.query_row(
            "SELECT COUNT(*) FROM entities WHERE kind LIKE 'object%'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(object_count, 0);
        Ok(())
    }

    fn request_for(temp: &tempfile::TempDir) -> Result<(Mutex<Store>, Request, PathBuf)> {
        let file = temp.path().join("asset.txt");
        fs::write(&file, b"asset")?;
        let snapshot = object_import_file_source::inspect(std::slice::from_ref(&file))?;
        let store = Mutex::new(Store::open(&temp.path().join("target"))?);
        store
            .lock()
            .unwrap()
            .put("project", "target-1", &serde_json::json!({"id":"target-1"}))?;
        Ok((
            store,
            Request {
                request_id: "request-1".into(),
                target_project_id: "target-1".into(),
                snapshot,
                groups: vec![],
            },
            file,
        ))
    }

    #[test]
    fn rejects_snapshot_and_file_mutations() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, mut request, file) = request_for(&temp)?;
        let original = request.snapshot.clone();
        request.snapshot.digest = "0".repeat(64);
        let error = prepare(&store, &request).unwrap_err().to_string();
        assert!(error.contains("IMPORT_SOURCE_CHANGED"));

        request.snapshot = original;
        fs::write(&file, b"other")?;
        let error = prepare(&store, &request).unwrap_err().to_string();
        assert!(error.contains("IMPORT_SOURCE_CHANGED"));
        Ok(())
    }

    #[test]
    fn rejects_invalid_groups_and_missing_target() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, mut request, _) = request_for(&temp)?;
        request.groups = vec![FileImportGroup {
            id: "group-1".into(),
            name: "Group".into(),
            paths: vec!["missing.txt".into()],
        }];
        let error = prepare(&store, &request).unwrap_err().to_string();
        assert!(error.contains("IMPORT_INVALID_GROUP_PATH"));

        request.groups.clear();
        request.target_project_id = "unknown-target".into();
        let error = prepare(&store, &request).unwrap_err().to_string();
        assert!(error.contains("UNKNOWN_IMPORT_TARGET_PROJECT"));
        Ok(())
    }

    #[test]
    fn rejects_changed_source_path_and_conflicting_request() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, mut request, _) = request_for(&temp)?;
        prepare(&store, &request)?;
        request.snapshot.source.paths[0] = temp.path().join("other.txt").display().to_string();
        let error = prepare(&store, &request).unwrap_err().to_string();
        assert!(error.contains("IMPORT_REQUEST_CONFLICT"));

        Ok(())
    }

    #[test]
    fn stored_receipt_is_replayable_offline_but_corruption_is_rejected() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (store, request, file) = request_for(&temp)?;
        let receipt = prepare(&store, &request)?;
        fs::remove_file(file)?;
        assert_eq!(prepare(&store, &request)?, receipt);
        assert_eq!(
            get(&store.lock().unwrap(), &receipt.preparation_id)?,
            Some(receipt.clone())
        );
        assert!(get(&store.lock().unwrap(), "missing")?.is_none());

        let original = serde_json::to_value(&receipt)?;
        for pointer in [
            "/schemaVersion",
            "/preparationId",
            "/requestDigest",
            "/targetProjectId",
            "/source/digest",
            "/identityMap/files",
            "/readyToCommit",
        ] {
            let mut invalid = original.clone();
            *invalid.pointer_mut(pointer).unwrap() = match pointer {
                "/schemaVersion" => serde_json::json!(2),
                "/readyToCommit" => serde_json::json!(true),
                "/identityMap/files" => serde_json::json!({}),
                _ => serde_json::json!("tampered"),
            };
            store
                .lock()
                .unwrap()
                .put(PREPARATION_KIND, &receipt.preparation_id, &invalid)?;
            assert!(
                get(&store.lock().unwrap(), &receipt.preparation_id).is_err(),
                "{pointer}"
            );
            assert!(prepare(&store, &request).is_err(), "{pointer}");
        }
        Ok(())
    }
}
