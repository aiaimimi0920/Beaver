//! Enable a partitioned recovery copy: partitions become project stores, the host copy keeps
//! only settings, registrations and explicitly retained legacy history.
use crate::{
    files::{file_hash, safe_path},
    journal::{FileOperation, OperationState},
    legacy_vault, migration_activation, migration_bundle,
    migration_bundle::{RestoreReceipt, RestoredProject},
    migration_import::{rebase_codex_indexes, tool_path},
    project_migration_activation_partition as partition,
    project_migration_partition::{Receipt, FORMAT, RECEIPT},
    store::Store,
};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use rusqlite::params;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

const RETAINED: &str = "migrationRetained";

/// Partition receipts route here; anything else is the prepared-import activation.
pub fn activate_copy(backup: &Path, prepared: &Path, tool_paths: Option<&Path>) -> Result<Value> {
    if fs::canonicalize(prepared)?.join(RECEIPT).is_file() {
        activate(backup, prepared, tool_paths)
    } else {
        migration_activation::activate(backup, prepared, tool_paths)
    }
}

fn receipts(backup: &Path, root: &Path) -> Result<(Receipt, RestoreReceipt)> {
    let receipt: Receipt =
        serde_json::from_value(migration_activation::read_json(&safe_path(root, RECEIPT)?)?)?;
    ensure!(receipt.format == FORMAT, "unsupported partition receipt");
    for (recorded, relative) in [
        (&receipt.backup_manifest_sha256, "BEAVER-MIGRATION.json"),
        (
            &receipt.application_manifest_sha256,
            "application/BEAVER-BACKUP.json",
        ),
    ] {
        ensure!(
            file_hash(&safe_path(backup, relative)?)?.as_ref() == Some(recorded),
            "archive does not match this partitioned copy"
        );
    }
    let restore: RestoreReceipt = serde_json::from_value(migration_activation::read_json(
        &safe_path(root, "RESTORE.json")?,
    )?)?;
    ensure!(
        fs::canonicalize(&receipt.restored_data)? == safe_path(root, "data")?
            && fs::canonicalize(&restore.restored_data)? == safe_path(root, "data")?,
        "partitioned copy moved since partitioning"
    );
    let manifest = migration_bundle::verify(backup)?;
    let archived: BTreeSet<_> = manifest.projects.iter().map(|p| p.id.as_str()).collect();
    let restored: BTreeSet<_> = restore.projects.iter().map(|p| p.id.as_str()).collect();
    let partitioned: BTreeSet<_> = receipt.projects.keys().map(String::as_str).collect();
    ensure!(
        archived == restored && archived == partitioned,
        "partitioned project set differs from archive"
    );
    Ok((receipt, restore))
}

/// Registrations are either still original (first run) or already restored (rerun).
fn host_projects(store: &Store, restore: &RestoreReceipt) -> Result<bool> {
    let projects = store.list_with_ids::<Value>("project")?;
    ensure!(
        projects.len() == restore.projects.len(),
        "host registration set differs from archive"
    );
    let mut converted = 0;
    for project in &restore.projects {
        let (_, value) = projects
            .iter()
            .find(|(key, _)| key == &project.id)
            .context("host registration missing")?;
        ensure!(value["id"] == project.id, "host registration id mismatch");
        if value["path"].as_str() == project.restored_path.to_str() {
            converted += 1;
        } else {
            ensure!(
                value["path"] == project.original_path,
                "host registration changed since partitioning"
            );
        }
    }
    ensure!(
        converted == 0 || converted == restore.projects.len(),
        "host registrations are partially converted"
    );
    Ok(converted > 0)
}

fn host_settings(
    store: &Store,
    mapping: &RestoreReceipt,
    external: &mut BTreeSet<String>,
) -> Result<Vec<(&'static str, String, Value)>> {
    let mut updates = Vec::new();
    if let Some(raw) = store.get::<Value>("settings", "main")? {
        let mut value = raw.clone();
        ensure!(value["tools"].is_object(), "invalid stored tools");
        for slot in ["codex", "godot", "blender", "node"] {
            tool_path(&mut value["tools"][slot], slot, mapping, external)?;
        }
        updates.push(("settings", raw.to_string(), value));
    }
    if let Some(raw) = store.get::<Value>("toolSetup", "main")? {
        let mut value = raw.clone();
        if let Some(steps) = value["steps"].as_array_mut() {
            for step in steps {
                let slot = step["name"]
                    .as_str()
                    .context("invalid setup tool name")?
                    .to_string();
                if let Some(result) = step.get_mut("result").filter(|r| !r.is_null()) {
                    ensure!(result.is_object(), "invalid setup result");
                    tool_path(&mut result["path"], &slot, mapping, external)?;
                    result["available"] = json!(false);
                }
            }
        }
        updates.push((
            "toolSetup",
            raw.to_string(),
            crate::tool_setup::recover(value),
        ));
    }
    Ok(updates)
}

/// Every task still in the host copy is legacy history: mark it so scheduling and
/// continuation refuse it until an explicit conversion exists.
fn retained_tasks(store: &Store, receipt: &Receipt, now: &str) -> Result<Vec<(String, Value)>> {
    let reasons: BTreeMap<_, _> = receipt
        .retained
        .iter()
        .filter(|item| item.table == "entities" && item.kind.as_deref() == Some("task"))
        .map(|item| (item.id.as_str(), item.reason.as_str()))
        .collect();
    store
        .list_with_ids::<Value>("task")?
        .into_iter()
        .map(|(id, task)| {
            ensure!(task["id"] == id, "host task identifier mismatch: {id}");
            let reason = reasons.get(id.as_str()).unwrap_or(&"HOST_UNPARTITIONED");
            let marker = task
                .get(RETAINED)
                .cloned()
                .unwrap_or_else(|| json!({"reason":reason,"source":FORMAT,"markedAt":now}));
            Ok((id, marker))
        })
        .collect()
}

/// Everything except destination tool detection; the copy stays pending and reruns are safe.
pub(crate) struct Staged {
    _lock: fs::File,
    store: Store,
    marker: PathBuf,
    activation: PathBuf,
    summary: Value,
}

pub(crate) fn stage(backup: &Path, prepared: &Path) -> Result<Staged> {
    ensure!(
        cfg!(windows),
        "migration activation currently requires Windows"
    );
    let root = fs::canonicalize(prepared)?;
    migration_bundle::ensure_activated(root.parent().context("prepared root has no parent")?)?;
    let marker = safe_path(&root, ".beaver-migration-pending")?;
    ensure!(
        marker.is_file(),
        "partitioned directory is not pending activation"
    );
    let activation = safe_path(&root, "ACTIVATION.json")?;
    ensure!(
        !activation.try_exists()?,
        "activation receipt already exists; inspect the existing result"
    );
    let (receipt, restore) = receipts(backup, &root)?;
    let data = safe_path(&root, "data")?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(safe_path(&data, ".beaver-native.lock")?)?;
    lock.try_lock_exclusive()
        .context("partitioned data directory is in use")?;
    let mut reports: BTreeMap<String, partition::PartitionReport> = restore
        .projects
        .iter()
        .map(|project| Ok((project.id.clone(), partition::check(project)?)))
        .collect::<Result<_>>()?;
    let mut store = Store::open(&data)?;
    migration_activation::check_tree(&data)?;
    let converted = host_projects(&store, &restore)?;
    ensure!(
        !store
            .list::<FileOperation>("operation")?
            .iter()
            .any(|operation| matches!(
                operation.state,
                OperationState::Applying | OperationState::Aborting
            )),
        "retained file journal requires manual recovery before activation"
    );
    let now = crate::asset_task::now();
    let mut external = BTreeSet::new();
    let updates = if converted {
        Vec::new()
    } else {
        host_settings(&store, &restore, &mut external)?
    };
    let retained = retained_tasks(&store, &receipt, &now)?;
    let credentials = legacy_vault::prepare(&store, &data.join("Local State"))?;
    let projects: Vec<&RestoredProject> = restore.projects.iter().collect();
    let credentials_converted = store.transaction(|db| {
        for (kind, raw, value) in &updates {
            ensure!(
                db.execute(
                    "UPDATE entities SET value=? WHERE kind=? AND id='main' AND value=?",
                    params![value.to_string(), kind, raw]
                )? == 1,
                "host entity changed during activation"
            );
        }
        for project in &projects {
            db.execute(
                "UPDATE entities SET value=json_set(value,'$.path',?) WHERE kind='project' AND id=? AND json_extract(value,'$.path')=?",
                params![project.restored_path.to_string_lossy().into_owned(), project.id, project.original_path],
            )?;
        }
        for (id, marker) in &retained {
            ensure!(
                db.execute(
                    &format!("UPDATE entities SET value=json_set(value,'$.{RETAINED}',json(?)) WHERE kind='task' AND id=?"),
                    params![marker.to_string(), id]
                )? == 1,
                "host task vanished during activation"
            );
        }
        credentials.apply(db)
    })?;
    let host_interrupted = store.recover_tasks()?;
    let index_paths = rebase_codex_indexes(&data, &restore.original_data, &restore.projects, true)?;
    for project in &restore.projects {
        let report = reports
            .get_mut(&project.id)
            .context("partition report missing")?;
        partition::convert(project, report)?;
    }
    let summary = json!({"format":"beaver-project-activation-v1","ready_to_activate":true,
        "data_directory":data,"projects":reports,"retained_tasks_marked":retained.len(),
        "host_tasks_interrupted":host_interrupted,"credentials_converted":credentials_converted,
        "host_codex_index_paths_rewritten":index_paths,"external_tools_to_check":external,
        "source_archive":fs::canonicalize(backup)?,
        "live_model_request_made":false,"default_data_directory_changed":false});
    Ok(Staged {
        _lock: lock,
        store,
        marker,
        activation,
        summary,
    })
}

fn activate(backup: &Path, prepared: &Path, tool_paths: Option<&Path>) -> Result<Value> {
    let mut staged = stage(backup, prepared)?;
    let (tools, activated_at) = migration_activation::bind_tools(&mut staged.store, tool_paths)?;
    let mut result = staged.summary;
    result["activated_at"] = json!(activated_at);
    result["tools"] = tools;
    drop(staged.store);
    migration_bundle::verify(backup)?;
    migration_activation::write_receipt(&staged.activation, &staged.marker, &result)?;
    Ok(result)
}

impl Staged {
    #[cfg(test)]
    pub(crate) fn summary(&self) -> &Value {
        &self.summary
    }
}
