//! Durable identity and declared-path generations. No project activation is performed.
//! The intermediate schema lacks project storage metadata and cannot replace project.sqlite.
use crate::{
    data_backup::{self, Entry},
    files::safe_path,
    project_derivation_copy::{self as copy, Prepared},
    project_derivation_database::Archive,
    project_derivation_validation, project_storage_layout as layout, store_schema,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const RECEIPT: &str = "IDENTITY-STAGE.json";
const DATABASE: &str = "identity.sqlite";
const ARCHIVE: &str = "validation-request-archive.json";
const FORMAT: &str = "beaver-project-derivation-stage-v2";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub format: String,
    pub preparation_sha256: String,
    pub entries: Vec<Entry>,
}

fn directory(preparation: &Path, generation: &str) -> Result<PathBuf> {
    layout::valid_id(generation)?;
    safe_path(preparation, &format!("identity-stage-{generation}"))
}

fn digest(prepared: &Prepared) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(prepared)?)
    ))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// New generations only. A failed generation remains for diagnosis; retry using a new name.
pub fn create(preparation: &Path, generation: &str) -> Result<Receipt> {
    let (prepared, snapshot) = copy::verified_snapshot(preparation)?;
    let directory = directory(preparation, generation)?;
    let staged = snapshot.derive(&prepared)?;
    fs::create_dir(&directory)?;
    let database = directory.join(DATABASE);
    // VACUUM INTO exports the committed in-memory DB without introducing a WAL sidecar.
    staged.connection.execute(
        "VACUUM INTO ?",
        [database.to_str().context("invalid stage path")?],
    )?;
    OpenOptions::new().write(true).open(&database)?.sync_all()?;
    write_new(
        &directory.join(ARCHIVE),
        &serde_json::to_vec_pretty(&staged.archive)?,
    )?;
    let receipt = Receipt {
        format: FORMAT.into(),
        preparation_sha256: digest(&prepared)?,
        entries: data_backup::inventory(&directory)?,
    };
    validate_payload(&directory, &prepared)?;
    write_new(
        &directory.join(RECEIPT),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

/// Requires the unchanged prepared copy, but never needs the original source machine/path.
pub fn inspect(preparation: &Path, generation: &str) -> Result<Receipt> {
    let prepared = copy::inspect(preparation)?;
    let directory = directory(preparation, generation)?;
    layout::ordinary(&directory, true)?;
    layout::ordinary(&directory.join(RECEIPT), false)?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(directory.join(RECEIPT))?)?;
    ensure!(
        receipt.format == FORMAT && receipt.preparation_sha256 == digest(&prepared)?,
        "identity stage preparation mismatch"
    );
    ensure!(
        data_backup::inventory_without(&directory, &[RECEIPT])? == receipt.entries,
        "identity stage changed or incomplete"
    );
    validate_payload(&directory, &prepared)?;
    Ok(receipt)
}

fn validate_payload(directory: &Path, prepared: &Prepared) -> Result<()> {
    let entries = data_backup::inventory_without(directory, &[RECEIPT])?;
    ensure!(
        entries.len() == 2
            && entries.iter().all(|entry| {
                (entry.path == DATABASE || entry.path == ARCHIVE) && entry.sha256.is_some()
            }),
        "unexpected identity stage files"
    );
    let archive: Archive = serde_json::from_slice(&fs::read(directory.join(ARCHIVE))?)?;
    ensure!(
        archive.format == "beaver-project-derivation-archive-v1"
            && archive.request_id == prepared.request.request_id
            && archive.source_project_id == prepared.request.source_project_id
            && archive.target_project_id == prepared.request.target_project_id,
        "identity archive provenance mismatch"
    );
    let expected: Vec<_> = prepared
        .identities
        .entities
        .iter()
        .filter_map(|entry| {
            matches!(
                entry.target,
                crate::project_derivation_identity::Target::Archive
            )
            .then_some(&entry.source)
        })
        .collect();
    ensure!(
        archive
            .records
            .iter()
            .map(|record| &record.source)
            .collect::<Vec<_>>()
            == expected,
        "identity archive coverage mismatch"
    );
    let temp = tempfile::tempdir()?;
    fs::copy(directory.join(DATABASE), temp.path().join(DATABASE))?;
    let db =
        Connection::open_with_flags(temp.path().join(DATABASE), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    for (name, sql) in store_schema::DEFINITIONS {
        store_schema::validate_definition(&db, name, sql)?;
    }
    let objects: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        objects == store_schema::DEFINITIONS.len() as i64,
        "unexpected identity stage schema"
    );
    let check: String = db.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    ensure!(check == "ok", "identity stage integrity failure");
    project_derivation_validation::validate(&db, &prepared.request.target_project_id)?;
    let replays: i64 = db.query_row(
        "SELECT count(*) FROM entities WHERE kind='validationRequest'",
        [],
        |row| row.get(0),
    )?;
    ensure!(replays == 0, "archived receipts leaked into replay state");
    Ok(())
}

#[cfg(all(test, windows))]
#[path = "project_derivation_stage_tests.rs"]
mod tests;
