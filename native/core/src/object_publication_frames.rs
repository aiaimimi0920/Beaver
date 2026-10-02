//! Frozen viewer frames may authorize only follow-up work for their exact published version.
use super::{storage, tasks, transact, State};
use crate::project_runtime::ProjectRuntime;
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub run_id: String,
    pub frame_id: String,
}

pub(crate) fn resolve(
    db: &Connection,
    project: &str,
    object: &str,
    version: &str,
    reference: &Reference,
) -> Result<Value> {
    let item = load(db, project, reference)?;
    let target = &item["source"]["target"];
    ensure!(
        target["objectId"] == object
            && target["versionId"] == version
            && target.get("attemptId").is_none(),
        "PREVIEW_FEEDBACK_SOURCE_MISMATCH"
    );
    Ok(item)
}

pub(crate) fn load(db: &Connection, project: &str, reference: &Reference) -> Result<Value> {
    ensure!(
        crate::object_task_types::valid_id(&reference.run_id)
            && crate::object_task_types::valid_id(&reference.frame_id),
        "INVALID_PREVIEW_FRAME_REFERENCE"
    );
    let saved = tasks::read::<Vec<Value>>(db, "objectPreviewFrames", &reference.run_id)?
        .context("PREVIEW_SAVED_NOT_FOUND")?;
    let item = saved
        .into_iter()
        .find(|item| item["id"] == reference.frame_id)
        .context("PREVIEW_SAVED_NOT_FOUND")?;
    let run = tasks::read::<Value>(db, "validationRun", &reference.run_id)?
        .context("PREVIEW_RUN_NOT_FOUND")?;
    let target = &item["source"]["target"];
    ensure!(
        item["projectId"] == project
            && item["runId"] == reference.run_id
            && item["source"]["runId"] == reference.run_id
            && run["id"] == reference.run_id
            && run["projectId"] == project
            && run["kind"] == "objectPreview"
            && run["status"] == "completed"
            && item["snapshotId"] == run["snapshotId"]
            && target["projectId"] == project,
        "PREVIEW_FEEDBACK_SOURCE_MISMATCH"
    );
    let path = target["path"].as_str().context("PREVIEW_SOURCE_MISSING")?;
    ensure!(
        target["sha256"].is_string() && run["snapshot"][path] == target["sha256"],
        "PREVIEW_SOURCE_HASH_MISMATCH"
    );
    crate::validation::preview_frames::validate_saved(&item)?;
    ensure!(
        item.get("selection").is_some(),
        "PREVIEW_FEEDBACK_SELECTION_REQUIRED"
    );
    Ok(item)
}

pub fn list(runtime: &ProjectRuntime, project: &str, publication: &str) -> Result<Vec<Value>> {
    list_exact(runtime, project, publication, None)
}

pub fn list_exact(
    runtime: &ProjectRuntime,
    project: &str,
    publication: &str,
    reference: Option<&Reference>,
) -> Result<Vec<Value>> {
    super::validate(runtime, project, publication)?;
    transact(runtime, |db| {
        let saved =
            storage::read(db, project, publication)?.context("OBJECT_PUBLICATION_NOT_FOUND")?;
        ensure!(
            saved.operation.state == State::Published,
            "OBJECT_PUBLICATION_NOT_PUBLISHED"
        );
        let object = &saved.operation.request.target.object_id;
        let version = &saved.operation.version_id;
        if let Some(reference) = reference {
            return Ok(vec![resolve(db, project, object, version, reference)?]);
        }
        // Select only references before decoding PNGs; never materialize all archived images.
        let mut query = db.prepare(
            "SELECT json_extract(frame.value, '$.runId'), json_extract(frame.value, '$.id')
             FROM entities AS archive, json_each(archive.value) AS frame
             WHERE archive.kind = 'objectPreviewFrames'
               AND json_extract(frame.value, '$.source.target.projectId') = ?
               AND json_extract(frame.value, '$.source.target.objectId') = ?
               AND json_extract(frame.value, '$.source.target.versionId') = ?
               AND json_type(frame.value, '$.selection') = 'object'
             ORDER BY json_extract(frame.value, '$.savedAt') DESC,
                      json_extract(frame.value, '$.id') DESC LIMIT 8",
        )?;
        let references = query
            .query_map(params![project, object, version], |row| {
                Ok(Reference {
                    run_id: row.get(0)?,
                    frame_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut result = vec![];
        for reference in references {
            result.push(resolve(db, project, object, version, &reference)?);
        }
        Ok(result)
    })
}
