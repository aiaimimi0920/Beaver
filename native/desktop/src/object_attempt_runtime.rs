//! Async execution controls use the selected project's runtime and typed scheduler API.
use anyhow::{anyhow, Result};
use beaver_core::{project_storage_router::ProjectStorageRouter, scheduler::Scheduler};
use serde_json::Value;

pub(crate) async fn call(
    scheduler: &Scheduler,
    router: &ProjectStorageRouter,
    method: &str,
    input: Value,
) -> Result<Value> {
    crate::business_catalog::validate(method, &input)
        .map_err(|(code, message)| anyhow!("{code}: {message}"))?;
    let project_id = input["projectId"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing projectId"))?;
    let runtime = router.runtime_for_project(project_id)?;
    match method {
        "objectTask.attemptFrames" => {
            let project = project_id.to_owned();
            let attempt = input["attemptId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing attemptId"))?
                .to_owned();
            let reference = input
                .get("previewFrame")
                .cloned()
                .map(serde_json::from_value)
                .transpose()?;
            tokio::task::spawn_blocking(move || {
                Ok(serde_json::to_value(beaver_core::object_run_recovery::candidate::publication::deferred::frames::list(
                    &runtime, &project, &attempt, reference.as_ref())?)?)
            }).await?
        }
        "objectTask.deferCandidateFeedback" => {
            let request = serde_json::from_value(input)?;
            let receipt = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::publication::deferred::create(
                    &runtime, &request,
                )
            })
            .await??;
            Ok(serde_json::to_value(receipt)?)
        }
        "objectTask.createPublicationFollowup" => {
            let request = serde_json::from_value(input)?;
            let receipt = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::publication::followup::create(
                    &runtime, &request,
                )
            })
            .await??;
            Ok(serde_json::to_value(receipt)?)
        }
        "objectTask.publicationFrames" => {
            let project = project_id.to_owned();
            let publication = input["publicationRequestId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing publicationRequestId"))?
                .to_owned();
            tokio::task::spawn_blocking(move || {
                Ok(serde_json::to_value(beaver_core::object_run_recovery::candidate::publication::followup::frames::list(
                    &runtime, &project, &publication)?)?)
            }).await?
        }
        "objectTask.publicationFollowups" => {
            let project = project_id.to_owned();
            let publication = input["publicationRequestId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing publicationRequestId"))?
                .to_owned();
            let receipts = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::publication::followup::list(
                    &runtime,
                    &project,
                    &publication,
                )
            })
            .await??;
            Ok(serde_json::to_value(receipts)?)
        }
        "objectTask.publicationPreview" => {
            let request = serde_json::from_value(input)?;
            let preview = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::publication::preview(
                    &runtime, &request,
                )
            })
            .await??;
            Ok(serde_json::to_value(preview)?)
        }
        "objectTask.publishCandidate" => {
            let request = serde_json::from_value(input)?;
            let operation = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::publication::publish(
                    &runtime, &request,
                )
            })
            .await??;
            if operation.state
                == beaver_core::object_run_recovery::candidate::publication::State::Published
            {
                // The acceptance receipt is committed even if the scheduler is shutting down.
                let _ = scheduler.wake();
            }
            Ok(serde_json::to_value(operation)?)
        }
        "objectTask.publications" | "objectTask.abortPublication" => {
            let project = project_id.to_owned();
            let abort = method == "objectTask.abortPublication";
            let field = if abort { "requestId" } else { "taskId" };
            let id = input[field]
                .as_str()
                .ok_or_else(|| anyhow!("Missing {field}"))?
                .to_owned();
            tokio::task::spawn_blocking(move || {
                use beaver_core::object_run_recovery::candidate::publication;
                if abort {
                    Ok(serde_json::to_value(publication::abort(
                        &runtime, &project, &id,
                    )?)?)
                } else {
                    Ok(serde_json::to_value(publication::list(
                        &runtime, &project, &id,
                    )?)?)
                }
            })
            .await?
        }
        "objectTask.prepareCandidateReview" => {
            let request = serde_json::from_value(input)?;
            let report = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::prepare(&runtime, &request)
            })
            .await??;
            Ok(serde_json::to_value(report)?)
        }
        "objectTask.candidateReviews" => {
            let project = project_id.to_owned();
            let attempt = input["attemptId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing attemptId"))?
                .to_owned();
            let reports = tokio::task::spawn_blocking(move || {
                beaver_core::object_run_recovery::candidate::list(&runtime, &project, &attempt)
            })
            .await??;
            Ok(serde_json::to_value(reports)?)
        }
        "objectTask.attemptFile" => {
            let request = serde_json::from_value(input)?;
            let response = tokio::task::spawn_blocking(move || {
                beaver_core::object_attempt_file::read(&runtime, &request)
            })
            .await??;
            Ok(serde_json::to_value(response)?)
        }
        "objectTask.attemptTrace" => {
            let request = serde_json::from_value(input)?;
            let response = tokio::task::spawn_blocking(move || {
                beaver_core::object_attempt_trace::read(&runtime, &request)
            })
            .await??;
            Ok(serde_json::to_value(response)?)
        }
        "objectTask.checkAttempt" => {
            let request = serde_json::from_value(input)?;
            let report = tokio::task::spawn_blocking(move || {
                beaver_core::object_attempt_checks::run(&runtime, &request)
            })
            .await??;
            Ok(serde_json::to_value(report)?)
        }
        "objectTask.attemptChecks" => {
            let project = project_id.to_owned();
            let attempt = input["attemptId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing attemptId"))?
                .to_owned();
            let reports = tokio::task::spawn_blocking(move || {
                beaver_core::object_attempt_checks::list(&runtime, &project, &attempt)
            })
            .await??;
            Ok(serde_json::to_value(reports)?)
        }
        "objectTask.recovery" => {
            let task = input["taskId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing taskId"))?;
            Ok(serde_json::to_value(
                beaver_core::object_run_recovery::get(&runtime, project_id, task)?,
            )?)
        }
        "objectTask.verifyRecovery" => {
            let request = serde_json::from_value(input)?;
            Ok(serde_json::to_value(
                scheduler
                    .verify_object_recovery(runtime, request)
                    .await
                    .map_err(anyhow::Error::msg)?,
            )?)
        }
        "objectTask.resumeRecovery"
        | "objectTask.advanceAttempt"
        | "objectTask.reworkCandidate" => {
            let request: beaver_core::object_run_recovery::resume::Request =
                serde_json::from_value(input)?;
            anyhow::ensure!(
                request.advance.is_some() == (method == "objectTask.advanceAttempt")
                    && request.rework.is_some() == (method == "objectTask.reworkCandidate"),
                "OBJECT_STAGE_APPROVAL_METHOD_MISMATCH"
            );
            Ok(serde_json::to_value(
                scheduler
                    .resume_object_recovery(runtime, request)
                    .await
                    .map_err(anyhow::Error::msg)?,
            )?)
        }
        "objectTask.disposeRecovery" => {
            let request = serde_json::from_value(input)?;
            Ok(serde_json::to_value(
                scheduler
                    .dispose_object_recovery(runtime, request)
                    .await
                    .map_err(anyhow::Error::msg)?,
            )?)
        }
        "objectTask.attempts" => {
            let run_id = input["runId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing runId"))?;
            Ok(serde_json::to_value(
                scheduler
                    .object_attempts(runtime, run_id.into())
                    .await
                    .map_err(anyhow::Error::msg)?,
            )?)
        }
        "objectTask.interrupt" => {
            let request = serde_json::from_value(input)?;
            Ok(serde_json::to_value(
                scheduler
                    .interrupt_attempt(runtime, request)
                    .await
                    .map_err(anyhow::Error::msg)?,
            )?)
        }
        _ => Err(anyhow!("Unknown object execution method")),
    }
}

#[cfg(test)]
#[path = "object_attempt_runtime_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "object_stage_advance_runtime_tests.rs"]
mod advance_tests;
#[cfg(test)]
#[path = "object_candidate_runtime_tests.rs"]
mod candidate_tests;
#[cfg(test)]
#[path = "object_publication_runtime_tests.rs"]
mod publication_tests;
#[cfg(test)]
#[path = "object_recovery_runtime_tests.rs"]
mod recovery_tests;
#[cfg(test)]
#[path = "object_recovery_resume_runtime_tests.rs"]
mod resume_tests;
