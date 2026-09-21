//! Inspect private SQLite indexes on temporary copies; JSONL and unknown columns stay opaque.
use crate::{
    data_backup::Manifest,
    files::safe_path,
    migration_bundle::Project,
    project_migration_files::FileIndex,
    project_migration_ownership::Entity,
    project_migration_references::{Checks, Issue},
    project_migration_session_paths::SessionPaths,
};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathRecord {
    pub index_path: String,
    pub table: &'static str,
    pub column: &'static str,
    pub rowid: i64,
    pub target_archive_path: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Sessions {
    pub indexes: usize,
    #[serde(flatten)]
    pub checks: Checks,
    pub records: Vec<PathRecord>,
}

fn is_index(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 3
        && parts[0] == "codex"
        && parts[2]
            .strip_prefix("state_")
            .and_then(|s| s.strip_suffix(".sqlite"))
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

fn columns(connection: &Connection, table: &str) -> Result<Option<Vec<String>>, &'static str> {
    let mut objects = connection
        .prepare("SELECT type FROM sqlite_schema WHERE name=?")
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
    let mut rows = objects
        .query([table])
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
    let Some(row) = rows
        .next()
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?
    else {
        return Ok(None);
    };
    if row.get::<_, String>(0).ok().as_deref() != Some("table") {
        return Err("SESSION_INDEX_SCHEMA_UNSUPPORTED");
    }
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
    Ok(Some(columns))
}

fn inspect_copy(
    data: &Path,
    database: &str,
    index: &FileIndex<'_>,
    paths: &SessionPaths<'_>,
    source: &Entity,
) -> Result<Sessions, &'static str> {
    index
        .owner(database)
        .map_err(|_| "FILE_OWNER_UNRESOLVED")?
        .ok_or("FILE_OWNER_UNRESOLVED")?;
    let temp = tempfile::tempdir().map_err(|_| "SESSION_INDEX_COPY_FAILED")?;
    for suffix in ["", "-wal", "-shm"] {
        let entry = match index.entry(&format!("{database}{suffix}")) {
            Ok(entry) => entry,
            Err("FILE_NOT_FOUND") if !suffix.is_empty() => continue,
            Err(reason) => return Err(reason),
        };
        if entry.sha256.is_none() {
            return Err("FILE_TYPE_MISMATCH");
        }
        let from = safe_path(data, &entry.path).map_err(|_| "SESSION_INDEX_COPY_FAILED")?;
        fs::copy(from, temp.path().join(format!("state.sqlite{suffix}")))
            .map_err(|_| "SESSION_INDEX_COPY_FAILED")?;
    }
    let connection = Connection::open_with_flags(
        temp.path().join("state.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| "SESSION_INDEX_UNREADABLE")?;
    connection
        .execute_batch("PRAGMA trusted_schema=OFF;")
        .map_err(|_| "SESSION_INDEX_UNREADABLE")?;
    let integrity: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| "SESSION_INDEX_UNREADABLE")?;
    if integrity != "ok" {
        return Err("SESSION_INDEX_INTEGRITY_FAILED");
    }
    let required = columns(&connection, "threads")?.ok_or("SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
    if !["id", "rollout_path", "cwd"]
        .iter()
        .all(|name| required.iter().any(|column| column == name))
    {
        return Err("SESSION_INDEX_SCHEMA_UNSUPPORTED");
    }
    let mut report = Sessions::default();
    for (table, column) in [
        ("threads", "rollout_path"),
        ("threads", "cwd"),
        ("project_roots", "path"),
        ("rollout_migration_skipped_rollouts", "rollout_path"),
    ] {
        let Some(columns) = columns(&connection, table)? else {
            continue;
        };
        if !columns.iter().any(|name| name == column) {
            return Err("SESSION_INDEX_SCHEMA_UNSUPPORTED");
        }
        let mut query = connection
            .prepare(&format!(
                "SELECT rowid,{column} FROM {table} ORDER BY rowid"
            ))
            .map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
        let mut rows = query.query([]).map_err(|_| "SESSION_INDEX_UNREADABLE")?;
        while let Some(row) = rows.next().map_err(|_| "SESSION_INDEX_UNREADABLE")? {
            let rowid: i64 = row.get(0).map_err(|_| "SESSION_INDEX_SCHEMA_UNSUPPORTED")?;
            let result = row
                .get::<_, String>(1)
                .map_err(|_| "INVALID_FILE_REFERENCE")
                .and_then(|value| {
                    paths.check(
                        database,
                        &value,
                        column == "rollout_path",
                        table == "rollout_migration_skipped_rollouts",
                    )
                });
            report.checks.checked += 1;
            match result {
                Ok(target_archive_path) => report.records.push(PathRecord {
                    index_path: source.id.clone(),
                    table,
                    column,
                    rowid,
                    target_archive_path,
                }),
                Err(reason) => report.checks.issues.push(Issue::new(
                    source,
                    &format!("/{table}/{rowid}/{column}"),
                    reason,
                )),
            }
        }
    }
    Ok(report)
}

pub(crate) fn inspect(
    data: &Path,
    application: &Manifest,
    projects: &[Project],
    index: &FileIndex<'_>,
) -> Sessions {
    let paths = SessionPaths::new(application, projects, index);
    let mut report = Sessions::default();
    for entry in application
        .entries
        .iter()
        .filter(|entry| is_index(&entry.path))
    {
        report.indexes += 1;
        let source = Entity {
            kind: "codex-state-index".into(),
            id: format!("application/data/{}", entry.path),
            value: None,
        };
        match inspect_copy(data, &entry.path, index, &paths, &source) {
            Ok(mut result) => {
                report.checks.checked += result.checks.checked;
                report.checks.issues.append(&mut result.checks.issues);
                report.records.append(&mut result.records);
            }
            Err(reason) => report.checks.issues.push(Issue::new(&source, "/", reason)),
        }
    }
    report
}
