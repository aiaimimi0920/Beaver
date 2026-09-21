//! Durable session-index generations bound to an explicit final project location.
//! Payloads contain only converted SQLite indexes; JSONL remains in the byte-preserving file stage.
use crate::{
    data_backup::{self, Entry},
    files::safe_path,
    migration_import,
    project_derivation_copy::{self as copy, Prepared},
    project_derivation_paths::Paths,
    project_derivation_session_index, project_derivation_session_paths,
    project_storage_layout as layout,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const FORMAT: &str = "beaver-project-derivation-sessions-v1";
const RECEIPT: &str = "SESSION-STAGE.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub format: String,
    pub preparation_sha256: String,
    pub binding: PathBuf,
    pub paths_rewritten: usize,
    pub entries: Vec<Entry>,
}

fn directory(preparation: &Path, generation: &str) -> Result<PathBuf> {
    layout::valid_id(generation)?;
    safe_path(preparation, &format!("session-stage-{generation}"))
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

fn validate_binding(binding: &Path) -> Result<()> {
    ensure!(binding.is_absolute(), "session binding must be absolute");
    migration_import::relative(binding, binding.to_str().context("non-UTF-8 binding")?)?;
    Ok(())
}

/// The binding need not exist yet. A moved final project requires a fresh generation.
/// Never changes the source, preparation, file stage, or destination binding directory.
pub fn create(preparation: &Path, generation: &str, binding: &Path) -> Result<Receipt> {
    validate_binding(binding)?;
    let prepared = copy::inspect(preparation)?;
    Paths(&prepared.identities).entries(&prepared.entries)?;
    let directory = directory(preparation, generation)?;
    fs::create_dir(&directory)?;
    write_new(
        &directory.join(".beaver-migration-pending"),
        FORMAT.as_bytes(),
    )?;
    let output = directory.join("indexes");
    fs::create_dir(&output)?;
    let mut paths_rewritten = 0;
    for entry in prepared
        .entries
        .iter()
        .filter(|entry| project_derivation_session_paths::is_index(&entry.path))
    {
        let target = safe_path(&output, &Paths(&prepared.identities).relative(&entry.path)?)?;
        fs::create_dir_all(target.parent().context("session output has no parent")?)?;
        paths_rewritten += project_derivation_session_index::convert(
            &prepared,
            &preparation.join("project"),
            &entry.path,
            &target,
            binding,
        )?;
    }
    copy::inspect(preparation)?;
    let receipt = Receipt {
        format: FORMAT.into(),
        preparation_sha256: digest(&prepared)?,
        binding: binding.into(),
        paths_rewritten,
        entries: data_backup::inventory(&output)?,
    };
    write_new(
        &directory.join(RECEIPT),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

/// Recovery is independent of the original machine. Reject stale destination bindings explicitly.
pub fn inspect(preparation: &Path, generation: &str, binding: &Path) -> Result<Receipt> {
    validate_binding(binding)?;
    let prepared = copy::inspect(preparation)?;
    let directory = directory(preparation, generation)?;
    layout::ordinary(&directory, true)?;
    for name in [RECEIPT, ".beaver-migration-pending"] {
        layout::ordinary(&directory.join(name), false)?;
    }
    ensure!(
        fs::read(directory.join(".beaver-migration-pending"))? == FORMAT.as_bytes(),
        "session pending marker changed"
    );
    let receipt: Receipt = serde_json::from_slice(&fs::read(directory.join(RECEIPT))?)?;
    ensure!(
        receipt.format == FORMAT && receipt.preparation_sha256 == digest(&prepared)?,
        "session preparation mismatch"
    );
    ensure!(
        receipt.binding == binding,
        "session destination binding changed; regenerate indexes"
    );
    let output = directory.join("indexes");
    layout::ordinary(&output, true)?;
    ensure!(
        data_backup::inventory(&output)? == receipt.entries,
        "session stage changed or incomplete"
    );
    let expected = prepared
        .entries
        .iter()
        .filter(|entry| project_derivation_session_paths::is_index(&entry.path))
        .map(|entry| Paths(&prepared.identities).relative(&entry.path))
        .collect::<Result<std::collections::BTreeSet<_>>>()?;
    let actual = receipt
        .entries
        .iter()
        .filter(|entry| entry.sha256.is_some())
        .map(|entry| entry.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    ensure!(actual == expected, "session index coverage mismatch");
    let mut names = fs::read_dir(&directory)?
        .map(|entry| Ok(entry?.file_name()))
        .collect::<Result<Vec<_>>>()?;
    names.sort();
    ensure!(
        names == [".beaver-migration-pending", RECEIPT, "indexes"].map(std::ffi::OsString::from),
        "unexpected session stage entries"
    );
    Ok(receipt)
}

#[cfg(all(test, windows))]
#[path = "project_derivation_sessions_tests.rs"]
mod tests;
