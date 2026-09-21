//! Byte-preserving, pending file generations. SQLite contents and manifests remain unconverted.
use crate::{
    data_backup::{self, Entry},
    files::safe_path,
    project_derivation_copy::{self as copy, Prepared},
    project_derivation_paths::Paths,
    project_storage_layout as layout,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const RECEIPT: &str = "FILES-STAGE.json";
const FORMAT: &str = "beaver-project-derivation-files-v1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub format: String,
    pub preparation_sha256: String,
    pub entries: Vec<Entry>,
}

fn directory(preparation: &Path, generation: &str) -> Result<PathBuf> {
    layout::valid_id(generation)?;
    safe_path(preparation, &format!("files-stage-{generation}"))
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

/// Retains failed generations. The caller must use a fresh name to retry.
/// This does not rewrite JSONL, Codex indexes, project metadata or database records.
pub fn create(preparation: &Path, generation: &str) -> Result<Receipt> {
    let prepared = copy::inspect(preparation)?;
    let paths = Paths(&prepared.identities);
    let entries = paths.entries(&prepared.entries)?;
    let directory = directory(preparation, generation)?;
    fs::create_dir(&directory)?;
    // Keep a local block as well as the preparation's ancestor block, even if moved alone.
    write_new(
        &directory.join(".beaver-migration-pending"),
        FORMAT.as_bytes(),
    )?;
    let target = directory.join("project");
    fs::create_dir(&target)?;
    let source = preparation.join("project");
    for entry in &prepared.entries {
        let output = safe_path(&target, &paths.relative(&entry.path)?)?;
        if entry.sha256.is_none() {
            fs::create_dir(output)?;
        } else {
            let mut input = File::open(safe_path(&source, &entry.path)?)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            ensure!(
                std::io::copy(&mut input, &mut output)? == entry.bytes,
                "file changed during derivation"
            );
            output.sync_all()?;
        }
    }
    ensure!(
        data_backup::inventory(&target)? == entries,
        "derived file contents differ"
    );
    copy::inspect(preparation)?;
    let receipt = Receipt {
        format: FORMAT.into(),
        preparation_sha256: digest(&prepared)?,
        entries,
    };
    write_new(
        &directory.join(RECEIPT),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

/// Validates exact coverage, remapped names and original content without the source machine.
pub fn inspect(preparation: &Path, generation: &str) -> Result<Receipt> {
    let prepared = copy::inspect(preparation)?;
    let directory = directory(preparation, generation)?;
    layout::ordinary(&directory, true)?;
    let pending = safe_path(&directory, ".beaver-migration-pending")?;
    layout::ordinary(&pending, false)?;
    ensure!(
        fs::read(pending)? == FORMAT.as_bytes(),
        "file stage pending marker changed"
    );
    let receipt = safe_path(&directory, RECEIPT)?;
    layout::ordinary(&receipt, false)?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(receipt)?)?;
    ensure!(
        receipt.format == FORMAT && receipt.preparation_sha256 == digest(&prepared)?,
        "file stage preparation mismatch"
    );
    ensure!(
        receipt.entries == Paths(&prepared.identities).entries(&prepared.entries)?,
        "file stage mapping mismatch"
    );
    let target = safe_path(&directory, "project")?;
    layout::ordinary(&target, true)?;
    ensure!(
        data_backup::inventory(&target)? == receipt.entries,
        "file stage changed or incomplete"
    );
    let mut names = fs::read_dir(&directory)?
        .map(|entry| Ok(entry?.file_name()))
        .collect::<Result<Vec<_>>>()?;
    names.sort();
    ensure!(
        names == [".beaver-migration-pending", RECEIPT, "project"].map(std::ffi::OsString::from),
        "unexpected file stage entries"
    );
    Ok(receipt)
}

#[cfg(all(test, windows))]
#[path = "project_derivation_files_tests.rs"]
mod tests;
