use crate::{
    object_task_queue::{self, QUEUE_KIND},
    object_task_storage::{self, CANCEL_RECEIPT_KIND, RUN_KIND, STATE_KIND, TASK_KIND},
    object_task_types::{
        valid_id, CancelPlannedReceipt, CancelPlannedRequest, Granularity, RunRecord, TaskRecord,
        MAX_REVISION,
    },
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

pub(crate) fn cancel_planned(
    runtime: &ProjectRuntime,
    request: &CancelPlannedRequest,
) -> Result<CancelPlannedReceipt> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.task_id) && valid_id(&request.request_id),
        "INVALID_OBJECT_TASK_ID: cancellation"
    );
    ensure!(
        request.expected_task_revision <= MAX_REVISION
            && request.expected_plan_revision <= MAX_REVISION,
        "INVALID_OBJECT_TASK_REVISION"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store
        .transaction(|connection| {
            if let Some(receipt) = object_task_storage::read::<CancelPlannedReceipt>(
                connection,
                CANCEL_RECEIPT_KIND,
                &request.request_id,
            )? {
                ensure!(
                    receipt.project_id == request.project_id
                        && receipt.task_id == request.task_id
                        && receipt.request_id == request.request_id
                        && receipt.previous_task_revision == request.expected_task_revision
                        && receipt.previous_plan_revision == request.expected_plan_revision,
                    "OBJECT_TASK_REQUEST_ID_CONFLICT"
                );
                return Ok(receipt);
            }
            let mut state = object_task_storage::plan_state(connection, &request.project_id)?;
            ensure!(
                state.revision == request.expected_plan_revision,
                "OBJECT_TASK_PLAN_REVISION_CONFLICT"
            );
            let task =
                object_task_storage::read::<TaskRecord>(connection, TASK_KIND, &request.task_id)?
                    .context("OBJECT_TASK_NOT_FOUND")?;
            ensure!(task.id == request.task_id, "OBJECT_TASK_IDENTITY_MISMATCH");
            ensure!(
                task.project_id == request.project_id,
                "OBJECT_TASK_PROJECT_MISMATCH"
            );
            ensure!(
                task.revision == request.expected_task_revision,
                "OBJECT_TASK_REVISION_CONFLICT"
            );
            ensure!(task.status == "planned", "OBJECT_TASK_NOT_PLANNED");
            ensure!(
                task.revision < MAX_REVISION && state.revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );

            let previous_task_revision = task.revision;
            let previous_plan_revision = state.revision;
            cancel_owned_work(connection, &task)?;
            state.revision += 1;
            object_task_storage::replace(connection, STATE_KIND, &request.project_id, &state)?;
            let receipt = CancelPlannedReceipt {
                project_id: request.project_id.clone(),
                task_id: task.id,
                request_id: request.request_id.clone(),
                previous_task_revision,
                task_revision: task.revision + 1,
                previous_plan_revision,
                plan_revision: state.revision,
            };
            object_task_storage::insert(
                connection,
                CANCEL_RECEIPT_KIND,
                &request.request_id,
                &receipt,
            )?;
            Ok(receipt)
        })
        .context("OBJECT_TASK_CANCEL_FAILED")
}

/// The caller validates the root revision or writer claim in this transaction.
pub(crate) fn cancel_owned_work(
    connection: &Connection,
    root: &TaskRecord,
) -> Result<Vec<TaskRecord>> {
    let tasks = object_task_storage::all_tasks(connection)?;
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for task in &tasks {
        if let Some(parent) = &task.parent_task_id {
            children.entry(parent).or_default().push(&task.id);
        }
    }
    let mut scope = HashSet::new();
    let mut pending = vec![root.id.as_str()];
    while let Some(id) = pending.pop() {
        if scope.insert(id) {
            pending.extend(children.get(id).into_iter().flatten().copied());
        }
    }
    let mut owned_tasks: Vec<_> = tasks
        .iter()
        .filter(|task| scope.contains(task.id.as_str()))
        .cloned()
        .collect();
    let mut owned_runs = Vec::new();
    let mut queue_entries = Vec::new();
    ensure!(
        owned_tasks.iter().any(|task| task == root),
        "OBJECT_TASK_IDENTITY_MISMATCH"
    );
    for task in &owned_tasks {
        ensure!(
            object_task_storage::read::<TaskRecord>(connection, TASK_KIND, &task.id)?.as_ref()
                == Some(task),
            "OBJECT_TASK_IDENTITY_MISMATCH"
        );
        ensure!(
            task.project_id == root.project_id,
            "OBJECT_TASK_PROJECT_MISMATCH"
        );
        ensure!(
            task.id == root.id || task.status == "planned" || task.status == "cancelled",
            "OBJECT_TASK_NOT_PLANNED: {}",
            task.id
        );
        ensure!(
            task.status == "cancelled" || task.revision < MAX_REVISION,
            "OBJECT_TASK_REVISION_EXHAUSTED"
        );
        if let Some(entry) = object_task_queue::read_entry(connection, &root.project_id, &task.id)?
        {
            ensure!(
                entry.task_status()? == task.status
                    && (!entry.holds_object() || task.id == root.id),
                "OBJECT_TASK_QUEUE_STATE_MISMATCH"
            );
            queue_entries.push(entry);
        }
        if task.granularity == Granularity::Medium {
            let run_id = task
                .run_id
                .as_deref()
                .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
            let run = object_task_storage::read::<RunRecord>(connection, RUN_KIND, run_id)?
                .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
            ensure!(
                run.id == run_id
                    && run.project_id == root.project_id
                    && Some(run.object_id.as_str()) == task.object_id.as_deref()
                    && run.medium_task_id == task.id,
                "OBJECT_TASK_MEDIUM_RUN_MISMATCH"
            );
            crate::object_run_preparation::require_unprepared(connection, run_id)?;
            ensure!(
                (task.id == root.id && run.status == root.status)
                    || run.status == "planned"
                    || run.status == "cancelled",
                "OBJECT_TASK_RUN_NOT_PLANNED: {run_id}"
            );
            ensure!(
                run.status == "cancelled" || run.revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );
            owned_runs.push(run);
        }
    }
    for task in &mut owned_tasks {
        if task.status != "cancelled" {
            task.status = "cancelled".into();
            task.revision += 1;
            object_task_storage::replace(connection, TASK_KIND, &task.id, task)?;
        }
    }
    for run in &mut owned_runs {
        if run.status != "cancelled" {
            run.status = "cancelled".into();
            run.revision += 1;
            object_task_storage::replace(connection, RUN_KIND, &run.id, run)?;
        }
    }
    for entry in &mut queue_entries {
        if entry.state != "cancelled" {
            entry.state = "cancelled".into();
            object_task_storage::replace(connection, QUEUE_KIND, &entry.id, entry)?;
        }
    }
    Ok(owned_tasks)
}
