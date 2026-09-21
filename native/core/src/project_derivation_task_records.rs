//! Task identity conversion only; paths and execution recovery are separate pending stages.
use crate::{
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

fn task(rewrite: &Rewrite<'_>, value: &mut Value) -> Result<()> {
    ensure!(value.is_object(), "invalid derived task");
    for field in ["id", "projectId"] {
        ensure!(value[field].is_string(), "task identity missing: {field}");
    }
    for (path, kind) in [
        ("/id", "task"),
        ("/projectId", "project"),
        ("/parentTaskId", "task"),
        ("/validationFeedbackId", "validationFeedback"),
        ("/validationRepair/runId", "validationRun"),
        ("/codeValidation/runId", "validationRun"),
        ("/feature/previous/taskId", "task"),
    ] {
        rewrite.one(value, path, kind)?;
    }
    for path in ["/subtaskIds", "/dependsOn"] {
        rewrite.many(value, path, "task")?;
    }
    if let Some(seed) = value.get_mut("assetFeedbackSeed").filter(|v| !v.is_null()) {
        crate::project_derivation_asset_records::feedback(rewrite, seed)?;
    }
    Ok(())
}

/// Pure conversion using a verified preparation map. Never enables or resumes imported work.
pub fn rewrite(map: &IdentityMap, kind: &str, id: &str, original: &Value) -> Result<(Key, Value)> {
    ensure!(
        map.format == "beaver-project-derivation-identities-v1",
        "unsupported identity map"
    );
    let rewrite = Rewrite(map);
    let key = rewrite.key(kind, id)?;
    let mut value = original.clone();
    if kind == "task-callback-revision" {
        ensure!(value.as_u64().is_some(), "invalid callback revision");
        return Ok((key, value));
    }
    ensure!(value.is_object(), "invalid task-related record");
    match kind {
        "task" => {
            ensure!(value["id"] == id, "task key mismatch");
            task(&rewrite, &mut value)?;
        }
        "operation" => {
            ensure!(value["id"] == id, "operation key mismatch");
            let after = &value["taskAfter"];
            ensure!(
                value["taskId"].is_string()
                    && value["taskId"] == after["id"]
                    && value["projectId"].is_string()
                    && value["projectId"] == after["projectId"],
                "operation task ownership mismatch"
            );
            rewrite.one(&mut value, "/id", "operation")?;
            rewrite.one(&mut value, "/taskId", "task")?;
            rewrite.one(&mut value, "/projectId", "project")?;
            task(&rewrite, &mut value["taskAfter"])?;
        }
        "feature" => {
            // Feature IDs are semantic names, while the storage key includes project identity.
            let semantic = value["id"].as_str().context("feature ID missing")?;
            let suffix = format!(":{semantic}");
            let project = id.strip_suffix(&suffix).context("feature key mismatch")?;
            ensure!(!semantic.is_empty(), "empty feature ID");
            ensure!(
                key.id == format!("{}:{semantic}", rewrite.key("project", project)?.id),
                "mapped feature key mismatch"
            );
            ensure!(value["taskId"].is_string(), "feature task missing");
            rewrite.one(&mut value, "/taskId", "task")?;
        }
        _ if kind.starts_with("task-callback/") => {
            let owner = kind.strip_prefix("task-callback/").unwrap();
            ensure!(value["taskId"] == owner, "callback ownership mismatch");
            ensure!(value["projectId"].is_string(), "callback project missing");
            let request: crate::task_callback_contract::Request =
                serde_json::from_value(value["request"].clone())?;
            ensure!(
                !matches!(
                    request,
                    crate::task_callback_contract::Request::State
                        | crate::task_callback_contract::Request::Receipt { .. }
                ),
                "read-only callback cannot have a mutation receipt"
            );
            ensure!(
                value["requestId"] == id
                    && value["request"]["requestId"] == id
                    && value["response"]["requestId"] == id,
                "callback request key mismatch"
            );
            ensure!(
                key.kind == format!("task-callback/{}", rewrite.key("task", owner)?.id)
                    && key.id == id,
                "callback namespace mapping mismatch"
            );
            // Requests/responses contain local work IDs and opaque reported data.
            // Preserve their exact bytes at the JSON value level for retry equality.
            rewrite.one(&mut value, "/taskId", "task")?;
            rewrite.one(&mut value, "/projectId", "project")?;
        }
        _ => bail!("unsupported task derivation record: {kind}"),
    }
    Ok((key, value))
}

#[cfg(test)]
#[path = "project_derivation_task_records_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "project_derivation_callback_records_tests.rs"]
mod callback_tests;
