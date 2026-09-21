//! Shared entity storage format; opening an existing project never runs this DDL.
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

pub(crate) const DEFINITIONS: &[(&str, &str)] = &[
    (
        "entities",
        "CREATE TABLE IF NOT EXISTS entities (
           kind TEXT NOT NULL, id TEXT NOT NULL, value TEXT NOT NULL,
           PRIMARY KEY(kind,id))",
    ),
    (
        "events",
        "CREATE TABLE IF NOT EXISTS events (
           seq INTEGER PRIMARY KEY, task TEXT NOT NULL, time TEXT NOT NULL,
           kind TEXT NOT NULL, text TEXT NOT NULL)",
    ),
    (
        "task_events",
        "CREATE INDEX IF NOT EXISTS task_events ON events(task,seq)",
    ),
    (
        "calls",
        "CREATE TABLE IF NOT EXISTS calls (
           seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
           task TEXT, project TEXT, method TEXT NOT NULL, value TEXT NOT NULL)",
    ),
    (
        "calls_task",
        "CREATE INDEX IF NOT EXISTS calls_task ON calls(task,seq)",
    ),
    (
        "calls_project",
        "CREATE INDEX IF NOT EXISTS calls_project ON calls(project,seq)",
    ),
];

pub(crate) fn initialize(connection: &Connection) -> Result<()> {
    for (_, sql) in DEFINITIONS {
        connection.execute_batch(sql)?;
    }
    Ok(())
}

pub(crate) fn validate_definition(connection: &Connection, name: &str, sql: &str) -> Result<()> {
    let stored: String = connection
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name=?",
            [name],
            |row| row.get(0),
        )
        .with_context(|| format!("项目数据库缺少结构：{name}"))?;
    let normalize = |value: &str| {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
            .replace("if not exists ", "")
    };
    ensure!(
        normalize(&stored) == normalize(sql),
        "项目数据库结构不匹配：{name}"
    );
    Ok(())
}
