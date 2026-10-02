//! Independent identities for manual, never-dispatched manufacturing plans and their receipts.
use crate::{
    object_run_recovery::resume::derivation::rework::Prompts,
    object_task_definition::TaskDefinitionRevision,
    object_task_draft_unlock::Receipt as UnlockReceipt,
    object_task_types::{
        CancelPlannedReceipt, CommitReceipt, Draft, PlanState, RunRecord, TaskRecord,
    },
    project_derivation_copy::Request,
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, Result};
use serde_json::Value;

pub(crate) fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "object_task"
            | "object_run"
            | "object_task_draft"
            | "object_task_plan_state"
            | "object_task_commit_receipt"
            | "object_task_cancel_receipt"
            | "object_task_definition_revision"
            | "object_task_draft_unlock_receipt"
    )
}

pub(crate) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let ids = PlanIds(request);
    let key = Rewrite(map).key(kind, id)?;
    let value = match kind {
        "object_task" => {
            let mut task: TaskRecord = serde_json::from_value(value.clone())?;
            prompts.task(&mut task);
            ids.task(&mut task)?;
            serde_json::to_value(task)?
        }
        "object_run" => {
            let mut run: RunRecord = serde_json::from_value(value.clone())?;
            ids.run(&mut run)?;
            serde_json::to_value(run)?
        }
        "object_task_draft" => {
            let mut draft: Draft = serde_json::from_value(value.clone())?;
            ids.draft(&mut draft)?;
            serde_json::to_value(draft)?
        }
        "object_task_plan_state" => {
            let mut state: PlanState = serde_json::from_value(value.clone())?;
            state.project_id = request.target_project_id.clone();
            for assumption in &mut state.assumptions {
                ids.provenance(&mut assumption.source_detail)?;
            }
            serde_json::to_value(state)?
        }
        "object_task_commit_receipt" => {
            let mut receipt: CommitReceipt = serde_json::from_value(value.clone())?;
            receipt.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request_id, "object_task_request")?;
            ids.list(&mut receipt.object_ids, "object")?;
            ids.list(&mut receipt.task_ids, "object_task")?;
            for run in &mut receipt.runs {
                ids.run(run)?;
            }
            serde_json::to_value(receipt)?
        }
        "object_task_cancel_receipt" => {
            let mut receipt: CancelPlannedReceipt = serde_json::from_value(value.clone())?;
            receipt.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request_id, "object_task_request")?;
            ids.id(&mut receipt.task_id, "object_task")?;
            serde_json::to_value(receipt)?
        }
        "object_task_definition_revision" => {
            let mut receipt: TaskDefinitionRevision = serde_json::from_value(value.clone())?;
            receipt.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request_id, "object_task_request")?;
            ids.id(&mut receipt.task_id, "object_task")?;
            ids.list(&mut receipt.before.depends_on, "object_task")?;
            ids.list(&mut receipt.after.depends_on, "object_task")?;
            ids.list(&mut receipt.affected_task_ids, "object_task")?;
            serde_json::to_value(receipt)?
        }
        "object_task_draft_unlock_receipt" => {
            let mut receipt: UnlockReceipt = serde_json::from_value(value.clone())?;
            receipt.request.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request.request_id, "object_task_request")?;
            ids.draft(&mut receipt.draft)?;
            serde_json::to_value(receipt)?
        }
        _ => bail!("unsupported derivation plan kind: {kind}"),
    };
    Ok((key, value))
}

#[cfg(all(test, windows))]
#[path = "project_derivation_plan_tests.rs"]
mod tests;
