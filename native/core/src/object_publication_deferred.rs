//! Frozen pre-publication feedback that becomes planned work only on publication commit.
use super::{prepare, storage, transact, Feedback, ReviewTarget};
use crate::{
    object_task_storage as tasks, object_task_types::valid_id, project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const KIND: &str = "object_publication_deferred";
#[path = "object_deferred_frames.rs"]
pub mod frames;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Later {
    pub title: String,
    pub acceptance: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub review: ReviewTarget,
    pub feedback: String,
    pub later: Later,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<super::super::super::resume::rework::image::Feedback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_frame: Option<super::followup::frames::Reference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relocation: Option<super::super::super::resume::rework::relocation::Confirmation>,
}

pub fn create(runtime: &ProjectRuntime, request: &Request) -> Result<Request> {
    if let Some(relocation) = &request.relocation {
        ensure!(
            request.preview_frame.is_some(),
            "FEEDBACK_RELOCATION_TARGET_REQUIRED"
        );
        relocation.validate()?;
    }
    ensure!(
        request.image.is_none() || request.preview_frame.is_none(),
        "FEEDBACK_IMAGE_CONFLICT"
    );
    prepare::validate_target(runtime, &request.review)?;
    ensure!(
        request.project_id == request.review.project_id && valid_id(&request.request_id),
        "INVALID_OBJECT_PUBLICATION_REQUEST"
    );
    for (value, limit) in [
        (&request.feedback, 4000),
        (&request.later.title, 300),
        (&request.later.acceptance, 10000),
    ] {
        ensure!(
            !value.trim().is_empty() && value.len() <= limit,
            "INVALID_PUBLICATION_FOLLOWUP_TEXT"
        );
    }
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_PUBLICATION_BUSY"))?;
    let key = crate::framework_checks::digest(&(&request.project_id, &request.request_id))?;
    if let Some(saved) = transact(runtime, |db| tasks::read::<Request>(db, KIND, &key))? {
        ensure!(saved == *request, "OBJECT_TASK_REQUEST_ID_CONFLICT");
        return Ok(saved);
    }
    let source = transact(runtime, |db| {
        Ok(prepare::load(runtime, db, &request.review)?.0)
    })?;
    if let Some(image) = &request.image {
        image.verify(
            runtime,
            source
                .records
                .attempt
                .as_ref()
                .context("OBJECT_ATTEMPT_NOT_FOUND")?,
        )?;
    }
    transact(runtime, |db| {
        if let Some(saved) = tasks::read::<Request>(db, KIND, &key)? {
            ensure!(saved == *request, "OBJECT_TASK_REQUEST_ID_CONFLICT");
            return Ok(saved);
        }
        storage::require_task_idle(db, &request.project_id, &request.review.target.task_id)?;
        let (current, _, _) = prepare::load(runtime, db, &request.review)?;
        ensure!(current == source, "OBJECT_CANDIDATE_RECORD_CHANGED");
        if let Some(reference) = &request.preview_frame {
            if let Some(relocation) = &request.relocation {
                super::super::super::resume::rework::relocation::resolve(
                    db,
                    current
                        .records
                        .attempt
                        .as_ref()
                        .context("OBJECT_ATTEMPT_NOT_FOUND")?,
                    reference,
                    relocation,
                )?;
            }
            frames::resolve(
                db,
                current
                    .records
                    .attempt
                    .as_ref()
                    .context("OBJECT_ATTEMPT_NOT_FOUND")?,
                reference,
            )?;
        }
        tasks::insert(db, KIND, &key, request)?;
        Ok(request.clone())
    })
}

pub(super) fn feedback(db: &Connection, project: &str, task: &str) -> Result<Vec<Feedback>> {
    tasks::read_all::<Request>(db, KIND)?
        .into_iter()
        .filter(|r| r.project_id == project && r.review.target.task_id == task)
        .map(|r| {
            Ok(Feedback {
                request_id: format!(
                    "deferred-{}",
                    crate::framework_checks::digest(&r.request_id)?
                ),
                attempt_id: r.review.target.attempt_id,
                feedback: r.feedback,
                image: r.image,
                preview_frame: r.preview_frame,
                relocation: r.relocation,
                later: Some(r.later),
                origin: None,
                relocation_requirement: None,
            })
        })
        .collect()
}

pub(super) fn commit(db: &Connection, operation: &super::Operation) -> Result<()> {
    for feedback in &operation.preview.feedback {
        let Some(later) = &feedback.later else {
            continue;
        };
        let mut text = format!(
            "{}\nOriginal feedback attempt: {}.",
            feedback.feedback, feedback.attempt_id
        );
        if let Some(image) = &feedback.image {
            text.push_str(&format!("\nHistorical PNG: {} (SHA-256 {}, {}x{}). Re-open the source attempt and re-localize against the pinned version before editing. Never apply historic coordinates without revalidation.", image.path, image.sha256, image.width, image.height));
            for (index, region) in image.regions.iter().enumerate() {
                text.push_str(&format!("\nRegion {} (approximate normalized x={:.6}, y={:.6}, width={:.6}, height={:.6}; exact annotation retained in publication): {}", index + 1, region.x, region.y, region.width, region.height, region.prompt));
            }
        }
        if let Some(reference) = &feedback.preview_frame {
            text.push_str(&format!("\nHistorical numbered Godot frame {} from preview run {}. Use feedbackImage to inspect the original PNG, camera and numbered region prompts. Re-localize against the pinned version before editing; never apply historic coordinates without revalidation.", reference.frame_id, reference.run_id));
        }
        if feedback.relocation.is_some() {
            text.push_str("\nfeedbackImage also retains the owner's region correspondence from an earlier attempt to the feedback checkpoint. It is historical evidence, not authorization to reuse coordinates on a later published checkpoint. Inspect both labeled frames and re-localize if the input has changed.");
        }
        super::followup::insert(
            db,
            &super::followup::Request {
                project_id: operation.request.project_id.clone(),
                request_id: format!(
                    "deferred-{}",
                    crate::framework_checks::digest(&feedback.request_id)?
                ),
                publication_request_id: operation.request.request_id.clone(),
                version_id: operation.version_id.clone(),
                title: later.title.clone(),
                feedback: text,
                acceptance: later.acceptance.clone(),
                preview_frame: None,
            },
            operation.request.target.clone(),
        )?;
    }
    Ok(())
}
