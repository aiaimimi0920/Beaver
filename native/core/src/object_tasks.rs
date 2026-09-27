use crate::{
    object_task_cancel, object_task_commit, object_task_drafts,
    object_task_queue::{self, Claim, QueueEntry},
    object_task_revisions,
    object_task_storage::{self, RUN_KIND, TASK_KIND},
    object_task_types::valid_id,
    project_runtime::ProjectRuntime,
};

pub use crate::object_task_queue::{Claim as ObjectTaskClaim, QueueEntry as ObjectTaskQueueEntry};

pub fn enqueue(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_ids: &[String],
) -> Result<Vec<QueueEntry>> {
    object_task_queue::enqueue(runtime, project_id, task_ids)
}

pub fn queue(runtime: &ProjectRuntime, project_id: &str) -> Result<Vec<QueueEntry>> {
    object_task_queue::list(runtime, project_id)
}

pub fn claim_next(
    runtime: &ProjectRuntime,
    project_id: &str,
    owner: &str,
) -> Result<Option<Claim>> {
    object_task_queue::claim_next(runtime, project_id, owner)
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
    object_task_queue::finish_claim(
        runtime,
        project_id,
        task_id,
        owner,
        claim_token,
        generation,
        success,
    )
}

pub fn cancel_claim(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
) -> Result<TaskRecord> {
    object_task_queue::cancel_claim(runtime, project_id, task_id, owner, claim_token, generation)
}
use anyhow::{ensure, Result};

pub use crate::object_task_definition::{
    DefinitionAdopter, RevisePlannedRequest, TaskDefinition, TaskDefinitionRevision,
};
pub use crate::object_task_types::{
    AssumptionSource, CancelPlannedReceipt, CancelPlannedRequest, CommitReceipt, CommitRequest,
    Draft, Granularity, ObjectProposal, PlanAssumption, PlanAssumptionRecord, PlanProposal,
    RunRecord, SaveDraftRequest, Snapshot, TaskProposal, TaskRecord, UnlockDraftRequest,
};

pub fn save_draft(runtime: &ProjectRuntime, request: &SaveDraftRequest) -> Result<Draft> {
    object_task_drafts::save(runtime, request)
}

pub fn unlock_draft(runtime: &ProjectRuntime, request: &UnlockDraftRequest) -> Result<Draft> {
    crate::object_task_draft_unlock::unlock(runtime, request)
}

pub fn get_draft(
    runtime: &ProjectRuntime,
    project_id: &str,
    draft_id: &str,
) -> Result<Option<Draft>> {
    object_task_drafts::get(runtime, project_id, draft_id)
}

pub fn commit(runtime: &ProjectRuntime, request: &CommitRequest) -> Result<CommitReceipt> {
    object_task_commit::commit(runtime, request)
}

pub fn cancel_planned(
    runtime: &ProjectRuntime,
    request: &CancelPlannedRequest,
) -> Result<CancelPlannedReceipt> {
    object_task_cancel::cancel_planned(runtime, request)
}

pub fn revise_planned(
    runtime: &ProjectRuntime,
    request: &RevisePlannedRequest,
) -> Result<TaskDefinitionRevision> {
    object_task_revisions::revise_planned(runtime, request)
}

pub fn revisions(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
) -> Result<Vec<TaskDefinitionRevision>> {
    object_task_revisions::history(runtime, project_id, task_id)
}

pub fn snapshot(runtime: &ProjectRuntime, project_id: &str) -> Result<Snapshot> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    let transaction = store.connection.unchecked_transaction()?;
    let state = object_task_storage::plan_state(&transaction, project_id)?;
    let mut tasks = object_task_storage::all_tasks(&transaction)?
        .into_iter()
        .filter(|task| task.project_id == project_id)
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.id.cmp(&right.id))
    });
    let runs = object_task_storage::all_runs(&transaction)?
        .into_iter()
        .filter(|run| run.project_id == project_id)
        .collect();
    let dispatch_controls = crate::object_task_dispatch::snapshot_in(&transaction, &tasks)?;
    let coarse_dispatch_controls =
        crate::object_task_coarse_dispatch::snapshot_in(&transaction, &tasks)?;
    let planning_states =
        crate::object_task_planning_declaration::snapshot_in(&transaction, &tasks)?;
    transaction.commit()?;
    Ok(Snapshot {
        planning_states,
        plan_revision: state.revision,
        tasks,
        runs,
        assumptions: state.assumptions,
        dispatch_controls,
        coarse_dispatch_controls,
    })
}

pub fn get_task(
    runtime: &ProjectRuntime,
    project_id: &str,
    task_id: &str,
) -> Result<Option<TaskRecord>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(valid_id(task_id), "INVALID_OBJECT_TASK_ID: task id");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    let task = store.get::<TaskRecord>(TASK_KIND, task_id)?;
    if let Some(task) = &task {
        ensure!(task.id == task_id, "OBJECT_TASK_IDENTITY_MISMATCH");
    }
    Ok(task.filter(|task| task.project_id == project_id))
}

pub fn get_run(
    runtime: &ProjectRuntime,
    project_id: &str,
    run_id: &str,
) -> Result<Option<RunRecord>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(valid_id(run_id), "INVALID_OBJECT_TASK_ID: run id");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    let run = store.get::<RunRecord>(RUN_KIND, run_id)?;
    if let Some(run) = &run {
        ensure!(run.id == run_id, "OBJECT_TASK_RUN_IDENTITY_MISMATCH");
    }
    Ok(run.filter(|run| run.project_id == project_id))
}

#[cfg(test)]
#[path = "object_task_tests.rs"]
mod tests;
