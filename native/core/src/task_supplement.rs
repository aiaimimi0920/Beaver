//! Withdraw an unexecuted integration supplement without fabricating a delivery.
use crate::{files::Files, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub fn withdrawn(child: &Value, parent: &Value) -> bool {
    child["supplementWithdrawal"]["parentId"] == parent["id"]
        && child["supplementWithdrawal"]["schemaVersion"] == 1
        && child["parentTaskId"] == parent["id"]
        && child["projectId"] == parent["projectId"]
        && child["origin"] == "user"
        && matches!(child["status"].as_str(), Some("failed" | "interrupted"))
        && child["accepted"] != true
}

pub fn withdraw(
    store: &mut Store,
    files: &Files,
    id: &str,
    parent_id: &str,
    reason: &str,
) -> Result<Value> {
    ensure!(
        !reason.trim().is_empty() && reason.chars().count() <= 2000,
        "Withdrawal reason required (max 2000 characters)"
    );
    let mut child: Value = store.get("task", id)?.context("Supplement missing")?;
    let parent: Value = store.get("task", parent_id)?.context("Parent missing")?;
    crate::object_framework::require_legacy(&child)?;
    crate::object_framework::require_legacy(&parent)?;
    ensure!(
        child.get("migrationRetained").is_none() && parent.get("migrationRetained").is_none(),
        "Retained legacy history cannot be changed"
    );
    ensure!(
        child["id"] == id && parent["id"] == parent_id && id != parent_id,
        "Identity mismatch"
    );
    ensure!(
        child["parentTaskId"] == parent_id && child["projectId"] == parent["projectId"],
        "Parent/project mismatch"
    );
    let ids = parent["subtaskIds"]
        .as_array()
        .context("Parent children missing")?;
    let position = ids
        .iter()
        .position(|v| v == id)
        .context("Parent does not own supplement")?;
    ensure!(
        ids.iter().filter(|v| *v == id).count() == 1,
        "Duplicate child identity"
    );
    if withdrawn(&child, &parent) {
        ensure!(
            child["supplementWithdrawal"]["reason"] == reason,
            "Withdrawal already recorded with another reason"
        );
        return Ok(child["supplementWithdrawal"].clone());
    }
    ensure!(
        child["supplementWithdrawal"].is_null(),
        "Invalid existing withdrawal"
    );
    ensure!(
        parent["status"] == "waitingChildren" && parent["integrationValidation"] == true,
        "Parent is not waiting for integration children"
    );
    let planned = parent["plan"]["steps"]
        .as_array()
        .context("Original plan missing")?
        .len();
    ensure!(
        planned > 0 && position >= planned,
        "Original plan steps cannot be withdrawn"
    );
    ensure!(
        child["origin"] == "user" && child["title"] == "落实集成验收期间的补充要求",
        "Not a user integration supplement"
    );
    ensure!(
        store
            .events(parent_id)?
            .iter()
            .any(|e| e.kind == "validationSteering"),
        "Integration steering provenance missing"
    );
    ensure!(
        matches!(child["status"].as_str(), Some("failed" | "interrupted"))
            && child["accepted"] != true,
        "Stop the unaccepted supplement first"
    );
    for key in [
        "threadId",
        "turnId",
        "externalRunId",
        "waitingDelivery",
        "codeValidation",
        "validationRepair",
        "acceptedAt",
        "restartContext",
    ] {
        ensure!(
            child[key].is_null(),
            "Supplement has execution or delivery evidence: {key}"
        );
    }
    ensure!(
        child["sessionHistory"].is_null()
            || child["sessionHistory"]
                .as_array()
                .is_some_and(Vec::is_empty),
        "Supplement has prior session history"
    );
    for key in ["changes", "conflicts"] {
        ensure!(
            child[key].as_array().is_some_and(Vec::is_empty),
            "Supplement has {key}"
        );
    }
    crate::external_run_recovery::require_clear(store, &child)?;
    crate::external_run_recovery::require_clear(store, &parent)?;
    ensure!(
        !crate::journal::Journal::new(store, files)
            .blocked(parent["projectId"].as_str().context("Project missing")?)?,
        "Project recovery is pending"
    );
    for task in store.list::<Value>("task")? {
        ensure!(
            task["parentTaskId"] != id
                && !task["dependsOn"]
                    .as_array()
                    .is_some_and(|deps| deps.iter().any(|v| v == id)),
            "Supplement has descendants or dependents"
        );
    }
    let workspace = files.resolve_workspace(
        id,
        Path::new(child["workspace"].as_str().context("Workspace missing")?),
    )?;
    let actual = files.capture(&workspace)?;
    ensure!(
        serde_json::to_value(actual)? == child["baseline"],
        "Workspace differs from baseline; preserve and review it"
    );
    let now = chrono::Utc::now().to_rfc3339();
    let record = json!({"schemaVersion":1,"parentId":parent_id,"reason":reason,"at":now,"instructionSuperseded":child["prompt"],"workspaceUnchanged":true});
    child["supplementWithdrawal"] = record.clone();
    child["updatedAt"] = json!(now);
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![child.to_string(), id],
        )?;
        for owner in [id, parent_id] {
            db.execute(
                "INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)",
                rusqlite::params![
                    owner,
                    now,
                    "integrationSupplementWithdrawn",
                    json!({"childId":id,"withdrawal":record}).to_string()
                ],
            )?;
        }
        Ok(())
    })?;
    Ok(record)
}
