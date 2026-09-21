use crate::{
    asset_feedback::{self, Submission},
    asset_reference,
    asset_task::{self, Feedback, Frame},
    files::Files,
    store::Store,
    task_relations,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

/// Caller holds the task's store boundary, also used by final delivery.
pub fn submit(
    store: &mut Store,
    files: &Files,
    input: Submission,
    live: Option<&Frame>,
    designs: &Value,
    blueprints: &Value,
) -> Result<Feedback> {
    input.validate()?;
    let task: Value = store
        .get("task", &input.id)?
        .context("Task no longer exists")?;
    crate::object_framework::require_legacy(&task)?;
    let mut state = asset_task::get(store, &input.id)?;
    if let Some(existing) = asset_feedback::duplicate(&state, &input)? {
        return Ok(existing);
    }
    let delivered = matches!(task["status"].as_str(), Some("completed" | "rolledBack"));
    // Task creation persists the seed atomically. Repair the source receipt after a crash.
    if delivered {
        for destination in store.list::<Value>("task")? {
            let seed = &destination["assetFeedbackSeed"];
            if seed["sourceTaskId"] == input.id && seed["id"] == input.feedback_id {
                let mut feedback: Feedback = serde_json::from_value(seed.clone())?;
                if feedback.fingerprint != input.fingerprint()? {
                    bail!("Feedback ID already belongs to different content");
                }
                feedback.task_id = destination["id"]
                    .as_str()
                    .context("Missing destination")?
                    .into();
                feedback.status = "forwarded".into();
                asset_task::enable(store, &destination)?;
                state.feedback.push(feedback.clone());
                asset_task::save(store, &state)?;
                return Ok(feedback);
            }
        }
    }
    let reference = asset_reference::get(store, &input.id, &input.reference_id)?;
    if reference.project_id != state.project_id {
        bail!("Reference belongs to another project");
    }
    asset_reference::read(files, &reference)?;
    if !delivered {
        if state.session_id.as_deref() != Some(reference.frame.session_id.as_str()) {
            bail!("Preview session changed; capture a new reference");
        }
        if reference.pick.is_some() {
            let current = live.context("Cannot verify this pick while Blender is disconnected")?;
            asset_reference::same_scene(&reference.frame, current)?;
        }
        return asset_feedback::accept(store, input, reference);
    }
    if state.checkpoint.is_none() {
        bail!("No saved asset scene is available for a follow-up; continue through task.followup and inspect the exported resources");
    }
    let mut feedback = asset_feedback::build(&mut state.clone(), input.clone(), reference)?;
    // Pin before creating the queued destination, so its multimodal input remains readable.
    store.put(
        "asset-reference",
        &feedback.reference.id,
        &feedback.reference,
    )?;
    let destination = task_relations::create_seeded(
        store,
        files,
        "task.followup",
        json!({"id":input.id,"text":input.text}),
        designs,
        blueprints,
        Some((&feedback, state.checkpoint.as_deref())),
    )?;
    feedback.task_id = destination["id"]
        .as_str()
        .context("Missing follow-up ID")?
        .into();
    feedback.round = 1;
    feedback.status = "forwarded".into();
    state.feedback.push(feedback.clone());
    asset_task::save(store, &state)?;
    asset_task::enable(store, &destination)?;
    Ok(feedback)
}
