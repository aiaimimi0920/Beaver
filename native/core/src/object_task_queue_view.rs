//! One transactional review of queue order, eligibility and optimistic version.
use crate::{
    object_task_coarse_dispatch, object_task_dispatch,
    object_task_queue::{self as queue, claim},
    object_task_storage as storage,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) const ORDER_KIND: &str = "object_task_queue_order_revision";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub task_id: String,
    pub object_id: String,
    pub title: String,
    pub state: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub project_id: String,
    pub version: String,
    pub items: Vec<Item>,
}

pub fn get(runtime: &ProjectRuntime, project_id: &str) -> Result<View> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| read_in(connection, project_id))
}

pub(crate) fn read_in(connection: &Connection, project_id: &str) -> Result<View> {
    let entries = queue::list_in(connection, project_id)?;
    let tasks: Vec<_> = storage::all_tasks(connection)?
        .into_iter()
        .filter(|task| task.project_id == project_id)
        .collect();
    let controls = object_task_dispatch::snapshot_in(connection, &tasks)?;
    let coarse = object_task_coarse_dispatch::snapshot_in(connection, &tasks)?;
    let mut held = HashSet::new();
    let mut records = Vec::new();
    for entry in &entries {
        let (task, run) = claim::read_medium(connection, project_id, &entry.task_id)?;
        claim::validate_state(entry, &task, &run)?;
        if entry.holds_object() {
            ensure!(
                held.insert(run.object_id.clone()),
                "OBJECT_TASK_QUEUE_MULTIPLE_OWNERS"
            );
        }
        records.push((task, run));
    }
    let order_revision = storage::read::<u64>(connection, ORDER_KIND, project_id)?.unwrap_or(0);
    // Include eligibility/identity records, not only positions: claims, cancellation,
    // definition changes and pause changes all invalidate an outstanding review.
    let version = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            order_revision,
            &entries,
            &tasks,
            &records,
            &controls,
            &coarse
        ))?)
    );
    let mut heads = HashSet::new();
    let mut items = Vec::new();
    for (entry, (task, run)) in entries.iter().zip(records) {
        let mut blockers = Vec::new();
        if entry.state == "queued" {
            if held.contains(&run.object_id) {
                blockers.push("objectHeld".into());
            }
            if !heads.insert(run.object_id.clone()) {
                blockers.push("earlierQueued".into());
            }
            if object_task_dispatch::read_in(connection, &task, &run)?.paused {
                blockers.push("paused".into());
            }
            if object_task_coarse_dispatch::parent_paused(connection, &task)? {
                blockers.push("coarsePaused".into());
            }
            if task.depends_on.iter().any(|id| {
                !tasks
                    .iter()
                    .any(|dependency| dependency.id == *id && dependency.status == "accepted")
            }) {
                blockers.push("dependencies".into());
            }
        }
        items.push(Item {
            task_id: task.id,
            object_id: run.object_id,
            title: task.title,
            state: entry.state.clone(),
            blockers,
        });
    }
    Ok(View {
        project_id: project_id.into(),
        version,
        items,
    })
}
