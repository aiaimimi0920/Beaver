use crate::{
    object_catalog, object_task_commit_records,
    object_task_storage::{self, DRAFT_KIND, RECEIPT_KIND, STATE_KIND},
    object_task_types::{
        CommitReceipt, CommitRequest, Draft, PlanAssumptionRecord, PlanState,
        MAX_ASSUMPTION_HISTORY, MAX_REVISION,
    },
    object_task_validation,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};

pub(crate) fn commit(runtime: &ProjectRuntime, request: &CommitRequest) -> Result<CommitReceipt> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        crate::object_task_types::valid_id(&request.request_id)
            && crate::object_task_types::valid_id(&request.draft_id),
        "INVALID_OBJECT_TASK_ID: commit"
    );
    ensure!(
        request.expected_draft_revision <= MAX_REVISION
            && request.expected_plan_revision <= MAX_REVISION,
        "INVALID_OBJECT_TASK_REVISION"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store
        .transaction(|connection| {
            if let Some(receipt) = object_task_storage::read::<CommitReceipt>(
                connection,
                RECEIPT_KIND,
                &request.request_id,
            )? {
                ensure!(
                    receipt.project_id == request.project_id
                        && receipt.request_id == request.request_id
                        && receipt.draft_id == request.draft_id
                        && receipt.draft_revision == request.expected_draft_revision
                        && receipt.previous_plan_revision == request.expected_plan_revision,
                    "OBJECT_TASK_REQUEST_ID_CONFLICT"
                );
                return Ok(receipt);
            }
            let state = object_task_storage::plan_state(connection, &request.project_id)?;
            ensure!(
                state.revision == request.expected_plan_revision,
                "OBJECT_TASK_PLAN_REVISION_CONFLICT"
            );
            let mut draft =
                object_task_storage::read::<Draft>(connection, DRAFT_KIND, &request.draft_id)?
                    .context("OBJECT_TASK_DRAFT_NOT_FOUND")?;
            ensure!(
                draft.id == request.draft_id,
                "OBJECT_TASK_DRAFT_IDENTITY_MISMATCH"
            );
            ensure!(
                draft.project_id == request.project_id,
                "OBJECT_TASK_DRAFT_PROJECT_MISMATCH"
            );
            ensure!(
                draft.revision == request.expected_draft_revision,
                "OBJECT_TASK_DRAFT_REVISION_CONFLICT"
            );
            ensure!(
                draft.plan_revision == state.revision,
                "OBJECT_TASK_DRAFT_PLAN_REVISION_CONFLICT"
            );
            ensure!(
                draft.committed_request_id.is_none(),
                "OBJECT_TASK_DRAFT_LOCKED"
            );
            ensure!(
                draft.revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );
            object_task_validation::validate_plan(
                connection,
                &request.project_id,
                &draft.plan,
                false,
            )?;
            let records =
                object_task_commit_records::prepare(connection, &request.project_id, &draft.plan)?;
            let new_assumptions = draft
                .plan
                .assumptions
                .iter()
                .filter(|proposal| {
                    !state.assumptions.iter().any(|accepted| {
                        accepted.id == proposal.id
                            && accepted.statement == proposal.statement
                            && accepted.basis == proposal.basis
                            && accepted.source == proposal.source
                            && accepted.source_detail == proposal.source_detail
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            ensure!(
                state.assumptions.len() + new_assumptions.len() <= MAX_ASSUMPTION_HISTORY,
                "OBJECT_TASK_ASSUMPTION_HISTORY_LIMIT"
            );
            let changed = records.changed() || !new_assumptions.is_empty();
            ensure!(
                !changed || state.revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );
            for object in &records.new_objects {
                object_catalog::insert(connection, object)?;
            }
            for run in &records.new_runs {
                object_task_storage::insert(
                    connection,
                    object_task_storage::RUN_KIND,
                    &run.id,
                    run,
                )?;
            }
            for task in &records.new_tasks {
                object_task_storage::insert(
                    connection,
                    object_task_storage::TASK_KIND,
                    &task.id,
                    task,
                )?;
            }
            let plan_revision = state.revision + u64::from(changed);
            if changed {
                let mut assumptions = state.assumptions;
                assumptions.extend(new_assumptions.into_iter().map(|proposal| {
                    PlanAssumptionRecord {
                        id: proposal.id,
                        statement: proposal.statement,
                        basis: proposal.basis,
                        source: proposal.source,
                        source_detail: proposal.source_detail,
                        plan_revision,
                    }
                }));
                object_task_storage::replace(
                    connection,
                    STATE_KIND,
                    &request.project_id,
                    &PlanState {
                        project_id: request.project_id.clone(),
                        revision: plan_revision,
                        assumptions,
                    },
                )?;
            }
            let receipt = CommitReceipt {
                project_id: request.project_id.clone(),
                request_id: request.request_id.clone(),
                draft_id: request.draft_id.clone(),
                draft_revision: draft.revision,
                previous_plan_revision: state.revision,
                plan_revision,
                object_ids: records
                    .objects
                    .iter()
                    .map(|object| object.id.clone())
                    .collect(),
                task_ids: records.tasks.iter().map(|task| task.id.clone()).collect(),
                runs: records.runs,
            };
            object_task_storage::insert(connection, RECEIPT_KIND, &request.request_id, &receipt)?;
            draft.revision += 1;
            draft.committed_request_id = Some(request.request_id.clone());
            object_task_storage::replace(connection, DRAFT_KIND, &draft.id, &draft)?;
            Ok(receipt)
        })
        .context("OBJECT_TASK_COMMIT_FAILED")
}
