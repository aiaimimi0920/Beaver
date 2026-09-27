//! Persistent responsibility-scope policy; never rewrites child controls or leases.
use crate::{
    object_framework::{Identity, VERSION},
    object_task_storage::{self as storage, TASK_KIND},
    object_task_types::{valid_id, Granularity, TaskRecord, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_task_coarse_dispatch_control";
const RECEIPT_KIND: &str = "object_task_coarse_dispatch_receipt";
const MAX_CONTROL_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Control {
    pub schema_version: u32,
    pub project_id: String,
    pub task_id: String,
    pub paused: bool,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPausedRequest {
    pub project_id: String,
    pub task_id: String,
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

fn read_task(connection: &Connection, project_id: &str, task_id: &str) -> Result<TaskRecord> {
    let task = storage::read::<TaskRecord>(connection, TASK_KIND, task_id)?
        .context("OBJECT_TASK_NOT_FOUND")?;
    ensure!(
        task.id == task_id
            && task.project_id == project_id
            && task.granularity == Granularity::Coarse
            && matches!(task.identity, Identity::Coarse { schema_version } if schema_version == VERSION)
            && task.object_id.is_none()
            && task.run_id.is_none()
            && task.stage_id.is_none()
            && task.parent_task_id.is_none(),
        "OBJECT_TASK_DISPATCH_TARGET_MISMATCH"
    );
    Ok(task)
}

fn read_in(connection: &Connection, task: &TaskRecord) -> Result<Control> {
    let Some(control) = storage::read::<Control>(connection, KIND, &task.id)? else {
        return Ok(Control {
            schema_version: 1,
            project_id: task.project_id.clone(),
            task_id: task.id.clone(),
            paused: false,
            revision: 0,
        });
    };
    ensure!(
        control.schema_version == 1
            && control.project_id == task.project_id
            && control.task_id == task.id
            && (1..=MAX_CONTROL_REVISION).contains(&control.revision),
        "OBJECT_TASK_DISPATCH_IDENTITY_MISMATCH"
    );
    Ok(control)
}

pub(crate) fn parent_paused(connection: &Connection, medium: &TaskRecord) -> Result<bool> {
    let Some(parent_id) = &medium.parent_task_id else {
        return Ok(false);
    };
    let parent = read_task(connection, &medium.project_id, parent_id)?;
    Ok(read_in(connection, &parent)?.paused)
}

pub(crate) fn snapshot_in(connection: &Connection, tasks: &[TaskRecord]) -> Result<Vec<Control>> {
    let mut controls = Vec::new();
    for task in tasks
        .iter()
        .filter(|task| task.granularity == Granularity::Coarse)
    {
        let task = read_task(connection, &task.project_id, &task.id)?;
        let control = read_in(connection, &task)?;
        if control.revision > 0 {
            controls.push(control);
        }
    }
    Ok(controls)
}

pub fn set_paused(runtime: &ProjectRuntime, request: &SetPausedRequest) -> Result<Receipt> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        [&request.project_id, &request.task_id, &request.request_id]
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
                    && receipt.result.paused == request.paused
                    && receipt.result.revision == request.expected_control_revision + 1,
                "OBJECT_TASK_DISPATCH_RECEIPT_MISMATCH"
            );
            return Ok(receipt);
        }
        let task = read_task(connection, &request.project_id, &request.task_id)?;
        ensure!(
            task.revision == request.expected_task_revision,
            "OBJECT_TASK_REVISION_CONFLICT"
        );
        ensure!(
            matches!(
                task.status.as_str(),
                "planned" | "queued" | "running" | "awaitingAcceptance" | "failed"
            ),
            "OBJECT_TASK_DISPATCH_NOT_CONTROLLABLE"
        );
        let mut control = read_in(connection, &task)?;
        ensure!(
            control.revision == request.expected_control_revision,
            "OBJECT_TASK_DISPATCH_REVISION_CONFLICT"
        );
        control.paused = request.paused;
        control.revision += 1;
        storage::replace(connection, KIND, &task.id, &control)?;
        let receipt = Receipt {
            request: request.clone(),
            result: control,
        };
        storage::insert(connection, RECEIPT_KIND, &key, &receipt)?;
        Ok(receipt)
    })
}
