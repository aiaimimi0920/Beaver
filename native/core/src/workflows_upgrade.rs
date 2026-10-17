//! Explicit host-only NPR upgrades for a project root or one owned, stopped task copy.
use super::package;
use crate::{files::safe_path, journal::Journal, project_runtime::ProjectRuntime, store::Store};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[path = "workflows_upgrade_review.rs"]
mod review;
#[cfg(test)]
#[path = "workflows_upgrade_tests.rs"]
mod tests;
#[path = "workflows_upgrade_transaction.rs"]
mod transaction;
#[path = "workflows_upgrade_package.rs"]
mod trust;

pub(crate) use review::verify as verify_review;

const KIND: &str = "npr-upgrade";
pub(crate) fn blocked(store: &Store, project: &str) -> Result<bool> {
    transaction::pending(&store.connection, project)
}
pub(crate) use transaction::pending as blocked_connection;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    id: String,
    task_id: Option<String>,
    expected_old_commit: Option<String>,
    expected_runtime_sha256: Option<String>,
    operation_id: Option<String>,
}

fn target(
    runtime: &ProjectRuntime,
    store: &Store,
    task_id: Option<&str>,
) -> Result<(PathBuf, Value)> {
    let Some(id) = task_id else {
        return Ok((runtime.project_root().into(), Value::Null));
    };
    crate::asset_task::validate_id(id)?;
    let task: Value = store.get("task", id)?.context("Task not found")?;
    ensure!(
        task["id"] == id && task["projectId"] == runtime.project_id(),
        "Task belongs to another project"
    );
    let recorded = task["workspace"]
        .as_str()
        .context("Task has no recorded workspace")?;
    let root = runtime.files().resolve_workspace(id, Path::new(recorded))?;
    ensure!(
        safe_path(&root, "project.godot")?.is_file(),
        "Task workspace is not a Godot project"
    );
    Ok((root, task))
}

fn quiescence(store: &Store, project_id: &str) -> Result<()> {
    for task in store.list::<Value>("task")? {
        if task["projectId"] != project_id {
            continue;
        }
        ensure!(
            matches!(
                task["status"].as_str(),
                Some(
                    "completed"
                        | "failed"
                        | "interrupted"
                        | "rolledBack"
                        | "cancelled"
                        | "conflict"
                )
            ) || (task["status"] == "waitingChildren" && task["planPaused"] == true),
            "Stop all active project tasks before NPR upgrade: {}",
            task["id"]
        );
        let id = task["id"].as_str().context("Invalid task id")?;
        crate::framework_operations::idle(store, id)?;
        crate::external_run_recovery::require_clear(store, &task)?;
    }
    for kind in [
        "validationRun",
        "object_run",
        "object_task_queue",
        "object_task_planning",
    ] {
        for record in store.list::<Value>(kind)? {
            let status = match kind {
                "object_task_planning" => &record["session"]["status"],
                "object_task_queue" => &record["state"],
                _ => &record["status"],
            };
            ensure!(
                !matches!(
                    status.as_str(),
                    Some(
                        "queued"
                            | "claimed"
                            | "running"
                            | "preparing"
                            | "active"
                            | "awaitingInput"
                            | "waitingChildren"
                    )
                ),
                "Stop active project operation before NPR upgrade: {kind}"
            );
        }
    }
    Ok(())
}

fn review_required(task: &Value) -> bool {
    task["productionWorkflow"]["id"] == super::WORKFLOW
        && task["productionWorkflow"]["sourceCommit"] != package::SOURCE_COMMIT
}

fn description(runtime: &ProjectRuntime, root: &Path, task: &Value) -> Value {
    json!({"projectId":runtime.project_id(),"taskId":task.get("id"),"path":root})
}

fn inspect(
    runtime: &ProjectRuntime,
    store: &mut Store,
    root: &Path,
    task: &Value,
) -> Result<Value> {
    let (config, runtime_hash) = trust::identity(root)?;
    let mut blockers = Vec::new();
    if let Err(error) = quiescence(store, runtime.project_id()) {
        blockers.push(error.to_string());
    }
    if Journal::new(store, &runtime.files()).blocked(runtime.project_id())? {
        blockers
            .push("Project recovery pending; inspect pending operations before upgrading".into());
    }
    if !task.is_null() && !matches!(task["status"].as_str(), Some("interrupted" | "failed")) {
        blockers.push(
            "Task workspace upgrades require an interrupted or failed, unaccepted task".into(),
        );
    }
    if task["accepted"] == true {
        blockers.push("Accepted task workspaces are immutable".into());
    }
    let changed = match trust::plan(root) {
        Ok(plan) => plan.changed,
        Err(error) => {
            blockers.push(error.to_string());
            Vec::new()
        }
    };
    let receipts = store.list::<transaction::Receipt>(KIND)?;
    let latest = receipts.iter().find(|r| {
        r.project_id == runtime.project_id() && r.task_id.as_deref() == task["id"].as_str()
    });
    let review_satisfied = !task.is_null() && review::verify(store, root, task).is_ok();
    let pending: Vec<_> = receipts
        .iter()
        .filter(|r| {
            r.project_id == runtime.project_id()
                && !matches!(r.status.as_str(), "completed" | "rolledBack")
        })
        .map(|r| json!({"operationId":r.operation_id,"taskId":r.task_id,"status":r.status}))
        .collect();
    Ok(
        json!({"target":description(runtime, root, task),"sourceCommit":config["packages"]["npr-characters"]["sourceCommit"],
        "targetCommit":package::SOURCE_COMMIT,"runtimeSha256":runtime_hash,"changedFiles":changed,
        "reviewRequired":review_required(task) && !review_satisfied,"reviewSatisfied":review_satisfied,"frozenWorkflow":task["productionWorkflow"],
        "latestOperation":latest.map(|r| json!({"operationId":r.operation_id,"status":r.status})),
        "currentWorkflow":crate::task_workflows::selection(root, super::WORKFLOW).ok(),
        "eligible":blockers.is_empty(),"blockers":blockers,"pendingOperations":pending,
        "reviewNotice":"Frozen plans and accepted artifacts are preserved. A frozen NPR plan with another pin requires a new reviewed plan before resumption."}),
    )
}

/// Requires an already opened project runtime, retaining its exclusive process writer lease.
/// Store ownership is held across preflight, backup, mutation and durable result publication.
pub fn call(runtime: &ProjectRuntime, method: &str, input: Value) -> Result<Value> {
    ensure!(
        input["id"] == runtime.project_id(),
        "Project runtime mismatch"
    );
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Project recovery busy"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Project materialization busy"))?;
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
    if method == "project.npr.reviewUpgrade" {
        let request: review::Request = serde_json::from_value(input)?;
        quiescence(&store, runtime.project_id())?;
        ensure!(
            !Journal::new(&mut store, &runtime.files()).blocked(runtime.project_id())?,
            "Project recovery pending"
        );
        let (root, task) = target(runtime, &store, Some(&request.task_id))?;
        return review::record(&store, &root, &task, request);
    }
    let request: Request = serde_json::from_value(input)?;
    ensure!(
        request.id == runtime.project_id(),
        "Project runtime mismatch"
    );
    if method == "project.npr.rollbackUpgrade" {
        let id = request
            .operation_id
            .as_deref()
            .context("Missing operationId")?;
        crate::asset_task::validate_id(id)?;
        let mut receipt: transaction::Receipt =
            store.get(KIND, id)?.context("Unknown upgrade operation")?;
        ensure!(
            receipt.project_id == runtime.project_id(),
            "Upgrade belongs to another project"
        );
        quiescence(&store, runtime.project_id())?;
        let (root, task) = target(runtime, &store, receipt.task_id.as_deref())?;
        ensure!(
            task.is_null()
                || (matches!(task["status"].as_str(), Some("interrupted" | "failed"))
                    && task["accepted"] != true),
            "Task rollback requires a stopped, unaccepted workspace"
        );
        if receipt.status != "rolledBack" {
            transaction::rollback(&store, &runtime.files(), &root, &mut receipt)?;
        }
        return Ok(
            json!({"operationId":id,"status":receipt.status,"target":description(runtime,&root,&task),"runtimeSha256":trust::identity(&root)?.1}),
        );
    }
    let (root, task) = target(runtime, &store, request.task_id.as_deref())?;
    if method == "project.npr.inspectUpgrade" {
        return inspect(runtime, &mut store, &root, &task);
    }
    ensure!(
        method == "project.npr.upgrade",
        "Unknown NPR upgrade method"
    );
    let inspected = inspect(runtime, &mut store, &root, &task)?;
    ensure!(
        inspected["eligible"] == true,
        "NPR upgrade blocked: {}",
        inspected["blockers"]
    );
    ensure!(
        request.expected_old_commit.as_deref() == inspected["sourceCommit"].as_str(),
        "Unexpected old NPR commit"
    );
    ensure!(
        request.expected_runtime_sha256.as_deref() == inspected["runtimeSha256"].as_str(),
        "Runtime CAS failed; inspect again"
    );
    let plan = trust::plan(&root)?;
    ensure!(
        request.expected_runtime_sha256.as_deref() == Some(plan.runtime_hash.as_str()),
        "Runtime changed after inspection"
    );
    let project: Value = store
        .get("project", runtime.project_id())?
        .context("Project not found")?;
    let mut receipt =
        transaction::prepare(&runtime.files(), &root, &project, request.task_id, &plan)?;
    transaction::apply(&store, &runtime.files(), &root, &mut receipt)?;
    Ok(
        json!({"operationId":receipt.operation_id,"status":receipt.status,"target":description(runtime,&root,&task),
        "runtime":receipt.runtime,"runtimeSha256":trust::identity(&root)?.1,"changedFiles":plan.changed,
        "reviewRequired":review_required(&task),"frozenWorkflow":task["productionWorkflow"],
        "validation":"Dependency integrity verified; model validation and visual review remain separate"}),
    )
}
