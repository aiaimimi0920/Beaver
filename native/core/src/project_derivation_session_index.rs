//! Rewrite a private SQLite copy; committed WAL and unknown columns are retained.
use crate::{
    files::safe_path, project_derivation_copy::Prepared, project_derivation_session_paths as paths,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::{fs, path::Path};

fn columns(connection: &Connection, table: &str) -> Result<Option<Vec<String>>> {
    let mut query = connection.prepare("SELECT type FROM sqlite_schema WHERE name=?")?;
    let mut rows = query.query([table])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    ensure!(
        row.get::<_, String>(0)? == "table",
        "unsupported session schema"
    );
    let mut query = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = query
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(names))
}

pub(crate) fn convert(
    prepared: &Prepared,
    source: &Path,
    database: &str,
    output: &Path,
    binding: &Path,
) -> Result<usize> {
    let temp = tempfile::tempdir()?;
    for suffix in ["", "-wal", "-shm"] {
        let name = format!("{database}{suffix}");
        let entry = prepared.entries.iter().find(|entry| entry.path == name);
        let Some(entry) = entry else {
            ensure!(!suffix.is_empty(), "session index missing");
            continue;
        };
        ensure!(entry.sha256.is_some(), "session index must be a file");
        fs::copy(
            safe_path(source, &name)?,
            temp.path().join(format!("state.sqlite{suffix}")),
        )?;
    }
    let mut connection = Connection::open_with_flags(
        temp.path().join("state.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?;
    connection.execute_batch("PRAGMA trusted_schema=OFF;")?;
    let check: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    ensure!(check == "ok", "session index integrity failure");
    let required = columns(&connection, "threads")?.context("missing session threads table")?;
    ensure!(
        ["id", "rollout_path", "cwd"]
            .iter()
            .all(|name| required.iter().any(|column| column == name)),
        "unsupported session threads schema"
    );
    // UPDATE triggers could modify unknown columns or tables, violating opaque preservation.
    let triggers: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='trigger'",
        [],
        |row| row.get(0),
    )?;
    ensure!(triggers == 0, "session index triggers are unsupported");
    let transaction = connection.transaction()?;
    let mut changed = 0;
    for (table, column) in [
        ("threads", "rollout_path"),
        ("threads", "cwd"),
        ("project_roots", "path"),
        ("rollout_migration_skipped_rollouts", "rollout_path"),
    ] {
        let Some(names) = columns(&transaction, table)? else {
            continue;
        };
        ensure!(
            names.iter().any(|name| name == column),
            "unsupported session path schema"
        );
        let records = transaction
            .prepare(&format!(
                "SELECT rowid,{column} FROM {table} ORDER BY rowid"
            ))?
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (rowid, value) in records {
            let target = paths::target(
                prepared,
                database,
                &value,
                column == "rollout_path",
                binding,
            )?;
            ensure!(
                transaction.execute(
                    &format!("UPDATE {table} SET {column}=? WHERE rowid=?"),
                    params![target.to_str().context("non-UTF-8 session target")?, rowid],
                )? == 1,
                "session row disappeared"
            );
            changed += 1;
        }
    }
    transaction.commit()?;
    connection.execute(
        "VACUUM INTO ?",
        [output.to_str().context("non-UTF-8 output")?],
    )?;
    fs::OpenOptions::new()
        .write(true)
        .open(output)?
        .sync_all()?;
    Ok(changed)
}
