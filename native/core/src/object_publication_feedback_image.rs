//! Resolve historical image authority from immutable publication and follow-up receipts.
use super::{storage, tasks, transact, Receipt, State, KIND};
use crate::{object_attempt::Attempt, object_attempt_file, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Binding {
    publication_request_id: String,
    version_id: String,
    feedback_request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<object_attempt_file::Request>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<crate::object_run_recovery::resume::rework::image::Feedback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_frame: Option<serde_json::Value>,
    #[serde(skip)]
    pub data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relocation: Option<crate::object_run_recovery::resume::rework::relocation::Resolved>,
}

pub(crate) fn resolve(runtime: &ProjectRuntime, attempt: &Attempt) -> Result<Option<Binding>> {
    let project = &attempt.preparation.project_id;
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    transact(runtime, |db| {
        let receipt = tasks::read_all::<Receipt>(db, KIND)?
            .into_iter()
            .find(|r| r.request.project_id == *project && r.fine_task_id == attempt.fine.id);
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        ensure!(
            receipt.run_id == attempt.preparation.run.id
                && receipt.medium_task_id == attempt.preparation.medium.id
                && receipt.source.object_id == attempt.preparation.run.object_id,
            "OBJECT_FEEDBACK_IMAGE_SCOPE_MISMATCH"
        );
        let saved = storage::read(db, project, &receipt.request.publication_request_id)?
            .context("OBJECT_PUBLICATION_NOT_FOUND")?;
        let operation = saved.operation;
        ensure!(
            operation.state == State::Published
                && operation.version_id == receipt.request.version_id
                && operation.request.target == receipt.source,
            "OBJECT_FEEDBACK_IMAGE_PUBLICATION_MISMATCH"
        );
        if let Some(reference) = &receipt.request.preview_frame {
            let mut frame = super::frames::resolve(
                db,
                project,
                &receipt.source.object_id,
                &receipt.request.version_id,
                reference,
            )?;
            let data_url = frame["frame"]
                .as_object_mut()
                .unwrap()
                .remove("dataUrl")
                .and_then(|v| v.as_str().map(str::to_owned))
                .context("PREVIEW_FRAME_PNG_REQUIRED")?;
            return Ok(Some(Binding {
                publication_request_id: receipt.request.publication_request_id,
                version_id: receipt.request.version_id,
                feedback_request_id: receipt.request.request_id,
                file: None,
                image: None,
                preview_frame: Some(frame),
                data_url: Some(data_url),
                relocation: None,
            }));
        }
        for feedback in operation.preview.feedback {
            // Only the automatically created deferred task owns this attachment.
            // Ordinary text follow-ups cannot select historical files by ID/path.
            if feedback.later.is_none()
                || receipt.request.request_id
                    != format!(
                        "deferred-{}",
                        crate::framework_checks::digest(&feedback.request_id)?
                    )
            {
                continue;
            }
            if feedback.preview_frame.is_none() && feedback.image.is_none() {
                return Ok(None);
            }
            let source = crate::object_attempt_view::read(db, project, &feedback.attempt_id)?;
            ensure!(
                source.preparation.run.id == receipt.source.run_id
                    && source.preparation.run.object_id == receipt.source.object_id,
                "OBJECT_FEEDBACK_IMAGE_SOURCE_MISMATCH"
            );
            if let Some(reference) = &feedback.preview_frame {
                let relocation = feedback
                    .relocation
                    .as_ref()
                    .map(|confirmation| {
                        crate::object_run_recovery::resume::rework::relocation::resolve(
                            db,
                            &source,
                            reference,
                            confirmation,
                        )
                    })
                    .transpose()?;
                let mut frame = super::super::deferred::frames::resolve(db, &source, reference)?;
                let data_url = frame["frame"]
                    .as_object_mut()
                    .unwrap()
                    .remove("dataUrl")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .context("PREVIEW_FRAME_PNG_REQUIRED")?;
                return Ok(Some(Binding {
                    publication_request_id: receipt.request.publication_request_id,
                    version_id: receipt.request.version_id,
                    feedback_request_id: feedback.request_id,
                    file: None,
                    image: None,
                    preview_frame: Some(frame),
                    data_url: Some(data_url),
                    relocation,
                }));
            }
            let Some(image) = feedback.image else {
                return Ok(None);
            };
            return Ok(Some(Binding {
                publication_request_id: receipt.request.publication_request_id,
                version_id: receipt.request.version_id,
                feedback_request_id: feedback.request_id,
                file: Some(object_attempt_file::Request {
                    project_id: project.clone(),
                    run_id: receipt.source.run_id,
                    attempt_id: feedback.attempt_id,
                    checkpoint: object_attempt_file::Checkpoint::Output,
                    path: image.path.clone(),
                    sha256: image.sha256.clone(),
                }),
                image: Some(image),
                preview_frame: None,
                data_url: None,
                relocation: None,
            }));
        }
        Ok(None)
    })
}
