use crate::{
    object_task_storage::{self, TASK_KIND},
    object_task_types::TaskRecord,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "object_task_queue_claim.rs"]
pub(crate) mod claim;
use claim::{cancel_claim_in, finish_claim_in};

pub(crate) const QUEUE_KIND: &str = "object_task_queue";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueEntry {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub position: u64,
    pub state: String,
    pub claim_token: Option<String>,
    pub owner: Option<String>,
    pub generation: u64,
    pub enqueued_at: u64,
}

impl QueueEntry {
    pub(crate) fn holds_object(&self) -> bool {
        matches!(
            self.state.as_str(),
            "claimed" | "running" | "awaitingAcceptance" | "failed"
        )
    }

    pub(crate) fn task_status(&self) -> Result<&str> {
        match self.state.as_str() {
            "queued" => Ok("planned"),
            "claimed" => Ok("queued"),
            "running" | "awaitingAcceptance" | "failed" | "cancelled" | "accepted" => {
                Ok(&self.state)
            }
            _ => anyhow::bail!("OBJECT_TASK_QUEUE_STATE_MISMATCH"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    pub task: TaskRecord,
    pub claim_token: String,
    pub generation: u64,
}

pub fn enqueue(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_ids: &[String],
) -> Result<Vec<QueueEntry>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| enqueue_in(connection, project_id, task_ids))
}

/// Returns the persisted queue for a project in deterministic display order.
/// Reading the queue never changes task or claim state.
pub fn list(runtime: &ProjectRuntime, project_id: &str) -> Result<Vec<QueueEntry>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| list_in(connection, project_id))
}

pub(crate) fn list_in(connection: &Connection, project_id: &str) -> Result<Vec<QueueEntry>> {
    let mut statement = connection.prepare(
        "SELECT id,value FROM entities WHERE kind=? AND json_extract(value,'$.projectId')=?",
    )?;
    let rows = statement.query_map(rusqlite::params![QUEUE_KIND, project_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut entries = rows
        .map(|row| {
            let (id, json) = row?;
            let entry: QueueEntry = serde_json::from_str(&json)?;
            ensure!(
                entry.id == id && id == format!("{project_id}:{}", entry.task_id),
                "OBJECT_TASK_QUEUE_IDENTITY_MISMATCH"
            );
            Ok(entry)
        })
        .collect::<Result<Vec<QueueEntry>>>()?;
    entries.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    Ok(entries)
}

pub(crate) fn read_entry(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<Option<QueueEntry>> {
    let id = format!("{project_id}:{task_id}");
    let entry = object_task_storage::read::<QueueEntry>(connection, QUEUE_KIND, &id)?;
    if let Some(entry) = &entry {
        ensure!(
            entry.id == id && entry.project_id == project_id && entry.task_id == task_id,
            "OBJECT_TASK_QUEUE_IDENTITY_MISMATCH"
        );
    }
    Ok(entry)
}

fn enqueue_in(
    connection: &Connection,
    project_id: &str,
    task_ids: &[String],
) -> Result<Vec<QueueEntry>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let mut entries = Vec::new();
    for task_id in task_ids {
        let id = format!("{project_id}:{task_id}");
        let (task, run) = claim::read_medium(connection, project_id, task_id)?;
        if let Some(existing) = read_entry(connection, project_id, task_id)? {
            claim::validate_state(&existing, &task, &run)?;
            entries.push(existing);
            continue;
        }
        ensure!(
            task.status == "planned",
            "OBJECT_TASK_NOT_PLANNED: {task_id}"
        );
        ensure!(run.status == "planned", "OBJECT_TASK_RUN_NOT_PLANNED");
        let entry = QueueEntry {
            id: id.clone(),
            project_id: project_id.to_owned(),
            task_id: task.id,
            position: task.position,
            state: "queued".into(),
            claim_token: None,
            owner: None,
            generation: 0,
            enqueued_at: now,
        };
        object_task_storage::insert(connection, QUEUE_KIND, &id, &entry)?;
        entries.push(entry);
    }
    entries.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    Ok(entries)
}

pub fn claim_next(
    runtime: &ProjectRuntime,
    project_id: &str,
    owner: &str,
) -> Result<Option<Claim>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(!owner.trim().is_empty(), "OBJECT_TASK_QUEUE_OWNER_REQUIRED");
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| claim_next_in(connection, project_id, owner))
}

pub fn finish_claim(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
    success: bool,
) -> Result<TaskRecord> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| {
        finish_claim_in(
            connection,
            project_id,
            task_id,
            owner,
            claim_token,
            generation,
            success,
        )
    })
}

/// Cancels an owned claim without allowing a stale worker to change it.
/// Cancellation is terminal for the queued task; a new attempt must be created
/// by the planning layer rather than silently reusing the old claim.
pub fn cancel_claim(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
) -> Result<TaskRecord> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| {
        cancel_claim_in(
            connection,
            project_id,
            task_id,
            owner,
            claim_token,
            generation,
        )
    })
}

pub(crate) fn claim_next_in(
    connection: &Connection,
    project_id: &str,
    owner: &str,
) -> Result<Option<Claim>> {
    let entries = list_in(connection, project_id)?;
    let mut reserved_objects = HashSet::new();
    for entry in entries.iter().filter(|entry| entry.holds_object()) {
        let (task, run) = claim::read_medium(connection, project_id, &entry.task_id)?;
        claim::validate_state(entry, &task, &run)?;
        ensure!(
            reserved_objects.insert(run.object_id),
            "OBJECT_TASK_QUEUE_MULTIPLE_OWNERS"
        );
    }
    for entry in entries.into_iter().filter(|entry| entry.state == "queued") {
        let (task, run) = claim::read_medium(connection, project_id, &entry.task_id)?;
        claim::validate_state(&entry, &task, &run)?;
        // A paused or dependency-blocked head still reserves its object's position.
        if !reserved_objects.insert(run.object_id.clone()) {
            continue;
        }
        if crate::object_task_dispatch::read_in(connection, &task, &run)?.paused
            || crate::object_task_coarse_dispatch::parent_paused(connection, &task)?
        {
            continue;
        }
        let mut dependencies_ready = true;
        for dependency in &task.depends_on {
            let Some(dep) =
                object_task_storage::read::<TaskRecord>(connection, TASK_KIND, dependency)?
            else {
                dependencies_ready = false;
                break;
            };
            if dep.id != *dependency || dep.project_id != project_id || dep.status != "accepted" {
                dependencies_ready = false;
                break;
            }
        }
        if !dependencies_ready {
            continue;
        }
        return claim::begin_claim(connection, entry, task, run, owner).map(Some);
    }
    Ok(None)
}

#[cfg(test)]
#[path = "object_task_queue_tests.rs"]
mod tests;
