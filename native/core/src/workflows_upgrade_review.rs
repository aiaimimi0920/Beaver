//! A separate reviewed execution binding; original plans and their frozen bindings stay immutable.
use super::{transaction::Receipt, trust, KIND};
use crate::{store::Store, task_workflows, workflows::RUNTIME_FILE};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

const REVIEW_KIND: &str = "npr-upgrade-review";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Request {
    pub id: String,
    pub task_id: String,
    operation_id: String,
    expected_runtime_sha256: String,
    expected_frozen_workflow: Value,
    reviewed_workflow: Value,
    review_reason: String,
    acceptance: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Review {
    project_id: String,
    task_id: String,
    operation_id: String,
    workspace: Value,
    original_workflow: Value,
    workflow: Value,
    runtime_sha256: String,
    plan: Value,
    parent_plan: Value,
    prompt: Value,
    review_reason: String,
    acceptance: String,
    reviewed_at: String,
}

fn parent_plan(store: &Store, task: &Value) -> Result<Value> {
    let Some(parent) = task["parentTaskId"].as_str() else { return Ok(Value::Null); };
    let parent: Value = store.get("task", parent)?.context("Parent plan missing")?;
    Ok(json!({"plan":parent["plan"],"workflowBindings":parent["workflowBindings"]}))
}

fn operation(store: &Store, task: &Value, id: &str, runtime_hash: &str) -> Result<()> {
    let receipt: Receipt = store.get(KIND, id)?.context("Upgrade operation missing")?;
    ensure!(receipt.status == "completed" && receipt.project_id == task["projectId"]
        && receipt.task_id.as_deref() == task["id"].as_str(),
        "Review requires a completed upgrade of this exact task workspace");
    ensure!(receipt.changes.iter().any(|change| change.path == RUNTIME_FILE
        && change.after.as_deref() == Some(runtime_hash)), "Review runtime does not match the successful upgrade");
    Ok(())
}

pub(super) fn record(store: &Store, root: &Path, task: &Value, request: Request) -> Result<Value> {
    ensure!(task["id"] == request.task_id && task["projectId"] == request.id, "Review target mismatch");
    ensure!(matches!(task["status"].as_str(), Some("interrupted" | "failed")) && task["accepted"] != true,
        "Only an unaccepted stopped task can receive a dependency review");
    ensure!(task["productionWorkflow"] == request.expected_frozen_workflow
        && task["productionWorkflow"]["id"] == crate::workflows::WORKFLOW,
        "Frozen workflow CAS failed");
    for text in [&request.review_reason, &request.acceptance] {
        ensure!(!text.trim().is_empty() && text.len() <= 16000, "Review reason and acceptance are required and bounded");
    }
    let (_, runtime_hash) = trust::identity(root)?;
    ensure!(runtime_hash == request.expected_runtime_sha256, "Review runtime CAS failed");
    let workflow = task_workflows::selection(root, crate::workflows::WORKFLOW)?;
    ensure!(workflow == request.reviewed_workflow, "Review must explicitly match the current compiled workflow");
    operation(store, task, &request.operation_id, &runtime_hash)?;
    let review = Review { project_id: request.id, task_id: request.task_id.clone(),
        operation_id: request.operation_id, workspace: task["workspace"].clone(),
        original_workflow: task["productionWorkflow"].clone(), workflow,
        runtime_sha256: runtime_hash, plan: task["plan"].clone(), parent_plan: parent_plan(store, task)?,
        prompt: task["prompt"].clone(), review_reason: request.review_reason,
        acceptance: request.acceptance, reviewed_at: chrono::Utc::now().to_rfc3339() };
    // Append-only history plus one current pointer; neither task nor parent plan is rewritten.
    let history_id = uuid::Uuid::new_v4().to_string();
    store.put("npr-upgrade-review-history", &history_id, &review)?;
    store.put(REVIEW_KIND, &request.task_id, &review)?;
    Ok(json!({"status":"reviewed","reviewId":history_id,"taskId":request.task_id,
        "workflow":review.workflow,"runtimeSha256":review.runtime_sha256,"resumed":false}))
}

pub(crate) fn verify(store: &Store, root: &Path, task: &Value) -> Result<()> {
    let id = task["id"].as_str().context("Task id missing")?;
    let review: Review = store.get(REVIEW_KIND, id)?.context("NPR dependency changed; explicitly review the completed task upgrade before resuming")?;
    let (_, runtime_hash) = trust::identity(root)?;
    ensure!(review.task_id == id && review.project_id == task["projectId"]
        && review.workspace == task["workspace"] && review.original_workflow == task["productionWorkflow"]
        && review.plan == task["plan"] && review.parent_plan == parent_plan(store, task)?
        && review.runtime_sha256 == runtime_hash,
        "NPR dependency review is stale; inspect and review again");
    operation(store, task, &review.operation_id, &runtime_hash)?;
    ensure!(task_workflows::selection(root, crate::workflows::WORKFLOW)? == review.workflow,
        "Reviewed NPR workflow changed; review again");
    Ok(())
}
