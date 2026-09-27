//! Read-only projection of frozen object evidence into the validation protocol.
use crate::{
    object_attempt_checks::{self as checks, Report},
    object_attempt_view::{self as views, Target},
    object_run_recovery::candidate,
    object_task_types::valid_id,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub attempt_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    ObjectAttempt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateReview {
    pub request_id: String,
    pub source_digest: String,
    pub output_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub kind: SourceKind,
    pub project_id: String,
    pub target: Target,
    pub stage_id: String,
    pub candidate_reviews: Vec<CandidateReview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub source: Source,
    pub report: Report,
}

pub fn get(runtime: &ProjectRuntime, request: &Request) -> Result<Evidence> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.request_id) && valid_id(&request.attempt_id),
        "INVALID_OBJECT_CHECK_REQUEST"
    );
    let report = checks::list(runtime, &request.project_id, &request.attempt_id)?
        .into_iter()
        .find(|report| report.request.request_id == request.request_id)
        .context("OBJECT_CHECK_REPORT_NOT_FOUND")?;
    let stage_id = {
        let handle = runtime.store();
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("validation object store lock poisoned"))?;
        let attempt = views::read(&store.connection, &request.project_id, &request.attempt_id)?;
        ensure!(
            Target::from_record(&attempt) == report.request.target,
            "OBJECT_ATTEMPT_STALE_TARGET"
        );
        attempt
            .fine
            .stage_id
            .context("OBJECT_ATTEMPT_STAGE_REQUIRED")?
    };
    let mut candidate_reviews = Vec::new();
    for review in candidate::list(runtime, &request.project_id, &request.attempt_id)? {
        if review.request.check_request_id != request.request_id {
            continue;
        }
        ensure!(
            review.request.target == report.request.target
                && review.output_digest == report.output_digest,
            "OBJECT_CANDIDATE_REPORT_MISMATCH"
        );
        candidate_reviews.push(CandidateReview {
            request_id: review.request.request_id,
            source_digest: review.source_digest,
            output_digest: review.output_digest,
        });
    }
    Ok(Evidence {
        source: Source {
            kind: SourceKind::ObjectAttempt,
            project_id: request.project_id.clone(),
            target: report.request.target.clone(),
            stage_id,
            candidate_reviews,
        },
        report,
    })
}
