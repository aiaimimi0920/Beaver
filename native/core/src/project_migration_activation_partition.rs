//! Check and convert one partition inside a pending recovery copy; nothing is scheduled here.
use crate::{
    files::safe_path,
    journal::{FileOperation, Journal, OperationState},
    migration_bundle::RestoredProject,
    migration_import::{relative, table_columns},
    project_storage::ProjectStore,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionReport {
    pub tasks: usize,
    pub operations: usize,
    pub session_indexes: usize,
    pub tasks_interrupted: usize,
}

fn task_workspace(root: &Path, id: &str, task: &Value, prefix: &str) -> Result<()> {
    ensure!(task["id"] == id, "{prefix} identifier mismatch: {id}");
    let workspace = task["workspace"]
        .as_str()
        .with_context(|| format!("{prefix} workspace missing: {id}"))?;
    ensure!(
        workspace == format!(".beaver/workspaces/{id}"),
        "{prefix} workspace is not project-relative: {id}"
    );
    ensure!(
        safe_path(root, workspace)?.is_dir(),
        "{prefix} workspace missing from the partition: {id}"
    );
    Ok(())
}

fn is_index(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("state_"))
        .and_then(|name| name.strip_suffix(".sqlite"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn check_index(root: &Path, home: &Path, database: &Path) -> Result<()> {
    let connection = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.execute_batch("PRAGMA trusted_schema=OFF;")?;
    let columns = table_columns(&connection, "threads")?;
    ensure!(
        ["id", "rollout_path", "cwd"]
            .iter()
            .all(|name| columns.iter().any(|column| column == name)),
        "unsupported Codex state index schema"
    );
    for (table, column, file) in [
        ("threads", "rollout_path", true),
        ("threads", "cwd", false),
        ("project_roots", "path", false),
        ("rollout_migration_skipped_rollouts", "rollout_path", true),
    ] {
        if !table_columns(&connection, table)?
            .iter()
            .any(|name| name == column)
        {
            continue;
        }
        let mut query = connection.prepare(&format!("SELECT {column} FROM {table}"))?;
        for value in query.query_map([], |row| row.get::<_, String>(0))? {
            let value = value?;
            let base = if file { home } else { root };
            let inside = relative(base, &value)?
                .with_context(|| format!("Codex {table}.{column} left the partition"))?;
            let path = if inside.is_empty() {
                base.to_path_buf()
            } else {
                safe_path(base, &inside)?
            };
            ensure!(
                if file { path.is_file() } else { path.is_dir() },
                "Codex {table}.{column} target missing from the partition"
            );
        }
    }
    Ok(())
}

/// Codex indexes were rewritten during partitioning; verify every path now lives in the
/// partition before the copy can be opened by the desktop.
fn check_indexes(root: &Path) -> Result<usize> {
    let codex = root.join(".beaver/workspaces/.codex");
    if !codex.is_dir() {
        return Ok(0);
    }
    let mut checked = 0;
    for home in fs::read_dir(&codex)? {
        let home = home?.path();
        if !home.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&home)? {
            let database = entry?.path();
            if is_index(&database) {
                check_index(root, &home, &database)?;
                checked += 1;
            }
        }
    }
    Ok(checked)
}

/// Read-only validation of a partition built by `partition-projects`.
pub(crate) fn check(project: &RestoredProject) -> Result<PartitionReport> {
    let store = ProjectStore::open_partition(&project.restored_path, &project.id)?;
    let root = store.project_root().to_path_buf();
    let entity: Value = store
        .store()
        .get("project", &project.id)?
        .context("partition lacks its project entity")?;
    // A rerun after an interrupted activation sees the already converted registration.
    ensure!(
        entity["id"] == project.id
            && (entity["path"] == project.original_path
                || entity["path"].as_str() == root.to_str()),
        "partition project registration changed since partitioning"
    );
    crate::migration_activation::check_tree(&root)?;
    let tasks: BTreeMap<String, Value> = store.store().list_with_ids("task")?.into_iter().collect();
    for (id, task) in &tasks {
        ensure!(
            task["projectId"] == project.id,
            "partition task belongs elsewhere: {id}"
        );
        task_workspace(&root, id, task, "partition task")?;
    }
    let operations = store.store().list::<FileOperation>("operation")?;
    for operation in &operations {
        let task = tasks
            .get(&operation.task_id)
            .context("partition operation task missing")?;
        ensure!(
            operation.project_id == project.id
                && task["projectId"] == operation.project_id
                && operation.task_after["projectId"] == operation.project_id,
            "partition operation project mismatch"
        );
        task_workspace(
            &root,
            &operation.task_id,
            &operation.task_after,
            "operation task",
        )?;
    }
    Ok(PartitionReport {
        tasks: tasks.len(),
        operations: operations.len(),
        session_indexes: check_indexes(&root)?,
        tasks_interrupted: 0,
    })
}

/// Point the partition's own registration at itself, finish file journals and interrupt
/// whatever was running when the archive was taken. Runs after every check has passed.
pub(crate) fn convert(project: &RestoredProject, report: &mut PartitionReport) -> Result<()> {
    let mut store = ProjectStore::open_partition(&project.restored_path, &project.id)?;
    let root = store.project_root().to_string_lossy().into_owned();
    let (id, original) = (project.id.clone(), project.original_path.clone());
    store.store_mut().transaction(|db| {
        let converted: i64 = db.query_row(
            "SELECT count(*) FROM entities WHERE kind='project' AND id=? AND json_extract(value,'$.path')=?",
            params![id, root],
            |row| row.get(0),
        )?;
        let updated = db.execute(
            "UPDATE entities SET value=json_set(value,'$.path',?) WHERE kind='project' AND id=? AND json_extract(value,'$.path')=?",
            params![root, id, original],
        )?;
        ensure!(
            updated + usize::try_from(converted)? == 1,
            "partition project registration changed during activation"
        );
        Ok(())
    })?;
    let runtime = store.into_runtime();
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("partition database lock unavailable"))?;
    Journal::new(&mut store, &files).recover()?;
    ensure!(
        !store
            .list::<FileOperation>("operation")?
            .iter()
            .any(|operation| matches!(
                operation.state,
                OperationState::Applying | OperationState::Aborting
            )),
        "partition file recovery remains blocked"
    );
    report.tasks_interrupted = store.recover_tasks()?;
    Ok(())
}
