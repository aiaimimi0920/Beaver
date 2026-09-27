//! Archived attempt-output frames remain bound to their original checkpoint after publication.
use super::super::followup::frames::{self, Reference};
use crate::{
    object_attempt::Attempt,
    object_attempt_file::{Checkpoint, Request},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection};
use serde_json::Value;

pub(crate) fn resolve(db: &Connection, source: &Attempt, reference: &Reference) -> Result<Value> {
    let project = &source.preparation.project_id;
    let item = frames::load(db, project, reference)?;
    let target: Request = serde_json::from_value(item["source"]["target"].clone())?;
    let output = source
        .output
        .as_ref()
        .context("OBJECT_ATTEMPT_FILE_OUTPUT_UNAVAILABLE")?;
    ensure!(
        target.project_id == *project
            && target.attempt_id == source.id
            && target.run_id == source.preparation.run.id
            && matches!(target.checkpoint, Checkpoint::Output)
            && output.get(&target.path) == Some(&target.sha256)
            && item["source"]["sourceDigest"] == crate::framework_checks::digest(output)?,
        "PREVIEW_FEEDBACK_ATTEMPT_MISMATCH"
    );
    Ok(item)
}

pub fn list(
    runtime: &ProjectRuntime,
    project: &str,
    attempt: &str,
    reference: Option<&Reference>,
) -> Result<Vec<Value>> {
    ensure!(runtime.project_id() == project, "PROJECT_RUNTIME_MISMATCH");
    super::super::transact(runtime, |db| {
        let source = crate::object_attempt_view::read(db, project, attempt)?;
        if let Some(reference) = reference {
            return Ok(vec![resolve(db, &source, reference)?]);
        }
        let mut query = db.prepare(
            "SELECT json_extract(frame.value,'$.runId'), json_extract(frame.value,'$.id')
             FROM entities AS archive, json_each(archive.value) AS frame
             WHERE archive.kind='objectPreviewFrames'
               AND json_extract(frame.value,'$.source.target.projectId')=?
               AND json_extract(frame.value,'$.source.target.attemptId')=?
               AND json_extract(frame.value,'$.source.target.checkpoint')='output'
               AND json_type(frame.value,'$.selection')='object'
             ORDER BY json_extract(frame.value,'$.savedAt') DESC,
                      json_extract(frame.value,'$.id') DESC LIMIT 8",
        )?;
        let refs = query
            .query_map(params![project, attempt], |row| {
                Ok(Reference {
                    run_id: row.get(0)?,
                    frame_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        refs.iter()
            .map(|reference| resolve(db, &source, reference))
            .collect()
    })
}
