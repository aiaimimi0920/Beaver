//! Preserve feedback authority across published follow-ups; never rewrite historical coordinates.
use super::{deferred, followup, storage, Feedback, ReviewTarget, State};
use crate::object_task_storage as tasks;
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    pub publication_request_id: String,
    pub version_id: String,
    pub run_id: String,
    pub published_frame: bool,
}

pub(super) fn inherited(db: &Connection, target: &ReviewTarget) -> Result<Vec<Feedback>> {
    let mut result = vec![];
    for receipt in tasks::read_all::<followup::Receipt>(db, followup::KIND)? {
        if receipt.request.project_id != target.project_id
            || receipt.medium_task_id != target.target.task_id
        {
            continue;
        }
        ensure!(
            receipt.run_id == target.target.run_id
                && receipt.source.object_id == target.target.object_id,
            "OBJECT_FEEDBACK_IMAGE_SCOPE_MISMATCH"
        );
        let saved = storage::read(
            db,
            &target.project_id,
            &receipt.request.publication_request_id,
        )?
        .context("OBJECT_PUBLICATION_NOT_FOUND")?;
        let op = saved.operation;
        ensure!(
            op.state == State::Published
                && op.version_id == receipt.request.version_id
                && op.request.target == receipt.source,
            "OBJECT_FEEDBACK_IMAGE_PUBLICATION_MISMATCH"
        );
        let mut item = Feedback {
            request_id: format!(
                "followup-{}",
                crate::framework_checks::digest(&receipt.request.request_id)?
            ),
            attempt_id: receipt.source.attempt_id,
            feedback: receipt.request.feedback,
            image: None,
            preview_frame: receipt.request.preview_frame,
            relocation: None,
            later: None,
            origin: Some(Origin {
                publication_request_id: receipt.request.publication_request_id,
                version_id: receipt.request.version_id,
                run_id: receipt.source.run_id,
                published_frame: false,
            }),
            relocation_requirement: None,
        };
        if item.preview_frame.is_some() {
            item.origin.as_mut().unwrap().published_frame = true;
        } else {
            // Automatic deferred work inherits the exact original attachment, not a new coordinate set.
            for original in op.preview.feedback {
                if original.later.is_some()
                    && receipt.request.request_id
                        == format!(
                            "deferred-{}",
                            crate::framework_checks::digest(&original.request_id)?
                        )
                {
                    item.attempt_id = original.attempt_id;
                    item.image = original.image;
                    item.preview_frame = original.preview_frame;
                    item.relocation = original.relocation;
                    if let Some(origin) = original.origin {
                        item.origin = Some(origin);
                    }
                    break;
                }
            }
        }
        result.push(item);
    }
    Ok(result)
}

pub(super) fn frame(
    db: &Connection,
    target: &ReviewTarget,
    item: &Feedback,
) -> Result<Option<Value>> {
    ensure!(
        item.relocation.is_none() || item.preview_frame.is_some(),
        "PREVIEW_RELOCATION_TARGET_REQUIRED"
    );
    let Some(reference) = &item.preview_frame else {
        return Ok(None);
    };
    ensure!(item.image.is_none(), "PREVIEW_FEEDBACK_IMAGE_CONFLICT");
    if let Some(origin) = item.origin.as_ref().filter(|o| o.published_frame) {
        ensure!(item.relocation.is_none(), "PREVIEW_FEEDBACK_IMAGE_CONFLICT");
        return Ok(Some(followup::frames::resolve(
            db,
            &target.project_id,
            &target.target.object_id,
            &origin.version_id,
            reference,
        )?));
    }
    let attempt = crate::object_attempt_view::read(db, &target.project_id, &item.attempt_id)?;
    let run = item
        .origin
        .as_ref()
        .map(|o| &o.run_id)
        .unwrap_or(&target.target.run_id);
    ensure!(
        attempt.preparation.run.id == *run
            && attempt.preparation.run.object_id == target.target.object_id,
        "PREVIEW_FEEDBACK_SOURCE_MISMATCH"
    );
    if let Some(confirmation) = &item.relocation {
        super::super::super::resume::rework::relocation::resolve(
            db,
            &attempt,
            reference,
            confirmation,
        )?;
    }
    Ok(Some(deferred::frames::resolve(db, &attempt, reference)?))
}

pub(super) fn prepare(
    runtime: &crate::project_runtime::ProjectRuntime,
    db: &Connection,
    target: &ReviewTarget,
    item: &mut Feedback,
) -> Result<()> {
    let evidence = frame(db, target, item)?;
    let (count, identity) = if let Some(frame) = evidence {
        let count = frame["selection"]["regions"]
            .as_array()
            .context("PREVIEW_FEEDBACK_SELECTION_REQUIRED")?
            .len();
        (count, frame["id"].clone())
    } else if let Some(image) = &item.image {
        let attempt = crate::object_attempt_view::read(db, &target.project_id, &item.attempt_id)?;
        let run = item
            .origin
            .as_ref()
            .map(|o| &o.run_id)
            .unwrap_or(&target.target.run_id);
        ensure!(
            !item.origin.as_ref().is_some_and(|o| o.published_frame)
                && attempt.preparation.run.id == *run
                && attempt.preparation.run.object_id == target.target.object_id,
            "PREVIEW_FEEDBACK_SOURCE_MISMATCH"
        );
        image.verify_resolved(runtime, &attempt)?;
        (image.regions.len(), Value::String(image.sha256.clone()))
    } else {
        return Ok(());
    };
    item.relocation_requirement = None;
    item.relocation_requirement = Some(super::relocation::Requirement {
        source_digest: crate::framework_checks::digest(&(&*item, &identity))?,
        region_count: count,
    });
    Ok(())
}
