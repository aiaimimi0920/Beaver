//! Rework attachment authority follows validated resume receipts, including plain retries.
use crate::{object_attempt::Attempt, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Binding {
    rework_request_id: String,
    source_attempt_id: String,
    preview_frame: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relocation: Option<super::relocation::Resolved>,
    #[serde(skip)]
    pub data_url: String,
}

pub(crate) fn resolve(runtime: &ProjectRuntime, attempt: &Attempt) -> Result<Option<Binding>> {
    let project = &attempt.preparation.project_id;
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    super::super::super::transact(runtime, |db| {
        let mut current = attempt.clone();
        let mut visited = BTreeSet::new();
        loop {
            ensure!(
                visited.insert(current.id.clone()),
                "OBJECT_RECOVERY_HISTORY_MISMATCH"
            );
            let mut statement = db.prepare(
                "SELECT json_extract(value,'$.operation.request.requestId') FROM entities
                 WHERE kind='object_recovery_resume'
                 AND json_extract(value,'$.operation.request.projectId')=?
                 AND json_extract(value,'$.started.id')=?",
            )?;
            let ids = statement
                .query_map(rusqlite::params![project, current.id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ensure!(ids.len() <= 1, "OBJECT_RECOVERY_HISTORY_MISMATCH");
            let Some(id) = ids.first() else {
                return Ok(None);
            };
            let saved = super::super::storage::read(db, project, id)?
                .context("OBJECT_RECOVERY_RESUME_RECEIPT_MISSING")?;
            let started = saved
                .started
                .as_ref()
                .context("OBJECT_RECOVERY_HISTORY_MISSING")?;
            ensure!(
                started.id == current.id
                    && started.preparation == current.preparation
                    && started.fine == current.fine
                    && started.input == current.input,
                "OBJECT_FEEDBACK_IMAGE_SCOPE_MISMATCH"
            );
            let source = saved
                .verification
                .records
                .attempt
                .context("OBJECT_ATTEMPT_NOT_FOUND")?;
            if let Some(approval) = saved.operation.request.rework {
                let Some(reference) = approval.preview_frame else {
                    return Ok(None);
                };
                let mut frame =
                    super::super::super::candidate::publication::deferred::frames::resolve(
                        db, &source, &reference,
                    )?;
                let data_url = frame["frame"]
                    .as_object_mut()
                    .context("PREVIEW_FRAME_PNG_REQUIRED")?
                    .remove("dataUrl")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .context("PREVIEW_FRAME_PNG_REQUIRED")?;
                let relocation = approval
                    .relocation
                    .as_ref()
                    .map(|confirmation| {
                        super::relocation::resolve(db, &source, &reference, confirmation)
                    })
                    .transpose()?;
                return Ok(Some(Binding {
                    rework_request_id: id.clone(),
                    source_attempt_id: source.id,
                    preview_frame: frame,
                    data_url,
                    relocation,
                }));
            }
            if saved.operation.request.advance.is_some() {
                return Ok(None);
            }
            current = source;
        }
    })
}
