//! Asset identity conversion; file paths and runtime recovery remain separate stages.
use crate::{
    asset_delivery_files::Candidate,
    asset_delivery_review::Decision,
    asset_feedback::Submission,
    asset_task::{Feedback, Reference, State},
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

fn reference(rewrite: &Rewrite<'_>, value: &mut Value) -> Result<()> {
    let _: Reference = serde_json::from_value(value.clone())?;
    for (field, kind) in [
        ("/id", "asset-reference"),
        ("/taskId", "task"),
        ("/projectId", "project"),
    ] {
        rewrite.one(value, field, kind)?;
    }
    Ok(())
}

fn submission(feedback: &Feedback) -> Submission {
    Submission {
        id: feedback.source_task_id.clone(),
        feedback_id: feedback.id.clone(),
        timing: feedback.timing.clone(),
        text: feedback.text.clone(),
        reference_id: feedback.reference.id.clone(),
        annotations: feedback.annotations.clone(),
    }
}

pub(crate) fn feedback(rewrite: &Rewrite<'_>, value: &mut Value) -> Result<()> {
    let old: Feedback = serde_json::from_value(value.clone())?;
    ensure!(
        old.fingerprint == submission(&old).fingerprint()?,
        "feedback fingerprint mismatch"
    );
    rewrite.one(value, "/sourceTaskId", "task")?;
    rewrite.one(value, "/taskId", "task")?;
    reference(rewrite, &mut value["reference"])?;
    let converted: Feedback = serde_json::from_value(value.clone())?;
    value["fingerprint"] = Value::String(submission(&converted).fingerprint()?);
    Ok(())
}

/// Converts only declared identities using a verified map, preserving unknown JSON fields.
pub fn rewrite(map: &IdentityMap, kind: &str, id: &str, original: &Value) -> Result<(Key, Value)> {
    ensure!(
        map.format == "beaver-project-derivation-identities-v1",
        "unsupported identity map"
    );
    let rewrite = Rewrite(map);
    let key = rewrite.key(kind, id)?;
    let mut value = original.clone();
    match kind {
        "asset-reference" => {
            ensure!(value["id"] == id, "asset reference key mismatch");
            reference(&rewrite, &mut value)?;
        }
        "asset-task" => {
            let source: State = serde_json::from_value(value.clone())?;
            ensure!(source.task_id == id, "asset state key mismatch");
            rewrite.one(&mut value, "/taskId", "task")?;
            rewrite.one(&mut value, "/projectId", "project")?;
            if let Some(frame) = value.get_mut("lastFrame").filter(|v| !v.is_null()) {
                reference(&rewrite, frame)?;
            }
            for item in value["feedback"]
                .as_array_mut()
                .context("feedback list missing")?
            {
                feedback(&rewrite, item)?;
            }
        }
        _ => {
            let (prefix, task) = kind.split_once('/').context("unsupported asset kind")?;
            ensure!(
                key.kind == format!("{prefix}/{}", rewrite.key("task", task)?.id) && key.id == id,
                "asset namespace mapping mismatch"
            );
            match prefix {
                "asset-delivery" => {
                    let candidate: Candidate = serde_json::from_value(value.clone())?;
                    ensure!(
                        candidate.id == id && candidate.task_id == task,
                        "candidate ownership mismatch"
                    );
                    rewrite.one(&mut value, "/taskId", "task")?;
                }
                "asset-delivery-decisions" => {
                    let request: Decision = serde_json::from_value(value["request"].clone())?;
                    ensure!(
                        request.id == task && request.request_id == id,
                        "decision ownership mismatch"
                    );
                    rewrite.one(&mut value, "/request/id", "task")?;
                }
                _ => bail!("unsupported asset derivation record: {kind}"),
            }
        }
    }
    Ok((key, value))
}

#[cfg(test)]
#[path = "project_derivation_asset_records_tests.rs"]
mod tests;
