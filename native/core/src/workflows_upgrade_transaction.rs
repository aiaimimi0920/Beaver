//! Durable host-store receipts and verified content backups; runtime metadata commits last.
use super::{package, trust, KIND};
use crate::{files::{file_hash, safe_path, Change, Files}, store::Store, workflows::RUNTIME_FILE};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Receipt {
    pub operation_id: String,
    pub created_at: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub status: String,
    pub changes: Vec<Change>,
    pub transitional_runtime: String,
    pub runtime: Value,
    pub previous_project_npr: Value,
}

pub(crate) fn pending(connection: &rusqlite::Connection, project: &str) -> Result<bool> {
    let mut query = connection.prepare("SELECT value FROM entities WHERE kind=?")?;
    for row in query.query_map([KIND], |row| row.get::<_, String>(0))? {
        let receipt: Receipt = serde_json::from_str(&row?)?;
        if receipt.project_id == project && !matches!(receipt.status.as_str(), "completed" | "rolledBack") { return Ok(true); }
    }
    Ok(false)
}

fn actual(root: &Path, relative: &str) -> Result<Option<String>> {
    file_hash(&safe_path(root, relative)?)
}

pub(super) fn prepare(
    files: &Files, root: &Path, project: &Value, task_id: Option<String>, plan: &trust::Plan,
) -> Result<Receipt> {
    let mut original: Vec<_> = plan.changed.iter().filter(|name| plan.before.contains_key(*name)).cloned().collect();
    original.push(RUNTIME_FILE.into());
    let before = files.capture_paths(root, original, Some(16 * 1024 * 1024))?;
    ensure!(before.get(RUNTIME_FILE) == Some(&plan.runtime_hash), "Runtime changed during backup");
    for name in &plan.changed {
        ensure!(before.get(name) == plan.before.get(name), "Dependency changed during backup: {name}");
    }
    let temporary = tempfile::tempdir()?;
    let mut targets = Vec::new();
    for name in &plan.changed {
        if let Some(bytes) = plan.after.get(name) {
            super::super::write_new(temporary.path(), name, bytes)?;
            targets.push(name.clone());
        }
    }
    super::super::write_new(temporary.path(), RUNTIME_FILE, &serde_json::to_vec_pretty(&plan.runtime)?)?;
    targets.push(RUNTIME_FILE.into());
    let after = files.capture_paths(temporary.path(), targets, Some(16 * 1024 * 1024))?;
    let mut transitional = plan.runtime.clone();
    transitional["packages"]["npr-characters"]["sourceCommit"] = json!(plan.source_commit);
    transitional["packages"]["npr-characters"]["status"] = json!("upgrading");
    super::super::write_new(temporary.path(), "upgrading.json", &serde_json::to_vec_pretty(&transitional)?)?;
    let transient = files.capture_paths(temporary.path(), vec!["upgrading.json".into()], Some(65536))?;
    #[cfg(unix)]
    std::fs::File::open(files.blob(&transient["upgrading.json"])? .parent().context("Blob directory missing")?)?.sync_all()?;
    let mut names = plan.changed.clone();
    names.push(RUNTIME_FILE.into());
    let changes = names.into_iter().map(|name| Change {
        before: before.get(&name).cloned(), after: after.get(&name).cloned(), path: name,
    }).collect();
    Ok(Receipt { operation_id: uuid::Uuid::new_v4().to_string(), created_at: chrono::Utc::now().to_rfc3339(),
        project_id: project["id"].as_str().context("Project id missing")?.into(),
        task_id, status: "applying".into(), changes,
        transitional_runtime: transient["upgrading.json"].clone(), runtime: plan.runtime.clone(),
        previous_project_npr: project["npr"].clone() })
}

fn save_project(store: &Store, receipt: &Receipt, restored: bool) -> Result<()> {
    if receipt.task_id.is_none() {
        let mut project: Value = store.get("project", &receipt.project_id)?.context("Project missing")?;
        project["npr"] = if restored { receipt.previous_project_npr.clone() }
            else { json!({"status":"ready","runtime":receipt.runtime}) };
        store.put("project", &receipt.project_id, &project)?;
    }
    Ok(())
}

pub(super) fn apply(store: &Store, files: &Files, root: &Path, receipt: &mut Receipt) -> Result<()> {
    // FULL-synchronous project SQLite owns the receipt; agents cannot forge file-path authority.
    store.put(KIND, &receipt.operation_id, receipt)?;
    let result = (|| -> Result<()> {
        let runtime = receipt.changes.last().context("Missing runtime transition")?;
        ensure!(actual(root, RUNTIME_FILE)? == runtime.before, "Runtime CAS failed");
        files.write(root, RUNTIME_FILE, Some(&receipt.transitional_runtime))?;
        for change in &receipt.changes {
            if change.path == RUNTIME_FILE { continue; }
            ensure!(actual(root, &change.path)? == change.before, "Dependency CAS failed: {}", change.path);
            files.write(root, &change.path, change.after.as_deref())?;
        }
        package::require_current(root, &receipt.runtime)?;
        ensure!(actual(root, RUNTIME_FILE)?.as_ref() == Some(&receipt.transitional_runtime), "Runtime CAS failed");
        files.write(root, RUNTIME_FILE, runtime.after.as_deref())?;
        let (current, _) = trust::identity(root)?;
        package::require_current(root, &current)?;
        save_project(store, receipt, false)?;
        receipt.status = "completed".into();
        store.put(KIND, &receipt.operation_id, receipt)?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Err(recovery) = rollback(store, files, root, receipt) {
            anyhow::bail!("NPR upgrade {id} failed: {error}; rollback blocked: {recovery}. Use project.npr.rollbackUpgrade after resolving the conflict", id = receipt.operation_id);
        }
        anyhow::bail!("NPR upgrade failed and was rolled back: {error}");
    }
    Ok(())
}

pub(super) fn rollback(store: &Store, files: &Files, root: &Path, receipt: &mut Receipt) -> Result<()> {
    let previous_runtime = actual(root, RUNTIME_FILE)?;
    // Preflight every path and backup before restoring anything; preserve concurrent edits.
    for change in &receipt.changes {
        let current = actual(root, &change.path)?;
        ensure!(current == change.before || current == change.after ||
            (change.path == RUNTIME_FILE && current.as_ref() == Some(&receipt.transitional_runtime)),
            "Rollback conflict, file preserved: {}", change.path);
        if let Some(hash) = &change.before {
            ensure!(file_hash(&files.blob(hash)?)?.as_ref() == Some(hash), "Backup missing or damaged: {}", change.path);
        }
    }
    receipt.status = "rollingBack".into();
    store.put(KIND, &receipt.operation_id, receipt)?;
    // Freeze workflow access even when rolling back an otherwise completed operation.
    ensure!(actual(root, RUNTIME_FILE)? == previous_runtime, "Rollback runtime CAS conflict");
    files.write(root, RUNTIME_FILE, Some(&receipt.transitional_runtime))?;
    for change in &receipt.changes {
        if change.path == RUNTIME_FILE { continue; }
        let current = actual(root, &change.path)?;
        if current != change.before {
            ensure!(current == change.after, "Rollback CAS conflict: {}", change.path);
            files.write(root, &change.path, change.before.as_deref())?;
        }
    }
    ensure!(actual(root, RUNTIME_FILE)?.as_ref() == Some(&receipt.transitional_runtime), "Rollback runtime CAS conflict");
    let original = receipt.changes.last().context("Missing runtime backup")?;
    files.write(root, RUNTIME_FILE, original.before.as_deref())?;
    save_project(store, receipt, true)?;
    receipt.status = "rolledBack".into();
    store.put(KIND, &receipt.operation_id, receipt)?;
    Ok(())
}
