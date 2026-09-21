//! Assemble a recovered project in a new destination, then explicitly publish it.
use crate::{
    data_backup::{self, Entry},
    files::{safe_path, Files},
    project_derivation_copy::{self as copy, Prepared},
    project_derivation_install,
    project_derivation_paths::Paths,
    project_derivation_session_index,
    project_derivation_session_paths::is_index,
    project_storage, project_storage_database as database, project_storage_layout as layout,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::Arc,
};

const FORMAT: &str = "beaver-project-derivation-assembly-v1";
const RECEIPT: &str = "ASSEMBLY.json";
const ACTIVATION: &str = "ASSEMBLY-ACTIVATION.json";
const PENDING: &str = ".beaver-migration-pending";
const ARCHIVE: &str = "validation-request-archive.json";

#[path = "project_derivation_activation.rs"]
mod activation;
pub use activation::{activate, inspect, inspect_activated};
pub(crate) use activation::{registration_receipt, verified_activation};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub format: String,
    pub preparation_sha256: String,
    pub binding: std::path::PathBuf,
    pub project_id: String,
    pub tasks_interrupted: usize,
    pub session_paths_rewritten: usize,
    pub entries: Vec<Entry>,
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn digest(prepared: &Prepared) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(prepared)?)
    ))
}

fn replaced(path: &str) -> bool {
    matches!(
        path,
        ".beaver/project.json"
            | ".beaver/project.sqlite"
            | ".beaver/project.sqlite-wal"
            | ".beaver/project.sqlite-shm"
    ) || is_index(path)
        || ["-wal", "-shm"]
            .iter()
            .any(|suffix| path.strip_suffix(suffix).is_some_and(is_index))
}

/// A failed destination stays pending. Retry into a new destination; never overwrite it.
pub fn create(preparation: &Path, destination: &Path) -> Result<Receipt> {
    ensure!(
        destination.is_absolute(),
        "assembly destination must be absolute"
    );
    let preparation = layout::partition_root(preparation)?;
    let (prepared, snapshot) = copy::verified_snapshot(&preparation)?;
    let staged = snapshot.derive(&prepared)?;
    let paths = Paths(&prepared.identities);
    paths.entries(&prepared.entries)?;
    let candidate = fs::canonicalize(destination.parent().context("destination parent missing")?)?
        .join(
            destination
                .file_name()
                .context("destination name missing")?,
        );
    for excluded in [&preparation, &prepared.request.source] {
        ensure!(
            crate::migration_import::relative(
                excluded,
                candidate.to_str().context("non-UTF-8 destination")?
            )?
            .is_none(),
            "assembly must be outside source and preparation"
        );
    }
    let destination = data_backup::new_destination(&preparation, destination)?;
    write_new(&destination.join(PENDING), FORMAT.as_bytes())?;
    let root = destination.join("project");
    fs::create_dir(&root)?;
    let root = layout::partition_root(&root)?;
    let source = preparation.join("project");
    for entry in &prepared.entries {
        if replaced(&entry.path) {
            continue;
        }
        let output = safe_path(&root, &paths.relative(&entry.path)?)?;
        if entry.sha256.is_none() {
            fs::create_dir(output)?;
        } else {
            let mut input = fs::File::open(safe_path(&source, &entry.path)?)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            ensure!(
                std::io::copy(&mut input, &mut output)? == entry.bytes,
                "assembly source changed"
            );
            output.sync_all()?;
        }
    }
    // Check byte-preserved content before journal recovery intentionally modifies target files.
    let expected: Vec<_> = prepared
        .entries
        .iter()
        .filter(|entry| !replaced(&entry.path))
        .map(|entry| {
            let mut entry = entry.clone();
            entry.path = paths.relative(&entry.path)?;
            Ok(entry)
        })
        .collect::<Result<_>>()?;
    let mut expected = expected;
    expected.sort_by(|left, right| left.path.cmp(&right.path));
    ensure!(
        data_backup::inventory(&root)? == expected,
        "assembled file contents differ"
    );
    let mut session_paths_rewritten = 0;
    for entry in prepared
        .entries
        .iter()
        .filter(|entry| is_index(&entry.path))
    {
        let output = safe_path(&root, &paths.relative(&entry.path)?)?;
        session_paths_rewritten += project_derivation_session_index::convert(
            &prepared,
            &source,
            &entry.path,
            &output,
            &root,
        )?;
    }
    let manifest = layout::Manifest {
        schema_version: layout::SCHEMA_VERSION,
        storage_version: layout::STORAGE_VERSION,
        project_id: prepared.request.target_project_id.clone(),
    };
    let control = root.join(layout::CONTROL_DIR);
    let lock = Arc::new(project_storage::lock(&control, true)?);
    let mut store = database::initialize(&control, &manifest, lock.clone())?;
    let files = Files::project(root.clone(), lock.clone());
    let tasks_interrupted = project_derivation_install::install(
        &staged,
        &mut store,
        &files,
        &root,
        &manifest.project_id,
    )?;
    write_new(
        &control.join(layout::MANIFEST),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    write_new(
        &destination.join(ARCHIVE),
        &serde_json::to_vec_pretty(&staged.archive)?,
    )?;
    layout::validate_files(&root)?;
    drop(store);
    drop(files);
    // Fresh storage validation includes exact schema, identity, WAL mode and integrity.
    database::snapshot(&control, &manifest)?;
    copy::inspect(&preparation)?;
    let receipt = Receipt {
        format: FORMAT.into(),
        preparation_sha256: digest(&prepared)?,
        binding: root,
        project_id: manifest.project_id,
        tasks_interrupted,
        session_paths_rewritten,
        entries: activation::inventory(&destination)?,
    };
    write_new(
        &destination.join(RECEIPT),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

#[cfg(all(test, windows))]
#[path = "project_derivation_assembly_tests.rs"]
mod tests;
