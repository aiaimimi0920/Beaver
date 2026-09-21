//! Install composed records into a fresh, metadata-bearing project database.
use crate::{
    files::Files, journal::Journal, project_derivation_database::Staged,
    project_derivation_validation, store::Store, validation,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params_from_iter, types::Value as SqlValue};
use serde_json::Value;
use std::path::Path;

/// The caller owns a new pending target and its exclusive project lock.
pub(crate) fn install(
    staged: &Staged,
    store: &mut Store,
    files: &Files,
    root: &Path,
    project_id: &str,
) -> Result<usize> {
    store.transaction(|target| {
        let count: i64 = target.query_row(
            "SELECT (SELECT count(*) FROM entities)+(SELECT count(*) FROM events)+(SELECT count(*) FROM calls)",
            [], |row| row.get(0),
        )?;
        ensure!(count == 0, "derivation destination is not empty");
        for (table, columns, placeholders) in [
            ("entities", "kind,id,value", "?,?,?"),
            ("events", "seq,task,time,kind,text", "?,?,?,?,?"),
            ("calls", "seq,id,task,project,method,value", "?,?,?,?,?,?"),
        ] {
            let mut query = staged.connection.prepare(&format!("SELECT {columns} FROM {table}"))?;
            let width = query.column_count();
            let mut rows = query.query([])?;
            let mut insert = target.prepare(&format!("INSERT INTO {table}({columns}) VALUES({placeholders})"))?;
            while let Some(row) = rows.next()? {
                let values = (0..width).map(|index| row.get::<_, SqlValue>(index))
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                insert.execute(params_from_iter(values))?;
            }
        }
        let sequence: i64 = staged.connection.query_row(
            "SELECT COALESCE(MAX(seq),0) FROM sqlite_sequence WHERE name='calls'", [], |row| row.get(0),
        )?;
        target.execute("DELETE FROM sqlite_sequence WHERE name='calls'", [])?;
        target.execute("INSERT INTO sqlite_sequence(name,seq) VALUES('calls',?)", [sequence])?;
        Ok(())
    })?;
    let mut project: Value = store
        .get("project", project_id)?
        .context("derived project missing")?;
    project["path"] = serde_json::to_value(root)?;
    store.put("project", project_id, &project)?;
    // Journal uses the project's stored path: bind it to this target before any recovery.
    let mut journal = Journal::new(store, files);
    journal.recover()?;
    ensure!(
        !journal.blocked(project_id)?,
        "derived file recovery remains blocked"
    );
    let interrupted = store.recover_tasks()?;
    validation::repository::recover(store)?;
    project_derivation_validation::validate(&store.connection, project_id)?;
    let busy: i64 = store
        .connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
    ensure!(busy == 0, "derived database checkpoint incomplete");
    Ok(interrupted)
}
