//! Durable verification evidence. Neither a report nor its replay grants execution rights.
use crate::{object_run_preparation::PreparationState, project_runtime::ProjectRuntime};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[path = "object_candidate_review.rs"]
pub mod candidate;
#[path = "object_recovery_derivation.rs"]
pub(crate) mod derivation;
#[path = "object_recovery_disposition.rs"]
pub mod disposition;
#[path = "object_recovery_files.rs"]
mod files;
#[path = "object_recovery_records.rs"]
mod records;
#[path = "object_recovery_resume.rs"]
pub mod resume;
#[path = "object_recovery_store.rs"]
pub(crate) mod storage;

pub(crate) const MAX_GENERATION: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    pub task_id: String,
    pub object_id: String,
    pub run_id: String,
    pub owner: String,
    pub claim_token: String,
    pub writer_generation: u64,
    pub task_revision: u64,
    pub run_revision: u64,
    pub object_revision: u64,
    pub control_revision: u64,
    pub recovery_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyRequest {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriterStatus {
    Active,
    StopRecorded,
    Unconfirmed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentStatus {
    Verified,
    Invalid,
    Unchecked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceStatus {
    MatchesCheckpoint,
    Drifted,
    Missing,
    Invalid,
    Unchecked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub records_current: bool,
    pub writer_status: WriterStatus,
    pub content_status: ContentStatus,
    pub workspace_status: WorkspaceStatus,
    pub paused: bool,
    pub issues: Vec<String>,
}

impl Report {
    /// A disposition may only be offered after durable evidence proves that
    /// the retained workspace is safe to handle manually. This is an
    /// eligibility signal, not a disposal or execution capability.
    pub fn can_dispose(&self) -> bool {
        self.records_current
            && matches!(self.writer_status, WriterStatus::StopRecorded)
            && matches!(self.content_status, ContentStatus::Verified)
            && matches!(self.workspace_status, WorkspaceStatus::MatchesCheckpoint)
            && !self.paused
            && self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub schema_version: u32,
    pub request: VerifyRequest,
    pub generation: u64,
    pub result: Option<Report>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub target: Target,
    pub preparation_state: PreparationState,
    pub paused: bool,
    pub operation: Option<Operation>,
    pub disposition: Option<disposition::Operation>,
    pub resume: Option<resume::Operation>,
    pub can_resume: bool,
    /// Compares durable records only; querying does not recheck workspace bytes.
    pub report_matches_records: bool,
    pub can_dispose: bool,
}

/// Querying never claims a verifier, reads workspace bytes, or reconstructs a worker.
pub fn get(runtime: &ProjectRuntime, project: &str, task: &str) -> Result<Option<View>> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    ensure!(
        crate::object_task_types::valid_id(task),
        "INVALID_OBJECT_TASK_ID"
    );
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object recovery store lock poisoned"))?;
    let transaction = store.connection.unchecked_transaction()?;
    let view = storage::view(&transaction, project, task)?;
    transaction.commit()?;
    Ok(view)
}

pub(crate) fn verify(
    runtime: &ProjectRuntime,
    request: &VerifyRequest,
    live: bool,
) -> Result<Operation> {
    verify_with(runtime, request, live, || Ok(()))
}

pub(crate) fn verify_with(
    runtime: &ProjectRuntime,
    request: &VerifyRequest,
    live: bool,
    after_files: impl FnOnce() -> Result<()>,
) -> Result<Operation> {
    storage::validate(runtime, request)?;
    // Clones share this gate and the project filesystem lease. A reopened host
    // may retry a pending operation, but an overlapping verifier cannot scan.
    let _guard = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let saved = transact(runtime, |connection| storage::begin(connection, request))?;
    if saved.operation.result.is_some() {
        return Ok(saved.operation);
    }
    let report = files::verify(runtime, &saved.records, live);
    after_files()?;
    transact(runtime, |connection| {
        storage::finish(connection, saved, report)
    })
}

fn transact<T>(
    runtime: &ProjectRuntime,
    work: impl FnOnce(&rusqlite::Connection) -> Result<T>,
) -> Result<T> {
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object recovery store lock poisoned"))?;
    store.transaction(work)
}
