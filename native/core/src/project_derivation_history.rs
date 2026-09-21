//! Copy diagnostic history into a caller-owned offline conversion transaction.
use crate::{
    project_derivation_identity::IdentityMap, project_derivation_validation_records::Rewrite,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, Transaction};
use serde_json::Value;
use std::collections::BTreeSet;

/// Keeps sequence numbers and opaque historical evidence. The caller must roll back on error
/// and perform runtime interruption and full target validation before committing or activating.
pub fn copy(source: &Connection, map: &IdentityMap, target: &Transaction<'_>) -> Result<()> {
    ensure!(
        map.format == "beaver-project-derivation-identities-v1",
        "unsupported identity map"
    );
    let count: i64 = target.query_row(
        "SELECT (SELECT COUNT(*) FROM events)+(SELECT COUNT(*) FROM calls)",
        [],
        |row| row.get(0),
    )?;
    ensure!(count == 0, "derivation history destination must be empty");
    let rewrite = Rewrite(map);
    let mut events = source.prepare("SELECT seq,task,time,kind,text FROM events ORDER BY seq")?;
    let rows = events.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    for row in rows {
        let (seq, task, time, kind, text) = row?;
        let task = rewrite.key("task", &task)?.id;
        target.execute(
            "INSERT INTO events(seq,task,time,kind,text) VALUES(?,?,?,?,?)",
            params![seq, task, time, kind, text],
        )?;
    }
    let mut calls =
        source.prepare("SELECT seq,id,task,project,method,value FROM calls ORDER BY seq")?;
    let rows = calls.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let (seq, id, task, project, method, raw) = row?;
        let mut value: Value = serde_json::from_str(&raw)?;
        ensure!(
            value["id"] == id && value["method"] == method,
            "call identity mismatch"
        );
        for (field, column) in [
            ("taskId", task.as_deref()),
            ("projectId", project.as_deref()),
        ] {
            ensure!(
                value[field] == column.map(Value::from).unwrap_or(Value::Null),
                "call column mismatch"
            );
        }
        ensure!(task.is_some() || project.is_some(), "call owner missing");
        let mapped = map.calls.get(&id).context("call mapping missing")?;
        ensure!(!mapped.is_empty() && mapped != &id, "invalid call mapping");
        seen.insert(id);
        value["id"] = Value::String(mapped.clone());
        rewrite.one(&mut value, "/taskId", "task")?;
        rewrite.one(&mut value, "/projectId", "project")?;
        target.execute(
            "INSERT INTO calls(seq,id,task,project,method,value) VALUES(?,?,?,?,?,?)",
            params![
                seq,
                mapped,
                value["taskId"].as_str(),
                value["projectId"].as_str(),
                method,
                value.to_string()
            ],
        )?;
    }
    ensure!(seen.len() == map.calls.len(), "unexpected call mappings");
    // Retention may have removed the highest row. Preserve cursors across future inserts.
    let sequence: i64 = source.query_row(
        "SELECT COALESCE(MAX(seq),0) FROM sqlite_sequence WHERE name='calls'",
        [],
        |row| row.get(0),
    )?;
    target.execute("DELETE FROM sqlite_sequence WHERE name='calls'", [])?;
    target.execute(
        "INSERT INTO sqlite_sequence(name,seq) VALUES('calls',?)",
        [sequence],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "project_derivation_history_tests.rs"]
mod tests;
