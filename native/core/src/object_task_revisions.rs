use crate::{
    object_task_definition::{
        DefinitionAdopter, RevisePlannedRequest, TaskDefinition, TaskDefinitionRevision,
    },
    object_task_revision_scope,
    object_task_storage::{self, REVISION_KIND, STATE_KIND, TASK_KIND},
    object_task_types::{valid_id, PlanProposal, TaskRecord, MAX_REVISION},
    object_task_validation,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::params;

pub(crate) fn revise_planned(
    runtime: &ProjectRuntime,
    request: &RevisePlannedRequest,
) -> Result<TaskDefinitionRevision> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.task_id) && valid_id(&request.request_id),
        "INVALID_OBJECT_TASK_ID: revision"
    );
    ensure!(
        request.expected_task_revision <= MAX_REVISION
            && request.expected_plan_revision <= MAX_REVISION,
        "INVALID_OBJECT_TASK_REVISION"
    );
    ensure!(
        !request.reason.trim().is_empty() && request.reason.len() <= 2_000,
        "INVALID_OBJECT_TASK_REVISION_REASON"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store
        .transaction(|connection| {
            if let Some(receipt) = object_task_storage::read::<TaskDefinitionRevision>(
                connection,
                REVISION_KIND,
                &request.request_id,
            )? {
                ensure!(
                    receipt.project_id == request.project_id
                        && receipt.task_id == request.task_id
                        && receipt.request_id == request.request_id
                        && receipt.previous_task_revision == request.expected_task_revision
                        && receipt.previous_plan_revision == request.expected_plan_revision
                        && receipt.after == request.definition
                        && receipt.reason == request.reason,
                    "OBJECT_TASK_REQUEST_ID_CONFLICT"
                );
                return Ok(receipt);
            }
            let mut state = object_task_storage::plan_state(connection, &request.project_id)?;
            ensure!(
                state.revision == request.expected_plan_revision,
                "OBJECT_TASK_PLAN_REVISION_CONFLICT"
            );
            let mut task =
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
            let before = TaskDefinition::from_task(&task);
            ensure!(
                before != request.definition,
                "OBJECT_TASK_DEFINITION_UNCHANGED"
            );
            let affected_task_ids =
                object_task_revision_scope::validate_and_describe(connection, &task)?;
            let proposal = request.definition.proposal(&task);
            object_task_validation::validate_available_references(
                connection,
                &request.project_id,
                &proposal,
            )?;
            object_task_validation::validate_plan(
                connection,
                &request.project_id,
                &PlanProposal {
                    tasks: vec![proposal],
                    ..Default::default()
                },
                false,
            )?;
            request.definition.apply_to(&mut task);
            task.revision += 1;
            state.revision += 1;
            let receipt = TaskDefinitionRevision {
                project_id: request.project_id.clone(),
                task_id: request.task_id.clone(),
                request_id: request.request_id.clone(),
                previous_task_revision: request.expected_task_revision,
                task_revision: task.revision,
                previous_plan_revision: request.expected_plan_revision,
                plan_revision: state.revision,
                before,
                after: request.definition.clone(),
                reason: request.reason.clone(),
                adopted_by: DefinitionAdopter::Owner,
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                affected_task_ids,
            };
            object_task_storage::replace(connection, TASK_KIND, &task.id, &task)?;
            object_task_storage::replace(connection, STATE_KIND, &request.project_id, &state)?;
            object_task_storage::insert(connection, REVISION_KIND, &request.request_id, &receipt)?;
            Ok(receipt)
        })
        .context("OBJECT_TASK_REVISE_FAILED")
}

pub(crate) fn history(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
) -> Result<Vec<TaskDefinitionRevision>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(valid_id(task_id), "INVALID_OBJECT_TASK_ID: task id");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    let task = object_task_storage::read::<TaskRecord>(&store.connection, TASK_KIND, task_id)?
        .context("OBJECT_TASK_NOT_FOUND")?;
    ensure!(
        task.id == task_id && task.project_id == project_id,
        "OBJECT_TASK_IDENTITY_MISMATCH"
    );
    let mut statement = store.connection.prepare(
        "SELECT value FROM entities WHERE kind=? AND json_extract(value,'$.projectId')=?
         AND json_extract(value,'$.taskId')=? ORDER BY json_extract(value,'$.taskRevision')",
    )?;
    let rows = statement.query_map(params![REVISION_KIND, project_id, task_id], |row| {
        row.get::<_, String>(0)
    })?;
    rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
}
