//! Explicit owner acceptance with a durable, retryable project materialization journal.
use super::{Report, Source};
use crate::{object_attempt_view::Target, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

#[path = "object_publication_commit.rs"]
mod commit;
#[path = "object_publication_deferred.rs"]
pub mod deferred;
#[path = "object_publication_files.rs"]
mod files;
#[path = "object_publication_followup.rs"]
pub mod followup;
#[path = "object_publication_history.rs"]
mod history;
#[path = "object_publication_integrity.rs"]
mod integrity;
#[path = "object_publication_prepare.rs"]
mod prepare;
#[path = "object_publication_store.rs"]
mod storage;
use super::super::transact;
pub(crate) use history::published_task;
pub(crate) use history::retained_attempt;
pub(crate) use storage::{blocked, require_task_idle, task_pending};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewTarget {
    pub project_id: String,
    pub target: Target,
    pub review_request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feedback {
    pub request_id: String,
    pub attempt_id: String,
    pub feedback: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<super::super::resume::rework::image::Feedback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_frame: Option<followup::frames::Reference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relocation: Option<super::super::resume::rework::relocation::Confirmation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later: Option<deferred::Later>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Resolution {
    Resolved,
    Waived,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedbackDecision {
    pub request_id: String,
    pub resolution: Resolution,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preview {
    pub review: ReviewTarget,
    pub digest: String,
    pub output_digest: String,
    pub baseline_version_id: Option<String>,
    pub accepted_version_id: Option<String>,
    pub object_revision: u64,
    pub replacement_required: bool,
    pub files: Vec<crate::object_catalog::ObjectFile>,
    // Includes unchanged owned files: their bytes must still match the accepted snapshot.
    pub paths: Vec<crate::files::Change>,
    pub feedback: Vec<Feedback>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
    pub review_request_id: String,
    pub preview_digest: String,
    pub acceptance_note: String,
    pub confirm_files: bool,
    pub confirm_replacement: bool,
    pub feedback: Vec<FeedbackDecision>,
}

impl Request {
    fn review(&self) -> ReviewTarget {
        ReviewTarget {
            project_id: self.project_id.clone(),
            target: self.target.clone(),
            review_request_id: self.review_request_id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Applying,
    Aborting,
    Published,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Published {
    pub version_id: String,
    pub object_revision: u64,
    pub task_revision: u64,
    pub run_revision: u64,
    pub plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub schema_version: u32,
    pub request: Request,
    pub preview: Preview,
    pub version_id: String,
    pub state: State,
    pub error: Option<String>,
    pub result: Option<Published>,
}

impl Operation {
    fn pending(&self) -> bool {
        matches!(self.state, State::Applying | State::Aborting)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    operation: Operation,
    source: Source,
    review: Report,
    manifest: crate::object_version_manifest::VersionManifest,
    // Persist intent before writing, so abort never owns an untouched matching external file.
    writes: Vec<String>,
}

pub fn preview(runtime: &ProjectRuntime, request: &ReviewTarget) -> Result<Preview> {
    prepare::validate_target(runtime, request)?;
    transact(runtime, |db| {
        storage::require_task_idle(db, &request.project_id, &request.target.task_id)?;
        Ok(prepare::load(db, request)?.2)
    })
}

pub fn list(runtime: &ProjectRuntime, project: &str, task: &str) -> Result<Vec<Operation>> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    transact(runtime, |db| history::list(db, project, task))
}

pub fn publish(runtime: &ProjectRuntime, request: &Request) -> Result<Operation> {
    execute_with(runtime, request, |_| Ok(()))
}

pub(crate) fn execute_with(
    runtime: &ProjectRuntime,
    request: &Request,
    checkpoint: impl Fn(&str) -> Result<()>,
) -> Result<Operation> {
    prepare::validate_target(runtime, &request.review())?;
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_PUBLICATION_BUSY"))?;
    let saved = transact(runtime, |db| storage::replay(db, request))?;
    let mut saved = match saved {
        Some(saved) => saved,
        None => {
            let (source, review, preview) =
                transact(runtime, |db| prepare::load(db, &request.review()))?;
            prepare::approve(request, &preview)?;
            let manifest = files::freeze(runtime, &source, &preview)?;
            let saved = Stored {
                operation: Operation {
                    schema_version: 1,
                    request: request.clone(),
                    preview,
                    version_id: manifest.version_id.clone(),
                    state: State::Applying,
                    error: None,
                    result: None,
                },
                source,
                review,
                manifest,
                writes: vec![],
            };
            transact(runtime, |db| storage::begin(runtime, db, &saved))?;
            saved
        }
    };
    if !saved.operation.pending() {
        return Ok(saved.operation);
    }
    ensure!(
        saved.operation.state == State::Applying,
        "OBJECT_PUBLICATION_ABORT_PENDING"
    );
    let result = (|| {
        checkpoint("prepared")?;
        transact(runtime, |db| storage::validate_current(db, &saved))?;
        files::apply(runtime, &mut saved, &checkpoint)?;
        checkpoint("beforeCommit")?;
        transact(runtime, |db| {
            storage::validate_current(db, &saved)?;
            files::verify_applied(runtime, &saved)?;
            commit::commit(runtime, db, saved.clone())
        })
    })();
    match result {
        Ok(operation) => Ok(operation),
        Err(error) => transact(runtime, |db| {
            storage::record_error(db, saved, format!("{error:#}"))
        }),
    }
}

/// Abort only the file application. Frozen work and object ownership remain available.
pub fn abort(runtime: &ProjectRuntime, project: &str, request_id: &str) -> Result<Operation> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_PUBLICATION_BUSY"))?;
    let mut saved = transact(runtime, |db| {
        let mut saved =
            storage::read(db, project, request_id)?.context("OBJECT_PUBLICATION_NOT_FOUND")?;
        ensure!(
            saved.operation.state != State::Published,
            "OBJECT_PUBLICATION_ALREADY_PUBLISHED"
        );
        if saved.operation.state != State::Aborted {
            saved.operation.state = State::Aborting;
            storage::save(db, &saved)?;
        }
        Ok(saved)
    })?;
    if saved.operation.state == State::Aborted {
        return Ok(saved.operation);
    }
    match files::reverse(runtime, &saved) {
        Ok(conflicts) => {
            saved.operation.state = State::Aborted;
            saved.operation.error = (!conflicts.is_empty()).then(|| {
                format!(
                    "OBJECT_PUBLICATION_EXTERNAL_CHANGES_RETAINED: {}",
                    conflicts.join(", ")
                )
            });
            transact(runtime, |db| {
                storage::save(db, &saved)?;
                Ok(saved.operation)
            })
        }
        Err(error) => transact(runtime, |db| {
            storage::record_error(db, saved, format!("{error:#}"))
        }),
    }
}
