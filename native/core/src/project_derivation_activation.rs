//! Read-only assembly verification and receipt-before-marker publication.
use super::*;
use serde_json::{json, Value};

const LOCK_PATH: &str = "project/.beaver/.project.lock";

pub(crate) struct Verified {
    pub receipt: Receipt,
    pub snapshot: database::Snapshot,
    // Keep writer exclusion through publication or the host registration commit.
    _lock: fs::File,
}

pub(super) fn inventory(destination: &Path) -> Result<Vec<Entry>> {
    let lock = safe_path(destination, LOCK_PATH)?;
    layout::ordinary(&lock, false)?;
    ensure!(fs::metadata(lock)?.len() == 0, "assembly lock file changed");
    // Windows prohibits reading an exclusively locked file, including from this process.
    let mut entries =
        data_backup::inventory_without(destination, &[PENDING, RECEIPT, ACTIVATION, LOCK_PATH])?;
    entries.push(Entry {
        path: LOCK_PATH.into(),
        bytes: 0,
        sha256: Some(format!("{:x}", Sha256::digest(b""))),
    });
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn read_receipt(preparation: &Path, destination: &Path, pending: bool) -> Result<Receipt> {
    let prepared = copy::inspect(preparation)?;
    layout::ordinary(&destination.join(RECEIPT), false)?;
    if pending {
        layout::ordinary(&destination.join(PENDING), false)?;
        ensure!(
            fs::read(destination.join(PENDING))? == FORMAT.as_bytes(),
            "assembly pending changed"
        );
    } else {
        ensure!(
            layout::absent(&destination.join(PENDING))?,
            "assembly remains pending"
        );
    }
    let receipt: Receipt = serde_json::from_slice(&fs::read(destination.join(RECEIPT))?)?;
    ensure!(
        receipt.format == FORMAT
            && receipt.preparation_sha256 == digest(&prepared)?
            && receipt.project_id == prepared.request.target_project_id,
        "assembly provenance mismatch"
    );
    let root = layout::partition_root(&destination.join("project"))?;
    ensure!(
        receipt.binding == root,
        "assembly moved; regenerate absolute session bindings"
    );
    layout::manifest_in(&root, Some(&receipt.project_id))?;
    layout::validate_files(&root)?;
    Ok(receipt)
}

fn verify(preparation: &Path, destination: &Path, pending: bool) -> Result<Verified> {
    let destination = layout::partition_root(destination)?;
    let root = layout::partition_root(&destination.join("project"))?;
    layout::validate_files(&root)?;
    let directory = root.join(layout::CONTROL_DIR);
    let lock = project_storage::lock(&directory, false)?;
    let receipt = read_receipt(preparation, &destination, pending)?;
    ensure!(
        inventory(&destination)? == receipt.entries,
        "assembly changed or incomplete"
    );
    let manifest = layout::manifest_in(&root, Some(&receipt.project_id))?;
    let snapshot = database::snapshot(&directory, &manifest)?;
    let project = snapshot
        .project(&receipt.project_id)?
        .context("assembled project missing")?;
    ensure!(
        project["id"] == receipt.project_id,
        "assembled project identity mismatch"
    );
    ensure!(
        project["path"].as_str() == root.to_str(),
        "assembled project binding mismatch"
    );
    Ok(Verified {
        receipt,
        snapshot,
        _lock: lock,
    })
}

fn activation_value(receipt: &Receipt) -> Value {
    json!({
        "format": "beaver-project-derivation-activation-v1",
        "assembly": receipt,
        "binding": receipt.binding,
        "host_registration_changed": false,
    })
}

fn check_activation(destination: &Path, receipt: &Receipt) -> Result<Value> {
    let path = destination.join(ACTIVATION);
    layout::ordinary(&path, false)?;
    let existing: Value = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        existing == activation_value(receipt),
        "assembly activation receipt differs"
    );
    Ok(existing)
}

/// Only successful host registration may authorize replay without the original inventory.
/// This still verifies provenance, absolute binding, layout and the complete activation record.
pub(crate) fn registration_receipt(preparation: &Path, destination: &Path) -> Result<Receipt> {
    let destination = layout::partition_root(destination)?;
    let receipt = read_receipt(preparation, &destination, false)?;
    check_activation(&destination, &receipt)?;
    Ok(receipt)
}

pub(crate) fn verified_activation(preparation: &Path, destination: &Path) -> Result<Verified> {
    let destination = layout::partition_root(destination)?;
    let verified = verify(preparation, &destination, false)?;
    check_activation(&destination, &verified.receipt)?;
    Ok(verified)
}

/// Verify an idle pending assembly. Only an isolated SQLite snapshot is opened.
pub fn inspect(preparation: &Path, destination: &Path) -> Result<Receipt> {
    Ok(verify(preparation, destination, true)?.receipt)
}

/// Sync the activation receipt before removing pending; retry preserves the receipt.
/// Local publication never changes host registration.
pub fn activate(preparation: &Path, destination: &Path) -> Result<Value> {
    let destination = layout::partition_root(destination)?;
    let activation = destination.join(ACTIVATION);
    let marker = destination.join(PENDING);
    let activated = !layout::absent(&activation)? && layout::absent(&marker)?;
    let verified = verify(preparation, &destination, !activated)?;
    let result = activation_value(&verified.receipt);
    if !layout::absent(&activation)? {
        check_activation(&destination, &verified.receipt)?;
        if !layout::absent(&marker)? {
            fs::remove_file(&marker)?;
        }
    } else {
        crate::migration_activation::write_receipt(&activation, &marker, &result)?;
    }
    Ok(result)
}

/// Full verification is for idle, unchanged assemblies, before ordinary project use.
pub fn inspect_activated(preparation: &Path, destination: &Path) -> Result<Value> {
    Ok(activation_value(
        &verified_activation(preparation, destination)?.receipt,
    ))
}
