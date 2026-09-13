//! Prepare an isolated import; activation remains gated on real session-resume validation.
use crate::{
    files::{file_hash, safe_path, Files},
    journal::{FileOperation, Journal, OperationState},
    legacy_vault, migration_bundle,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use rusqlite::params;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct ImportReceipt {
    pub format: &'static str,
    pub restored_data: std::path::PathBuf,
    pub backup_manifest_sha256: String,
    pub application_manifest_sha256: String,
    pub paths_rewritten: bool,
    pub codex_index_paths_rewritten: usize,
    pub credentials_converted: usize,
    pub credentials_verified: bool,
    pub journals_recovered: usize,
    pub tasks_interrupted: usize,
    pub external_tools_to_check: Vec<String>,
    pub ready_to_activate: bool,
    pub activation_blockers: Vec<&'static str>,
}

// Old roots may no longer exist. Compare lexical components, never string prefixes or
// canonicalize old paths (which could resolve to a different directory after migration).
fn components(value: &str) -> Result<Vec<String>> {
    let value = value.replace('\\', "/");
    let value = if let Some(rest) = value.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else {
        value.strip_prefix("//?/").unwrap_or(&value).to_owned()
    };
    let (prefix, rest) = if let Some(rest) = value.strip_prefix("//") {
        ("//".to_string(), rest)
    } else {
        let bytes = value.as_bytes();
        ensure!(
            bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":/",
            "migration requires an absolute Windows path"
        );
        (value[..2].into(), &value[3..])
    };
    let mut parts = vec![prefix];
    for part in rest.trim_end_matches('/').split('/') {
        ensure!(
            !part.is_empty()
                && !part.ends_with(['.', ' '])
                && !part.contains([':', '\0', '<', '>', '"', '|', '?', '*']),
            "ambiguous migration path component"
        );
        parts.push(part.into());
    }
    ensure!(parts[0] != "//" || parts.len() >= 3, "invalid UNC path");
    Ok(parts)
}

fn relative(old_root: &Path, value: &str) -> Result<Option<String>> {
    let root = components(old_root.to_str().context("non-UTF-8 data root")?)?;
    let value = components(value)?;
    if value.len() < root.len()
        || !root
            .iter()
            .zip(&value)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        return Ok(None);
    }
    Ok(Some(value[root.len()..].join("/")))
}

fn task_path(task: &mut Value, id: &str, mapping: &migration_bundle::RestoreReceipt) -> Result<()> {
    ensure!(uuid::Uuid::parse_str(id).is_ok(), "invalid task identifier");
    ensure!(task["id"].as_str() == Some(id), "task identifier mismatch");
    ensure!(
        mapping.projects.iter().any(|p| task["projectId"] == p.id),
        "task references an unarchived project"
    );
    let workspace = task["workspace"]
        .as_str()
        .context("task workspace missing")?;
    let expected = format!("workspaces/{id}");
    ensure!(
        relative(&mapping.original_data, workspace)?
            .is_some_and(|path| path.eq_ignore_ascii_case(&expected)),
        "task workspace is outside its managed directory"
    );
    let restored = safe_path(&mapping.restored_data, &expected)?;
    ensure!(restored.is_dir(), "archived task workspace is missing");
    task["workspace"] = json!(restored);
    Ok(())
}

fn tool_path(
    value: &mut Value,
    slot: &str,
    mapping: &migration_bundle::RestoreReceipt,
    external: &mut BTreeSet<String>,
) -> Result<()> {
    let path = value.as_str().context("invalid stored tool path")?;
    if path.is_empty() {
        return Ok(());
    }
    // Bare command names are also external, to be re-detected on the destination host.
    if !Path::new(path).is_absolute() {
        external.insert(slot.into());
        return Ok(());
    }
    if let Some(relative) = relative(&mapping.original_data, path)? {
        ensure!(
            relative
                .split('/')
                .next()
                .is_some_and(|s| s.eq_ignore_ascii_case("tools")),
            "stored tool is inside data but outside managed tools"
        );
        let restored = safe_path(&mapping.restored_data, &relative)?;
        ensure!(restored.is_file(), "archived managed tool is missing");
        *value = json!(restored);
    } else {
        external.insert(slot.into());
    }
    Ok(())
}

fn map_index_path(
    value: &str,
    original: &Path,
    restored: &Path,
    projects: &[migration_bundle::RestoredProject],
) -> Result<String> {
    if let Some(relative) = relative(original, value)? {
        let path = if relative.is_empty() {
            restored.to_path_buf()
        } else {
            safe_path(restored, &relative)?
        };
        return Ok(path.to_string_lossy().into_owned());
    }
    for project in projects {
        if let Some(relative) = relative(Path::new(&project.original_path), value)? {
            let path = if relative.is_empty() {
                project.restored_path.clone()
            } else {
                safe_path(&project.restored_path, &relative)?
            };
            return Ok(path.to_string_lossy().into_owned());
        }
    }
    Ok(value.to_owned())
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns)
}

/// Codex stores its rollout and cwd paths in private SQLite indexes. Keep JSONL opaque,
/// but update only verified path columns so thread/resume can locate copied history.
pub(crate) fn rebase_codex_indexes(
    data: &Path,
    original: &Path,
    projects: &[migration_bundle::RestoredProject],
    rewrite: bool,
) -> Result<usize> {
    let codex = safe_path(data, "codex")?;
    let mut databases = Vec::new();
    if codex.is_dir() {
        for task in fs::read_dir(&codex)? {
            let task = task?;
            let name = task
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid Codex home name"))?;
            let home = safe_path(&codex, &name)?;
            if !home.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&home)? {
                let entry = entry?;
                let file = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("invalid Codex database name"))?;
                if file
                    .strip_prefix("state_")
                    .and_then(|s| s.strip_suffix(".sqlite"))
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                {
                    databases.push(safe_path(&home, &file)?);
                }
            }
        }
    }
    let mut changed = 0;
    for database in databases {
        let mut connection = Connection::open_with_flags(
            &database,
            if rewrite {
                OpenFlags::SQLITE_OPEN_READ_WRITE
            } else {
                OpenFlags::SQLITE_OPEN_READ_ONLY
            },
        )?;
        let transaction = connection.transaction()?;
        let columns = table_columns(&transaction, "threads")?;
        ensure!(
            ["id", "rollout_path", "cwd"]
                .iter()
                .all(|name| columns.iter().any(|column| column == name)),
            "unsupported Codex state index schema"
        );
        for (table, column) in [
            ("threads", "rollout_path"),
            ("threads", "cwd"),
            ("project_roots", "path"),
            ("rollout_migration_skipped_rollouts", "rollout_path"),
        ] {
            let columns = table_columns(&transaction, table)?;
            if !columns.iter().any(|name| name == column) {
                continue;
            }
            let mut query = transaction.prepare(&format!("SELECT rowid,{column} FROM {table}"))?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(query);
            for (rowid, old) in rows {
                let next = map_index_path(&old, original, data, projects)?;
                if table == "threads" && column == "rollout_path" {
                    let home = database.parent().context("Codex index has no home")?;
                    let relative = relative(home, &next)?
                        .context("Codex rollout is outside its copied home")?;
                    ensure!(
                        safe_path(home, &relative)?.is_file(),
                        "Codex rollout history missing after path migration"
                    );
                }
                if table == "threads" && column == "cwd" {
                    let managed = relative(data, &next)?.is_some()
                        || projects
                            .iter()
                            .any(|p| relative(&p.restored_path, &next).ok().flatten().is_some());
                    ensure!(
                        managed && Path::new(&next).is_dir(),
                        "Codex thread cwd is outside the restored directories or missing"
                    );
                }
                if next != old {
                    ensure!(rewrite, "Codex index contains an unconverted path");
                    transaction.execute(
                        &format!("UPDATE {table} SET {column}=? WHERE rowid=?"),
                        params![next, rowid],
                    )?;
                    changed += 1;
                }
            }
        }
        transaction.commit()?;
    }
    Ok(changed)
}

/// No production/source Store is opened, and no pending marker is ever removed here.
pub fn prepare(backup: &Path, destination: &Path) -> Result<ImportReceipt> {
    ensure!(
        cfg!(windows),
        "import preparation currently requires Windows"
    );
    let mapping = migration_bundle::restore(backup, destination)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(safe_path(&mapping.restored_data, ".beaver-native.lock")?)?;
    lock.try_lock_exclusive()
        .context("import data directory is in use")?;
    let mut store = Store::open(&mapping.restored_data)?;
    let mut external = BTreeSet::new();
    let rows = store.transaction(|db| {
        let mut query = db.prepare(
            "SELECT kind,id,value FROM entities WHERE kind IN ('project','task','operation','settings','toolSetup') ORDER BY rowid",
        )?;
        let rows = query.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    })?;
    let tasks: BTreeMap<_, _> = rows
        .iter()
        .filter(|(kind, _, _)| kind == "task")
        .map(|(_, id, raw)| Ok((id.clone(), serde_json::from_str::<Value>(raw)?)))
        .collect::<Result<_>>()?;
    let mut updates = Vec::new();
    let mut pending_journals = 0;
    for (kind, id, raw) in rows {
        let mut value: Value = serde_json::from_str(&raw)?;
        ensure!(value.is_object(), "invalid migration entity");
        match kind.as_str() {
            "project" => {
                let project = mapping
                    .projects
                    .iter()
                    .find(|p| p.id == id)
                    .context("project missing from restore mapping")?;
                ensure!(
                    value["id"] == id && value["path"] == project.original_path,
                    "project mapping changed"
                );
                value["path"] = json!(project.restored_path);
            }
            "task" => task_path(&mut value, &id, &mapping)?,
            "operation" => {
                let operation: FileOperation = serde_json::from_value(value.clone())?;
                ensure!(operation.id == id, "operation identifier mismatch");
                let current = tasks
                    .get(&operation.task_id)
                    .context("operation task missing")?;
                ensure!(
                    current["projectId"] == operation.project_id
                        && operation.task_after["projectId"] == operation.project_id,
                    "operation project mismatch"
                );
                task_path(&mut value["taskAfter"], &operation.task_id, &mapping)?;
                if matches!(
                    operation.state,
                    OperationState::Applying | OperationState::Aborting
                ) {
                    pending_journals += 1;
                }
            }
            "settings" if id == "main" => {
                ensure!(value["tools"].is_object(), "invalid stored tools");
                for slot in ["codex", "godot", "blender", "node"] {
                    tool_path(&mut value["tools"][slot], slot, &mapping, &mut external)?;
                }
            }
            "toolSetup" if id == "main" => {
                if let Some(steps) = value["steps"].as_array_mut() {
                    for step in steps {
                        let slot = step["name"]
                            .as_str()
                            .context("invalid setup tool name")?
                            .to_string();
                        if let Some(result) =
                            step.get_mut("result").filter(|result| !result.is_null())
                        {
                            ensure!(result.is_object(), "invalid setup result");
                            tool_path(&mut result["path"], &slot, &mapping, &mut external)?;
                            result["available"] = json!(false);
                        }
                    }
                }
                value = crate::tool_setup::recover(value);
            }
            _ => continue,
        }
        updates.push((kind, id, raw, value));
    }
    let credentials = legacy_vault::prepare(&store, &mapping.restored_data.join("Local State"))?;
    let codex_paths_rewritten = rebase_codex_indexes(
        &mapping.restored_data,
        &mapping.original_data,
        &mapping.projects,
        true,
    )?;
    let converted = store.transaction(|db| {
        for (kind, id, raw, value) in &updates {
            ensure!(
                db.execute(
                    "UPDATE entities SET value=? WHERE kind=? AND id=? AND value=?",
                    params![value.to_string(), kind, id, raw]
                )? == 1,
                "entity changed during import"
            );
        }
        credentials.apply(db)
    })?;
    let files = Files::new(mapping.restored_data.clone());
    Journal::new(&mut store, &files).recover()?;
    ensure!(
        !store
            .list::<FileOperation>("operation")?
            .iter()
            .any(|operation| matches!(
                operation.state,
                OperationState::Applying | OperationState::Aborting
            )),
        "file recovery remains blocked; import copy is still pending"
    );
    let interrupted = store.recover_tasks()?;
    drop(store);
    migration_bundle::verify(backup)?;
    let receipt = ImportReceipt {
        format: "beaver-migration-prepared-v1",
        restored_data: mapping.restored_data,
        backup_manifest_sha256: file_hash(&backup.join("BEAVER-MIGRATION.json"))?.context("backup manifest missing")?,
        application_manifest_sha256: file_hash(&backup.join("application/BEAVER-BACKUP.json"))?.context("application manifest missing")?,
        paths_rewritten: true,
        codex_index_paths_rewritten: codex_paths_rewritten,
        credentials_converted: converted,
        credentials_verified: true,
        journals_recovered: pending_journals,
        tasks_interrupted: interrupted,
        external_tools_to_check: external.into_iter().collect(),
        ready_to_activate: false,
        activation_blockers: vec![
            "run activate-import with the original verified bundle to check destination tools and enable this copy",
        ],
    };
    let root = receipt
        .restored_data
        .parent()
        .context("import root missing")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("IMPORT.json"))?;
    file.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
    file.sync_all()?;
    Ok(receipt)
}

#[cfg(all(test, windows))]
#[path = "migration_import_tests.rs"]
mod tests;
