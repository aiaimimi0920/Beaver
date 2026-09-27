//! Owner feedback restarts only the reviewed final fine, preserving all accepted stages.
use super::{records::Records, Request};
use crate::{
    object_attempt::State,
    object_task_types::{valid_id, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[path = "object_rework_frame.rs"]
pub(crate) mod frame;
#[path = "object_rework_image.rs"]
pub mod image;
#[path = "object_feedback_relocation.rs"]
pub mod relocation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub review_request_id: String,
    pub attempt_id: String,
    pub fine_task_id: String,
    pub feedback: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<image::Feedback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_frame: Option<super::super::candidate::publication::followup::frames::Reference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relocation: Option<relocation::Confirmation>,
}

pub(super) fn validate(approval: &Approval) -> Result<()> {
    if let Some(relocation) = &approval.relocation {
        ensure!(
            approval.preview_frame.is_some(),
            "FEEDBACK_RELOCATION_TARGET_REQUIRED"
        );
        relocation.validate()?;
    }
    ensure!(
        approval.image.is_none() || approval.preview_frame.is_none(),
        "OBJECT_REWORK_IMAGE_FRAME_CONFLICT"
    );
    if let Some(image) = &approval.image {
        image.validate()?;
    }
    ensure!(
        valid_id(&approval.review_request_id)
            && valid_id(&approval.attempt_id)
            && valid_id(&approval.fine_task_id)
            && !approval.feedback.trim().is_empty()
            && approval.feedback.len() <= 4000,
        "INVALID_OBJECT_CANDIDATE_REWORK"
    );
    Ok(())
}

fn definition(
    connection: &Connection,
    records: &Records,
    approval: &Approval,
) -> Result<TaskRecord> {
    validate(approval)?;
    let previous = records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    ensure!(
        previous.state == State::AwaitingGate
            && previous.id == approval.attempt_id
            && previous.fine.id == approval.fine_task_id,
        "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH"
    );
    let mut fine = records.fine.clone().context("OBJECT_ATTEMPT_NOT_FOUND")?;
    if let Some(reference) = &approval.preview_frame {
        super::super::candidate::publication::deferred::frames::resolve(
            connection, previous, reference,
        )?;
        if let Some(relocation) = &approval.relocation {
            self::relocation::resolve(connection, previous, reference, relocation)?;
            fine.prompt.push_str("\n\nOwner-confirmed historical region correspondence is attached to feedbackImage. Inspect both labeled images and the zero-based region mapping. An absent region has no current counterpart; never copy its old coordinates. Confirmation applies only to this input checkpoint, not future topology.");
        }
        fine.prompt.push_str(&format!(
            "\n\nArchived numbered frame feedback (attempt {}; archive {} / {}): call beaver_object_attempt feedbackImage to inspect the original PNG, numbered regions, optional historical static mesh receipts and camera provenance. Box receipts include occluded meshes and may be truncated. Re-localize against current content before editing; image coordinates alone do not assert 3D hits.",
            previous.id, reference.run_id, reference.frame_id
        ));
    }
    if let Some(image) = &approval.image {
        image.require_source(previous)?;
        fine.prompt.push_str(&format!(
            "\n\nFrozen PNG feedback (output of attempt {}; source file is in this task input; normalized top-left coordinates, regions numbered in array order):\n{}",
            previous.id, serde_json::to_string(image)?
        ));
    }
    fine.prompt.push_str(&format!(
        "\n\nOwner rework feedback (candidate review {}, attempt {}):\n{}",
        approval.review_request_id, approval.attempt_id, approval.feedback
    ));
    ensure!(
        fine.prompt.len() <= 20_000,
        "OBJECT_CANDIDATE_REWORK_PROMPT_TOO_LONG"
    );
    Ok(fine)
}

pub(super) fn select(
    connection: &Connection,
    records: &Records,
    request: &Request,
) -> Result<TaskRecord> {
    let approval = request
        .rework
        .as_ref()
        .context("INVALID_OBJECT_CANDIDATE_REWORK")?;
    let fine = definition(connection, records, approval)?;
    super::super::candidate::require_rework_source(
        connection,
        &request.project_id,
        &approval.review_request_id,
        records,
        true,
    )?;
    Ok(fine)
}

pub(super) fn validate_receipt(
    connection: &Connection,
    saved: &super::storage::Stored,
) -> Result<()> {
    let request = &saved.operation.request;
    let approval = request
        .rework
        .as_ref()
        .context("INVALID_OBJECT_CANDIDATE_REWORK")?;
    let records = &saved.verification.records;
    ensure!(
        request.advance.is_none()
            && saved.fresh_checks.is_none()
            && saved.next.as_ref() == Some(&definition(connection, records, approval)?),
        "OBJECT_CANDIDATE_REWORK_RECEIPT_MISMATCH"
    );
    super::super::candidate::require_rework_source(
        connection,
        &request.project_id,
        &approval.review_request_id,
        records,
        false,
    )
}

pub(crate) fn feedback(
    connection: &Connection,
    project: &str,
    task: &str,
) -> Result<Vec<super::super::candidate::publication::Feedback>> {
    let mut statement = connection.prepare(
        "SELECT json_extract(value,'$.operation.request.requestId') FROM entities
         WHERE kind='object_recovery_resume'
         AND json_extract(value,'$.operation.request.projectId')=?
         AND json_extract(value,'$.operation.request.target.taskId')=? ORDER BY id",
    )?;
    let ids = statement
        .query_map(rusqlite::params![project, task], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut feedback = Vec::new();
    for id in ids {
        let saved = super::storage::read(connection, project, &id)?
            .context("OBJECT_RECOVERY_RESUME_RECEIPT_MISSING")?;
        if let Some(approval) = saved
            .operation
            .request
            .rework
            .filter(|_| matches!(saved.operation.result, Some(super::Outcome::Started { .. })))
        {
            feedback.push(super::super::candidate::publication::Feedback {
                request_id: id,
                attempt_id: approval.attempt_id,
                feedback: approval.feedback,
                image: approval.image,
                preview_frame: approval.preview_frame,
                relocation: approval.relocation,
                later: None,
            });
        }
    }
    feedback.sort_by(|a, b| a.request_id.cmp(&b.request_id));
    Ok(feedback)
}
