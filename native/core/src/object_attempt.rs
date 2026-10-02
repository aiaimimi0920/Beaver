//! A single fine execution owns its frozen inputs until its writer has stopped.
use crate::{
    files::Snapshot,
    object_run_preparation::{self, Preparation, PreparationClaim, PreparationState},
    object_task_storage,
    object_task_types::{RunRecord, TaskRecord},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "object_attempt_store.rs"]
mod storage;
pub(crate) use storage::select_fine;

pub(crate) const KIND: &str = "object_attempt";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Running,
    AwaitingGate,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attempt {
    pub schema_version: u32,
    pub id: String,
    pub preparation: Preparation,
    pub fine: TaskRecord,
    pub input: Snapshot,
    pub state: State,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub output: Option<Snapshot>,
    pub error: Option<String>,
}

/// Only fresh preparation or a committed explicit retry/advance creates this lease. Durable records
/// are evidence; deserializing them cannot restart an execution after a crash.
pub(crate) struct Lease {
    record: Attempt,
    medium: TaskRecord,
    run: RunRecord,
    fine: TaskRecord,
}

impl Lease {
    pub(crate) fn record(&self) -> &Attempt {
        &self.record
    }
}

pub(crate) fn successor(
    connection: &rusqlite::Connection,
    previous: &Attempt,
    medium: TaskRecord,
    run: RunRecord,
    fine: TaskRecord,
) -> Result<Lease> {
    let record = Attempt {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        preparation: previous.preparation.clone(),
        fine: fine.clone(),
        input: previous
            .output
            .clone()
            .ok_or_else(|| anyhow::anyhow!("OBJECT_RECOVERY_STOP_UNCONFIRMED"))?,
        state: State::Running,
        thread_id: None,
        turn_id: None,
        output: None,
        error: None,
    };
    let mut lease = Lease {
        record,
        medium,
        run,
        fine,
    };
    storage::transition(connection, &mut lease, "running", "running")?;
    object_task_storage::insert(connection, KIND, &lease.record.id, &lease.record)?;
    Ok(lease)
}

pub fn list(runtime: &ProjectRuntime, run_id: &str) -> Result<Vec<Attempt>> {
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object attempt store lock poisoned"))?;
    let mut records = Vec::new();
    for (id, record) in store.list_with_ids::<Attempt>(KIND)? {
        ensure!(
            id == record.id
                && record.schema_version == 1
                && record.preparation.project_id == runtime.project_id()
                && record.preparation.run.project_id == runtime.project_id()
                && record.fine.project_id == runtime.project_id()
                && record.fine.run_id.as_deref() == Some(record.preparation.run.id.as_str()),
            "OBJECT_ATTEMPT_IDENTITY_MISMATCH"
        );
        if record.preparation.run.id == run_id {
            records.push(record);
        }
    }
    Ok(records)
}

pub(crate) fn start(runtime: &ProjectRuntime, claim: PreparationClaim) -> Result<Option<Lease>> {
    let prepared = object_run_preparation::prepare(runtime, claim)?;
    if prepared.state == PreparationState::Failed {
        return Ok(None);
    }
    transact(runtime, |connection| storage::start(connection, prepared))
}

pub(crate) fn validate(runtime: &ProjectRuntime, lease: &Lease) -> Result<()> {
    ensure!(
        runtime.project_id() == lease.record.preparation.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    transact(runtime, |connection| storage::current(connection, lease))
}

pub(crate) fn validate_callback(runtime: &ProjectRuntime, lease: &Lease) -> Result<()> {
    ensure!(
        runtime.project_id() == lease.record.preparation.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    transact(runtime, |connection| {
        storage::current(connection, lease)?;
        ensure!(
            !crate::object_attempt_control::requested(connection, &lease.record)?,
            "OBJECT_ATTEMPT_INTERRUPTED"
        );
        Ok(())
    })
}

pub(crate) fn record_event(
    runtime: &ProjectRuntime,
    lease: &Lease,
    entry: crate::object_attempt_trace::Entry,
) -> Result<()> {
    ensure!(
        runtime.project_id() == lease.record.preparation.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    transact(runtime, |connection| {
        storage::current(connection, lease)?;
        crate::object_attempt_trace::append(connection, lease, entry)
    })
}

pub(crate) fn bind(
    runtime: &ProjectRuntime,
    lease: &mut Lease,
    thread: &str,
    turn: Option<&str>,
) -> Result<()> {
    validate(runtime, lease)?;
    ensure!(
        !thread.is_empty() && turn.is_none_or(|id| !id.is_empty()),
        "OBJECT_ATTEMPT_RPC_IDENTITY_MISMATCH"
    );
    ensure!(
        lease
            .record
            .thread_id
            .as_deref()
            .is_none_or(|id| id == thread)
            && lease.record.turn_id.is_none(),
        "OBJECT_ATTEMPT_RPC_ALREADY_BOUND"
    );
    let mut record = lease.record.clone();
    record.thread_id = Some(thread.into());
    record.turn_id = turn.map(str::to_owned);
    transact(runtime, |connection| {
        storage::current(connection, lease)?;
        ensure!(
            !crate::object_attempt_control::requested(connection, &lease.record)?,
            "OBJECT_ATTEMPT_INTERRUPTED"
        );
        object_task_storage::replace(connection, KIND, &record.id, &record)
    })?;
    lease.record = record;
    Ok(())
}

/// Caller must have successfully disposed of the owned process tree. A close
/// failure must leave this lease running and must never reach checkpoint capture.
pub(crate) fn finish(
    runtime: &ProjectRuntime,
    lease: Lease,
    state: State,
    error: Option<String>,
    cancelled: &AtomicBool,
) -> Result<()> {
    ensure!(state != State::Running, "OBJECT_ATTEMPT_INVALID_FINISH");
    validate(runtime, &lease)?;
    let preparation = &lease.record.preparation;
    let path = runtime.files().resolve_workspace(
        &preparation.run.id,
        std::path::Path::new(&preparation.workspace),
    )?;
    let output = runtime.files().capture(&path)?;
    transact(runtime, |connection| {
        // Cancellation is sampled at the final checkpoint transaction, after
        // capture. Later interruption cannot rewrite an already frozen result.
        let (state, error) = if cancelled.load(Ordering::SeqCst)
            || crate::object_attempt_control::requested(connection, &lease.record)?
        {
            (
                State::Interrupted,
                Some("OBJECT_ATTEMPT_INTERRUPTED".into()),
            )
        } else {
            (state, error)
        };
        storage::finish(connection, lease, state, error, output)
    })
}

fn transact<T>(
    runtime: &ProjectRuntime,
    work: impl FnOnce(&rusqlite::Connection) -> Result<T>,
) -> Result<T> {
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object attempt store lock poisoned"))?;
    store.transaction(work)
}
