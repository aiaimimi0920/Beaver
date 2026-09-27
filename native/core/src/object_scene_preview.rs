//! Frozen scene captures reuse the durable visual worker, never acceptance flows.
use crate::{
    object_catalog::{self, ObjectVersion},
    object_import_content, object_import_snapshot, object_run_baseline,
    project_runtime::ProjectRuntime,
    validation::repository,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    pub project_id: String,
    pub object_id: String,
    pub version_id: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    project_id: String,
    request_id: String,
    target: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolution: Option<crate::frozen_scene_preview::Resolution>,
}

fn key(target: &Target) -> Result<String> {
    repository::digest(target)
}

pub fn latest(runtime: &ProjectRuntime, target: &Target) -> Result<Value> {
    crate::frozen_scene_preview::latest(runtime, &serde_json::to_value(target)?, &key(target)?)
}

pub fn enqueue(runtime: &ProjectRuntime, input: &Value) -> Result<Value> {
    let input: Input = serde_json::from_value(input.clone())?;
    let target = &input.target;
    crate::frozen_scene_preview::enqueue(
        runtime,
        &serde_json::to_value(&input)?,
        "object.scenePreview.run",
        key(target)?,
        |store| {
            let objects = object_catalog::search(&store, &target.project_id, None)?;
            let selection = object_import_snapshot::select_frozen(
                &objects,
                &target.project_id,
                &target.object_id,
                &target.version_id,
            )?;
            for manifest in &selection.versions {
                let object = objects
                    .iter()
                    .find(|object| object.id == manifest.object_id)
                    .context("OBJECT_NOT_FOUND")?;
                let version = object
                    .versions
                    .iter()
                    .find(|version| version.version_id == manifest.version_id)
                    .context("OBJECT_VERSION_NOT_FOUND")?;
                let saved: (String, ObjectVersion) = store
                    .get("object_version", &manifest.version_id)?
                    .context("OBJECT_VERSION_NOT_FOUND")?;
                ensure!(
                    saved == (object.id.clone(), version.clone()),
                    "OBJECT_VERSION_MISMATCH"
                );
            }
            let selected = selection
                .versions
                .iter()
                .find(|version| {
                    version.object_id == target.object_id && version.version_id == target.version_id
                })
                .context("OBJECT_VERSION_NOT_FOUND")?;
            ensure!(
                selected
                    .files
                    .iter()
                    .any(|file| file.path == target.path && file.sha256 == target.sha256),
                "PREVIEW_SCENE_NOT_IN_VERSION"
            );
            let snapshot = object_run_baseline::snapshot(&selection.versions)?;
            object_import_content::verify(runtime.project_root(), &selection.versions)?;
            Ok((snapshot, selection.digest))
        },
    )
}

#[cfg(test)]
#[path = "object_scene_preview_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "object_blender_preview_tests.rs"]
mod blender_tests;
