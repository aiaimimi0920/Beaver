//! Render immutable attempt checkpoints through the durable preview worker.
use crate::{
    object_attempt_file::{Checkpoint, Request as Target},
    project_runtime::ProjectRuntime,
    validation::repository,
};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    project_id: String,
    request_id: String,
    target: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolution: Option<crate::frozen_scene_preview::Resolution>,
}

pub fn latest(runtime: &ProjectRuntime, target: &Target) -> Result<Value> {
    crate::frozen_scene_preview::latest(
        runtime,
        &serde_json::to_value(target)?,
        &repository::digest(target)?,
    )
}

pub fn enqueue(runtime: &ProjectRuntime, input: &Value) -> Result<Value> {
    let input: Input = serde_json::from_value(input.clone())?;
    let target = &input.target;
    crate::frozen_scene_preview::enqueue(
        runtime,
        &serde_json::to_value(&input)?,
        "object.attemptScenePreview.run",
        repository::digest(target)?,
        |store| {
            let attempt = crate::object_attempt_view::read(
                &store.connection,
                &target.project_id,
                &target.attempt_id,
            )?;
            ensure!(
                attempt.preparation.run.id == target.run_id,
                "OBJECT_ATTEMPT_FILE_RUN_MISMATCH"
            );
            let snapshot = match target.checkpoint {
                Checkpoint::Input => &attempt.input,
                Checkpoint::Output => attempt
                    .output
                    .as_ref()
                    .context("OBJECT_ATTEMPT_FILE_OUTPUT_UNAVAILABLE")?,
            };
            let hash = snapshot
                .get(&target.path)
                .context("OBJECT_ATTEMPT_FILE_NOT_IN_CHECKPOINT")?;
            ensure!(hash == &target.sha256, "OBJECT_ATTEMPT_FILE_STALE_HASH");
            crate::object_attempt_checks::verify_snapshot(runtime, snapshot)?;
            Ok((snapshot.clone(), repository::digest(snapshot)?))
        },
    )
}
