//! Framework ownership and candidate digest conversion; runtime/path recovery is separate.
use crate::{
    asset_delivery_files::Candidate,
    framework_checks::digest,
    framework_contract::{Configuration, Job},
    framework_operations::Operation,
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, ensure, Context, Result};
use rusqlite::Connection;
use serde_json::Value;

fn context(rewrite: &Rewrite<'_>, value: &mut Value, task: &str) -> Result<()> {
    ensure!(value["taskId"] == task, "framework context owner mismatch");
    rewrite.one(value, "/taskId", "task")?;
    rewrite.one(value, "/projectId", "project")
}

fn candidate_digest(
    db: &Connection,
    rewrite: &Rewrite<'_>,
    task: &str,
    value: &mut Value,
) -> Result<()> {
    let id = value["candidateId"]
        .as_str()
        .context("framework candidate missing")?;
    let candidate: Candidate =
        crate::task_callback::get(db, &format!("asset-delivery/{task}"), id)?
            .context("framework candidate not found")?;
    ensure!(
        candidate.id == id && candidate.task_id == task,
        "framework candidate owner mismatch"
    );
    // Running check placeholders have no result digest yet.
    if value.get("candidateSha256").is_none() {
        ensure!(
            value["status"] == "running" && value["passed"] == false,
            "framework candidate digest missing"
        );
        return Ok(());
    }
    ensure!(
        value["candidateSha256"] == digest(&candidate)?,
        "framework candidate digest mismatch"
    );
    let mut converted = candidate;
    converted.task_id = rewrite.key("task", task)?.id;
    value["candidateSha256"] = Value::String(digest(&converted)?);
    // Configuration history is not rewritten: old revision/hash pairs must remain stale.
    Ok(())
}

pub fn rewrite(
    db: &Connection,
    map: &IdentityMap,
    kind: &str,
    id: &str,
    original: &Value,
) -> Result<(Key, Value)> {
    ensure!(
        map.format == "beaver-project-derivation-identities-v1",
        "unsupported identity map"
    );
    let rewrite = Rewrite(map);
    let key = rewrite.key(kind, id)?;
    let mut value = original.clone();
    match kind {
        "framework-configuration" => {
            let _: Configuration = serde_json::from_value(value.clone())?;
            ensure!(
                key.id == rewrite.key("task", id)?.id,
                "configuration owner mismatch"
            );
        }
        "framework-operation" => {
            let operation: Operation = serde_json::from_value(value.clone())?;
            ensure!(operation.id == id, "framework operation key mismatch");
            rewrite.one(&mut value, "/id", kind)?;
            rewrite.one(&mut value, "/taskId", "task")?;
            if !matches!(operation.job, Job::Callback { .. }) && !value["result"].is_null() {
                context(
                    &rewrite,
                    &mut value["result"]["context"],
                    &operation.task_id,
                )?;
                if let Job::Check { candidate_id } = operation.job {
                    ensure!(
                        value["result"]["value"]["candidateId"] == candidate_id,
                        "operation candidate mismatch"
                    );
                    candidate_digest(
                        db,
                        &rewrite,
                        &operation.task_id,
                        &mut value["result"]["value"],
                    )?;
                }
            }
        }
        _ => {
            let (prefix, task) = kind.split_once('/').context("unsupported framework kind")?;
            ensure!(
                key.kind == format!("{prefix}/{}", rewrite.key("task", task)?.id),
                "framework namespace mismatch"
            );
            match prefix {
                "framework-trace" | "framework-observation" | "framework-recovery" => {
                    context(&rewrite, &mut value["context"], task)?;
                    match prefix {
                        "framework-trace" => {
                            ensure!(value["id"] == id, "trace key mismatch");
                            if let Some(call) = map.calls.get(id) {
                                ensure!(*call == key.id, "trace call mapping mismatch");
                            }
                            value["id"] = Value::String(key.id.clone());
                        }
                        "framework-observation" => {
                            ensure!(value["referenceId"] == id, "observation key mismatch");
                            rewrite.one(&mut value, "/referenceId", "asset-reference")?;
                            ensure!(
                                value["referenceId"] == key.id,
                                "mapped observation key mismatch"
                            );
                        }
                        _ => {
                            rewrite.one(&mut value, "/operationId", "framework-operation")?;
                        }
                    }
                }
                "framework-check" | "framework-judgment" | "framework-judgment-history" => {
                    ensure!(key.id == id, "framework local key mismatch");
                    if prefix == "framework-check" {
                        ensure!(value["candidateId"] == id, "check key mismatch");
                    }
                    if prefix == "framework-judgment" {
                        let source = value["source"]
                            .as_str()
                            .context("judgment source missing")?;
                        let candidate = value["candidateId"]
                            .as_str()
                            .context("judgment candidate missing")?;
                        ensure!(
                            id == format!("{source}-{candidate}"),
                            "judgment key mismatch"
                        );
                    }
                    candidate_digest(db, &rewrite, task, &mut value)?;
                }
                _ => bail!("unsupported framework derivation record: {kind}"),
            }
        }
    }
    Ok((key, value))
}

#[cfg(test)]
#[path = "project_derivation_framework_records_tests.rs"]
mod tests;
