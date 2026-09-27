use crate::object_task_types::{PlanState, RunRecord, TaskRecord};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

pub(crate) const TASK_KIND: &str = "object_task";
pub(crate) const RUN_KIND: &str = "object_run";
pub(crate) const DRAFT_KIND: &str = "object_task_draft";
pub(crate) const STATE_KIND: &str = "object_task_plan_state";
pub(crate) const RECEIPT_KIND: &str = "object_task_commit_receipt";
pub(crate) const CANCEL_RECEIPT_KIND: &str = "object_task_cancel_receipt";
pub(crate) const REVISION_KIND: &str = "object_task_definition_revision";

pub(crate) fn read<T: DeserializeOwned>(
    connection: &Connection,
    kind: &str,
    id: &str,
) -> Result<Option<T>> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![kind, id],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|json| Ok(serde_json::from_str(&json)?))
        .transpose()
}

pub(crate) fn insert<T: Serialize>(
    connection: &Connection,
    kind: &str,
    id: &str,
    value: &T,
) -> Result<()> {
    connection.execute(
        "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
        params![kind, id, serde_json::to_string(value)?],
    )?;
    Ok(())
}

pub(crate) fn replace<T: Serialize>(
    connection: &Connection,
    kind: &str,
    id: &str,
    value: &T,
) -> Result<()> {
    connection.execute(
        "INSERT INTO entities(kind,id,value) VALUES(?,?,?)
         ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
        params![kind, id, serde_json::to_string(value)?],
    )?;
    Ok(())
}

pub(crate) fn plan_state(connection: &Connection, project_id: &str) -> Result<PlanState> {
    Ok(
        read::<PlanState>(connection, STATE_KIND, project_id)?.unwrap_or(PlanState {
            project_id: project_id.to_owned(),
            revision: 0,
            assumptions: vec![],
        }),
    )
}

pub(crate) fn all_tasks(connection: &Connection) -> Result<Vec<TaskRecord>> {
    read_all(connection, TASK_KIND)
}

pub(crate) fn all_runs(connection: &Connection) -> Result<Vec<RunRecord>> {
    read_all(connection, RUN_KIND)
}

pub(crate) fn read_all<T: DeserializeOwned>(connection: &Connection, kind: &str) -> Result<Vec<T>> {
    let mut statement =
        connection.prepare("SELECT value FROM entities WHERE kind=? ORDER BY id")?;
    let rows = statement.query_map([kind], |row| row.get::<_, String>(0))?;
    rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
}
