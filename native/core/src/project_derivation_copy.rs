//! Offline preparation only: the copied project stays blocked until identity remapping.
use crate::{
    data_backup::{self, Entry},
    files::safe_path,
    project_storage, project_storage_database as database, project_storage_layout as layout,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const PENDING: &str = ".beaver-migration-pending";
const RECEIPT: &str = "DERIVATION-COPY.json";
const EXCLUDED: &[&str] = &[".beaver/.project.lock"];

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub source: PathBuf,
    pub source_project_id: String,
    pub target_project_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prepared {
    pub format: String,
    pub request: Request,
    pub entries: Vec<Entry>,
    pub identities: crate::project_derivation_identity::IdentityMap,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInspection {
    pub request: Request,
    pub entities: usize,
    pub calls: usize,
}

/// Discover identity and reject unsupported history without opening the source database.
/// The proposed identities are not reserved; preparation revalidates the offline source.
pub fn inspect_source(source: &Path) -> Result<SourceInspection> {
    let source = layout::root(source)?;
    layout::read_manifest(&source, None)?;
    layout::validate_files(&source)?;
    let directory = source.join(layout::CONTROL_DIR);
    let _lock = project_storage::lock(&directory, false)?;
    let _database = data_backup::hold_named_database(&directory, layout::DATABASE)?;
    let manifest = layout::manifest_in(&source, None)?;
    layout::validate_files(&source)?;
    let snapshot = database::snapshot(&directory, &manifest)?;
    let request = Request {
        request_id: uuid::Uuid::new_v4().to_string(),
        source,
        source_project_id: manifest.project_id,
        target_project_id: uuid::Uuid::new_v4().to_string(),
    };
    validate_request(&request)?;
    let identities = snapshot.derivation_identities(&request)?;
    let entries = data_backup::inventory_without(&request.source, EXCLUDED)?;
    crate::project_derivation_paths::Paths(&identities).entries(&entries)?;
    snapshot.derivation_blobs(&entries)?;
    Ok(SourceInspection {
        request,
        entities: identities.entities.len(),
        calls: identities.calls.len(),
    })
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn validate_request(request: &Request) -> Result<()> {
    layout::valid_id(&request.request_id)?;
    layout::valid_id(&request.source_project_id)?;
    layout::valid_id(&request.target_project_id)?;
    ensure!(request.source.is_absolute(), "source must be absolute");
    ensure!(
        request.source_project_id != request.target_project_id,
        "independent copy requires a new project identity"
    );
    Ok(())
}

/// Requires an idle source and a new destination. Never opens source SQLite or registers a copy.
/// Failures retain the pending directory; inspect a completed preparation or retry a new target.
pub fn prepare(request: Request, destination: &Path) -> Result<Prepared> {
    validate_request(&request)?;
    ensure!(destination.is_absolute(), "destination must be absolute");
    let source = layout::root(&request.source)?;
    let directory = source.join(layout::CONTROL_DIR);
    layout::read_manifest(&source, Some(&request.source_project_id))?;
    layout::validate_files(&source)?;
    let _lock = project_storage::lock(&directory, false)?;
    let _database = data_backup::hold_named_database(&directory, layout::DATABASE)?;
    let manifest = layout::manifest_in(&source, Some(&request.source_project_id))?;
    layout::validate_files(&source)?;
    let snapshot = database::snapshot(&directory, &manifest)?;
    let identities = snapshot.derivation_identities(&request)?;
    let project = snapshot
        .project(&request.source_project_id)?
        .context("missing project entity")?;
    ensure!(
        project["id"] == request.source_project_id,
        "project entity identity mismatch"
    );
    let request = Request {
        source: source.clone(),
        ..request
    };
    let entries = data_backup::inventory_without(&source, EXCLUDED)?;
    crate::project_derivation_paths::Paths(&identities).entries(&entries)?;
    snapshot.derivation_blobs(&entries)?;
    let destination = data_backup::new_destination(&source, destination)?;
    write_new(&destination.join(PENDING), &serde_json::to_vec(&request)?)?;
    let target = destination.join("project");
    fs::create_dir(&target)?;
    for entry in &entries {
        let output = safe_path(&target, &entry.path)?;
        if entry.sha256.is_none() {
            fs::create_dir(output)?;
        } else {
            let mut input = File::open(safe_path(&source, &entry.path)?)?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)?;
            ensure!(
                std::io::copy(&mut input, &mut output)? == entry.bytes,
                "source changed during copy"
            );
            output.sync_all()?;
        }
    }
    // A lock represents local ownership, not source content. Windows forbids reading a held lock.
    write_new(&target.join(layout::CONTROL_DIR).join(layout::LOCK), &[])?;
    ensure!(
        data_backup::inventory_without(&target, EXCLUDED)? == entries,
        "copy differs from source"
    );
    ensure!(
        data_backup::inventory_without(&source, EXCLUDED)? == entries,
        "source changed during copy"
    );
    let prepared = Prepared {
        format: "beaver-project-derivation-copy-v2".into(),
        request,
        entries,
        identities,
    };
    write_new(
        &destination.join(RECEIPT),
        &serde_json::to_vec_pretty(&prepared)?,
    )?;
    Ok(prepared)
}

/// Revalidates a completed preparation without the original machine or any SQLite writes.
/// This is not activation: neither pending nor the copied source identities are changed.
pub fn inspect(destination: &Path) -> Result<Prepared> {
    Ok(verified_snapshot(destination)?.0)
}

pub(crate) fn verified_snapshot(destination: &Path) -> Result<(Prepared, database::Snapshot)> {
    let destination = layout::partition_root(destination)?;
    layout::ordinary(&destination.join(PENDING), false)?;
    layout::ordinary(&destination.join(RECEIPT), false)?;
    let request: Request = serde_json::from_slice(&fs::read(destination.join(PENDING))?)?;
    validate_request(&request)?;
    let prepared: Prepared = serde_json::from_slice(&fs::read(destination.join(RECEIPT))?)?;
    ensure!(
        prepared.format == "beaver-project-derivation-copy-v2" && prepared.request == request,
        "invalid derivation copy receipt"
    );
    let target = destination.join("project");
    layout::ordinary(&target, true)?;
    let manifest = layout::manifest_in(&target, Some(&request.source_project_id))?;
    layout::validate_files(&target)?;
    let _lock = project_storage::lock(&target.join(layout::CONTROL_DIR), false)?;
    let _database =
        data_backup::hold_named_database(&target.join(layout::CONTROL_DIR), layout::DATABASE)?;
    ensure!(
        data_backup::inventory_without(&target, EXCLUDED)? == prepared.entries,
        "prepared copy changed or is incomplete"
    );
    let snapshot = database::snapshot(&target.join(layout::CONTROL_DIR), &manifest)?;
    let identities = snapshot.derivation_identities(&request)?;
    snapshot.derivation_blobs(&prepared.entries)?;
    ensure!(
        identities == prepared.identities,
        "derivation identity map changed or is incomplete"
    );
    Ok((prepared, snapshot))
}

#[cfg(all(test, windows))]
#[path = "project_derivation_copy_tests.rs"]
mod tests;
