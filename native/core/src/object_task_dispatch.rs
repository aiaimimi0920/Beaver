//! Durable medium dispatch policy, independent of an in-flight execution lease.
use crate::{
    object_task_queue::{self as queue, claim},
    object_task_storage as storage,
    object_task_types::{valid_id, Granularity, RunRecord, TaskRecord, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_task_dispatch_control";
const RECEIPT_KIND: &str = "object_task_dispatch_receipt";
const MAX_CONTROL_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Control {
    pub schema_version: u32,
    pub project_id: String,
    pub task_id: String,
    pub object_id: String,
    pub run_id: String,
    pub paused: bool,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPausedRequest {
    pub project_id: String,
    pub task_id: String,
    pub object_id: String,
    pub run_id: String,
    pub request_id: String,
    pub expected_task_revision: u64,
    pub expected_control_revision: u64,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request: SetPausedRequest,
    pub result: Control,
}

pub(crate) fn read_in(
    connection: &Connection,
    task: &TaskRecord,
    run: &RunRecord,
) -> Result<Control> {
    let Some(control) = storage::read::<Control>(connection, KIND, &run.id)? else {
        return Ok(Control {
            schema_version: 1,
            project_id: task.project_id.clone(),
            task_id: task.id.clone(),
            object_id: run.object_id.clone(),
            run_id: run.id.clone(),
            paused: false,
            revision: 0,
        });
    };
    ensure!(
        control.schema_version == 1
            && control.project_id == task.project_id
            && control.task_id == task.id
            && control.object_id == run.object_id
            && control.run_id == run.id
            && (1..=MAX_CONTROL_REVISION).contains(&control.revision),
        "OBJECT_TASK_DISPATCH_IDENTITY_MISMATCH"
    );
    Ok(control)
}

pub(crate) fn snapshot_in(connection: &Connection, tasks: &[TaskRecord]) -> Result<Vec<Control>> {
    let controls = tasks
        .iter()
        .filter(|task| task.granularity == Granularity::Medium)
        .map(|task| {
            let (medium, run) = claim::read_medium(connection, &task.project_id, &task.id)?;
            read_in(connection, &medium, &run)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(controls
        .into_iter()
        .filter(|control| control.revision > 0)
        .collect())
}

pub fn set_paused(runtime: &ProjectRuntime, request: &SetPausedRequest) -> Result<Receipt> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        [
            &request.project_id,
            &request.task_id,
            &request.object_id,
            &request.run_id,
            &request.request_id,
        ]
        .into_iter()
        .all(|id| valid_id(id)),
        "INVALID_OBJECT_TASK_DISPATCH_ID"
    );
    ensure!(
        request.expected_task_revision <= MAX_REVISION
            && request.expected_control_revision < MAX_CONTROL_REVISION,
        "INVALID_OBJECT_TASK_DISPATCH_REVISION"
    );
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            &request.project_id,
            &request.request_id
        ))?)
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = storage::read::<Receipt>(connection, RECEIPT_KIND, &key)? {
            ensure!(
                receipt.request == *request,
                "OBJECT_TASK_DISPATCH_REQUEST_CONFLICT"
            );
            ensure!(
                receipt.result.schema_version == 1
                    && receipt.result.project_id == request.project_id
                    && receipt.result.task_id == request.task_id
                    && receipt.result.object_id == request.object_id
                    && receipt.result.run_id == request.run_id
                    && receipt.result.paused == request.paused
                    && receipt.result.revision == request.expected_control_revision + 1,
                "OBJECT_TASK_DISPATCH_RECEIPT_MISMATCH"
            );
            return Ok(receipt);
        }
        let (task, run) = claim::read_medium(connection, &request.project_id, &request.task_id)?;
        ensure!(
            run.id == request.run_id && run.object_id == request.object_id,
            "OBJECT_TASK_DISPATCH_TARGET_MISMATCH"
        );
        ensure!(
            task.revision == request.expected_task_revision,
            "OBJECT_TASK_REVISION_CONFLICT"
        );
        ensure!(
            task.status == run.status
                && matches!(
                    task.status.as_str(),
                    "planned" | "queued" | "running" | "awaitingAcceptance" | "failed"
                ),
            "OBJECT_TASK_DISPATCH_NOT_CONTROLLABLE"
        );
        let entry = queue::read_entry(connection, &request.project_id, &request.task_id)?;
        if task.status != "planned" {
            entry
                .as_ref()
                .context("OBJECT_TASK_QUEUE_ENTRY_NOT_FOUND")?;
        }
        if let Some(entry) = entry {
            claim::validate_state(&entry, &task, &run)?;
        }
        let mut control = read_in(connection, &task, &run)?;
        ensure!(
            control.revision == request.expected_control_revision,
            "OBJECT_TASK_DISPATCH_REVISION_CONFLICT"
        );
        control.paused = request.paused;
        control.revision += 1;
        storage::replace(connection, KIND, &run.id, &control)?;
        let receipt = Receipt {
            request: request.clone(),
            result: control,
        };
        storage::insert(connection, RECEIPT_KIND, &key, &receipt)?;
        Ok(receipt)
    })
}
