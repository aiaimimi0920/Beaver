//! Reserve a medium writer, freeze its inputs, then prepare files outside SQLite.
use crate::{
    object_task_types::{RunRecord, TaskRecord},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[path = "object_run_prepare_store.rs"]
mod storage;
#[path = "object_run_workspace.rs"]
mod workspace;
pub use crate::object_run_baseline::FrozenBaseline;
pub(crate) use storage::require_unprepared;
pub(crate) use storage::{read as read_record, require_ready};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreparationState {
    Pending,
    Preparing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preparation {
    pub schema_version: u32,
    pub id: String,
    pub project_id: String,
    pub owner: String,
    pub claim_token: String,
    pub generation: u64,
    // Frozen claim-time projections, not the current task/run status.
    pub medium: TaskRecord,
    pub run: RunRecord,
    pub workspace: String,
    pub baseline: Option<FrozenBaseline>,
    pub state: PreparationState,
    pub error: Option<String>,
}

/// Only a newly committed claim can initiate file preparation. Reading a log
/// after restart never recreates this capability or replays file operations.
pub struct PreparationClaim {
    record: Preparation,
}

impl PreparationClaim {
    pub fn record(&self) -> &Preparation {
        &self.record
    }
}

pub fn claim_next(
    runtime: &ProjectRuntime,
    project_id: &str,
    owner: &str,
) -> Result<Option<PreparationClaim>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(!owner.trim().is_empty(), "OBJECT_TASK_QUEUE_OWNER_REQUIRED");
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object run store lock poisoned"))?;
    store.transaction(|connection| {
        storage::begin(connection, project_id, owner)
            .map(|record| record.map(|record| PreparationClaim { record }))
    })
}

pub fn get(
    runtime: &ProjectRuntime,
    project_id: &str,
    run_id: &str,
) -> Result<Option<Preparation>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object run store lock poisoned"))?;
    storage::read(&store.connection, project_id, run_id)
}

pub fn prepare(runtime: &ProjectRuntime, claim: PreparationClaim) -> Result<Preparation> {
    prepare_with(runtime, claim, || Ok(()))
}

pub(crate) fn prepare_with(
    runtime: &ProjectRuntime,
    claim: PreparationClaim,
    after_files: impl FnOnce() -> Result<()>,
) -> Result<Preparation> {
    let mut record = claim.record;
    ensure!(
        record.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    if record.state == PreparationState::Failed {
        return Ok(record);
    }
    {
        let handle = runtime.store();
        let mut store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("object run store lock poisoned"))?;
        store.transaction(|connection| storage::start(connection, &mut record))?;
    }
    let result = workspace::create(runtime, &record);
    after_files()?;
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object run store lock poisoned"))?;
    store.transaction(|connection| {
        storage::finish(
            connection,
            record,
            result.err().map(|error| format!("{error:#}")),
        )
    })
}
