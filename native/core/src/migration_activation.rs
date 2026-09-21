//! Explicitly enable an already prepared copy after checking its actual destination state.
use crate::{
    files::{file_hash, safe_path},
    journal::{FileOperation, OperationState},
    migration_bundle, preferences,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

pub(crate) fn read_json(path: &Path) -> Result<Value> {
    ensure!(
        fs::metadata(path)?.len() <= 1024 * 1024,
        "migration receipt or tool configuration too large"
    );
    Ok(serde_json::from_reader(File::open(path)?)?)
}

pub(crate) fn check_tree(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("invalid prepared filename"))?;
        let path = safe_path(root, &name)?;
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            metadata.is_file() || metadata.is_dir(),
            "unsupported prepared file type"
        );
        if metadata.is_dir() {
            check_tree(&path)?;
        }
    }
    Ok(())
}

fn check_task(
    data: &Path,
    task: &Value,
    id: &str,
    projects: &[migration_bundle::Project],
) -> Result<()> {
    ensure!(
        uuid::Uuid::parse_str(id).is_ok() && task["id"] == id,
        "invalid imported task identifier"
    );
    ensure!(
        projects.iter().any(|p| task["projectId"] == p.id),
        "imported task project missing"
    );
    let path = task["workspace"]
        .as_str()
        .context("imported workspace missing")?;
    ensure!(
        fs::canonicalize(path)? == safe_path(data, &format!("workspaces/{id}"))?,
        "imported task points outside its copied workspace"
    );
    Ok(())
}

fn validate_state(
    store: &mut Store,
    root: &Path,
    manifest: &migration_bundle::Manifest,
) -> Result<()> {
    let data = safe_path(root, "data")?;
    let projects = store.list::<Value>("project")?;
    ensure!(
        projects.len() == manifest.projects.len(),
        "prepared project set differs from archive"
    );
    for project in &manifest.projects {
        let value = store
            .get::<Value>("project", &project.id)?
            .context("prepared project missing")?;
        let target = safe_path(root, &format!("projects/{}", project.id))?;
        ensure!(
            value["id"] == project.id
                && fs::canonicalize(
                    value["path"]
                        .as_str()
                        .context("prepared project path missing")?
                )? == target,
            "prepared project points outside its restored directory"
        );
        ensure!(
            target.join("project.godot").is_file(),
            "prepared Godot project is missing"
        );
        check_tree(&target)?;
    }
    // Recheck links and unsupported files before handing ownership to the desktop.
    // Do not hash our exclusively locked .beaver-native.lock on Windows.
    check_tree(&data)?;
    let restored = manifest
        .projects
        .iter()
        .map(|project| {
            Ok(migration_bundle::RestoredProject {
                id: project.id.clone(),
                original_path: project.stored_path.clone(),
                restored_path: safe_path(root, &format!("projects/{}", project.id))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    crate::migration_import::rebase_codex_indexes(&data, &data, &restored, false)?;
    let tasks = store.transaction(|db| {
        let mut statement = db.prepare("SELECT id,value FROM entities WHERE kind='task'")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|(id, raw)| Ok((id, serde_json::from_str::<Value>(&raw)?)))
            .collect::<Result<BTreeMap<_, _>>>()
    })?;
    for (id, task) in &tasks {
        check_task(&data, task, id, &manifest.projects)?;
        ensure!(
            !matches!(task["status"].as_str(), Some("running" | "queued")),
            "prepared task was started before activation"
        );
    }
    for operation in store.list::<FileOperation>("operation")? {
        ensure!(
            !matches!(
                operation.state,
                OperationState::Applying | OperationState::Aborting
            ),
            "file recovery still blocked"
        );
        let task = tasks
            .get(&operation.task_id)
            .context("operation task missing")?;
        ensure!(
            task["projectId"] == operation.project_id
                && operation.task_after["projectId"] == operation.project_id,
            "operation project mismatch"
        );
        check_task(
            &data,
            &operation.task_after,
            &operation.task_id,
            &manifest.projects,
        )?;
    }
    Ok(())
}

pub fn activate(backup: &Path, prepared: &Path, tool_paths: Option<&Path>) -> Result<Value> {
    ensure!(
        cfg!(windows),
        "migration activation currently requires Windows"
    );
    let root = fs::canonicalize(prepared)?;
    migration_bundle::ensure_activated(root.parent().context("prepared root has no parent")?)?;
    let marker = safe_path(&root, ".beaver-migration-pending")?;
    ensure!(
        marker.is_file(),
        "prepared directory is not pending activation"
    );
    let activation = safe_path(&root, "ACTIVATION.json")?;
    ensure!(
        !activation.try_exists()?,
        "activation receipt already exists; inspect the existing result"
    );
    let receipt = read_json(&safe_path(&root, "IMPORT.json")?)?;
    ensure!(
        receipt["format"] == "beaver-migration-prepared-v1"
            && receipt["paths_rewritten"] == true
            && receipt["credentials_verified"] == true,
        "import preparation is not complete"
    );
    let manifest = migration_bundle::verify(backup)?;
    for (field, relative) in [
        ("backup_manifest_sha256", "BEAVER-MIGRATION.json"),
        (
            "application_manifest_sha256",
            "application/BEAVER-BACKUP.json",
        ),
    ] {
        ensure!(
            receipt[field].as_str() == file_hash(&safe_path(backup, relative)?)?.as_deref(),
            "archive does not match this prepared copy"
        );
    }
    let data = safe_path(&root, "data")?;
    ensure!(
        fs::canonicalize(
            receipt["restored_data"]
                .as_str()
                .context("prepared data path missing")?
        )? == data,
        "prepared copy moved since preparation"
    );
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(safe_path(&data, ".beaver-native.lock")?)?;
    lock.try_lock_exclusive()
        .context("prepared data directory is in use")?;
    let mut store = Store::open(&data)?;
    validate_state(&mut store, &root, &manifest)?;
    let (detected, now) = bind_tools(&mut store, tool_paths)?;
    drop(store);
    migration_bundle::verify(backup)?;
    let result = json!({"format":"beaver-migration-activation-v1","ready_to_activate":true,"activated_at":now,
        "data_directory":data,"tools":detected,"source_archive":fs::canonicalize(backup)?,
        "live_model_request_made":false,"default_data_directory_changed":false});
    write_receipt(&activation, &marker, &result)?;
    Ok(result)
}

/// The receipt is durable before the pending marker disappears.
pub(crate) fn write_receipt(activation: &Path, marker: &Path, result: &Value) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(activation)?;
    file.write_all(&serde_json::to_vec_pretty(result)?)?;
    file.sync_all()?;
    fs::remove_file(marker)
        .context("activation receipt saved but pending marker could not be removed")?;
    Ok(())
}

/// Detect destination tools against the copied settings and record the outcome in the copy.
pub(crate) fn bind_tools(store: &mut Store, tool_paths: Option<&Path>) -> Result<(Value, String)> {
    let secrets = preferences::CAPABILITIES
        .into_iter()
        .chain(["cloud"])
        .map(|slot| preferences::key(&store, &preferences::SystemVault, slot))
        .collect::<Result<Vec<_>>>()?;
    let mut settings = preferences::read(
        &store,
        serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
    )?;
    if let Some(path) = tool_paths {
        settings["tools"] = read_json(path)?;
    }
    preferences::validate(settings.clone())?;
    let detected = crate::tools::detect(&settings["tools"], &secrets)?;
    let tools = detected
        .as_array()
        .context("invalid tool detection result")?;
    for name in ["codex", "godot"] {
        ensure!(
            tools
                .iter()
                .any(|tool| tool["name"] == name && tool["available"] == true),
            "required destination tool unavailable: {name}"
        );
    }
    if settings["mcp"]["godot"] == true {
        ensure!(
            tools
                .iter()
                .any(|tool| tool["name"] == "node" && tool["available"] == true),
            "Godot MCP requires a usable Node installation"
        );
    }
    for tool in tools {
        if tool["available"] == true {
            let name = tool["name"].as_str().context("tool name missing")?;
            settings["tools"][name] = tool["path"].clone();
        }
    }
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let setup = json!({"status":if tools.iter().all(|tool| tool["available"] == true) {"completed"} else {"failed"},
        "updatedAt":now,"steps":tools.iter().map(|tool| json!({"name":tool["name"],"status":if tool["available"] == true {"ready"} else {"failed"},"result":tool})).collect::<Vec<_>>()});
    store.transaction(|db| {
        for (kind, value) in [("settings", &settings), ("toolSetup", &setup)] {
            db.execute("INSERT INTO entities(kind,id,value) VALUES(?,'main',?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value", rusqlite::params![kind,value.to_string()])?;
        }
        Ok(())
    })?;
    Ok((detected, now))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn fixture() -> Result<(
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        String,
    )> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        let game = temp.path().join("game");
        let id = uuid::Uuid::new_v4().to_string();
        fs::create_dir(&game)?;
        fs::write(game.join("project.godot"), "config_version=5")?;
        let store = Store::open(&source)?;
        store.put("project", &id, &json!({"id":id,"path":game}))?;
        drop(store);
        let backup = temp.path().join("backup");
        migration_bundle::create(&source, &backup)?;
        let target = temp.path().join("prepared");
        crate::migration_import::prepare(&backup, &target)?;
        Ok((temp, backup, target, id))
    }

    #[test]
    fn activation_requires_matching_archive_and_exclusive_ownership() -> Result<()> {
        let (_temp, backup, target, _id) = fixture()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(target.join("data/.beaver-native.lock"))?;
        lock.try_lock_exclusive()?;
        assert!(activate(&backup, &target, None)
            .unwrap_err()
            .to_string()
            .contains("in use"));
        drop(lock);
        let receipt_path = target.join("IMPORT.json");
        let mut receipt = read_json(&receipt_path)?;
        receipt["backup_manifest_sha256"] = json!("different-archive");
        fs::write(&receipt_path, serde_json::to_vec(&receipt)?)?;
        assert!(activate(&backup, &target, None)
            .unwrap_err()
            .to_string()
            .contains("does not match"));
        assert!(target.join(".beaver-migration-pending").is_file());
        assert!(!target.join("ACTIVATION.json").exists());
        Ok(())
    }

    #[test]
    fn changed_project_path_blocks_activation_before_tool_execution() -> Result<()> {
        let (temp, backup, target, id) = fixture()?;
        let store = Store::open(&target.join("data"))?;
        let mut project = store.get::<Value>("project", &id)?.unwrap();
        project["path"] = json!(temp.path().join("game"));
        store.put("project", &id, &project)?;
        drop(store);
        assert!(activate(&backup, &target, None)
            .unwrap_err()
            .to_string()
            .contains("outside"));
        assert!(target.join(".beaver-migration-pending").is_file());
        assert!(!target.join("ACTIVATION.json").exists());
        Ok(())
    }
}
