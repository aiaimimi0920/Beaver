//! Durable interruption intent; completion is frozen with the output checkpoint.
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_view::{self as views, Target, View},
    object_task_storage as storage,
    object_task_types::{valid_id, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub(crate) const KIND: &str = "object_attempt_interrupt_receipt";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterruptRequest {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
    pub expected_task_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request: InterruptRequest,
    pub result: View,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Pending {
    pub(crate) request: InterruptRequest,
    pub(crate) result: Option<View>,
}

fn validate(runtime: &ProjectRuntime, request: &InterruptRequest) -> Result<()> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let target = &request.target;
    ensure!(
        [
            &request.project_id,
            &request.request_id,
            &target.task_id,
            &target.fine_task_id,
            &target.object_id,
            &target.run_id,
            &target.attempt_id
        ]
        .into_iter()
        .all(|id| valid_id(id))
            && !target.owner.trim().is_empty()
            && target.owner.len() <= 128
            && !target.claim_token.is_empty()
            && target.claim_token.len() <= 128
            && target.generation > 0
            && target.generation <= MAX_REVISION
            && request.expected_task_revision <= MAX_REVISION
            && [&target.thread_id, &target.turn_id].into_iter().all(|id| id
                .as_ref()
                .is_none_or(|id| !id.is_empty() && id.len() <= 128)),
        "INVALID_OBJECT_ATTEMPT_INTERRUPT"
    );
    Ok(())
}

fn replay(connection: &Connection, request: &InterruptRequest) -> Result<Option<Pending>> {
    let receipt: Option<Pending> = storage::read(connection, KIND, &request.request_id)?;
    if let Some(receipt) = &receipt {
        ensure!(
            receipt.request == *request,
            "OBJECT_ATTEMPT_REQUEST_CONFLICT"
        );
        validate_result(receipt)?;
    }
    Ok(receipt)
}

pub(crate) fn validate_result(receipt: &Pending) -> Result<()> {
    if let Some(result) = &receipt.result {
        ensure!(
            result.project_id == receipt.request.project_id
                && result.target == receipt.request.target
                && result.state != State::Running
                && result.output_captured
                && result.task_revision <= MAX_REVISION,
            "OBJECT_ATTEMPT_RECEIPT_MISMATCH"
        );
    }
    Ok(())
}

/// Called by the scheduler with its live writer match. False means a durable
/// result already exists; true means this exact worker must be stopped and joined.
pub(crate) fn request(
    runtime: &ProjectRuntime,
    request: &InterruptRequest,
    live: bool,
) -> Result<bool> {
    validate(runtime, request)?;
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object attempt store lock poisoned"))?;
    store.transaction(|connection| {
        let previous = replay(connection, request)?;
        if previous
            .as_ref()
            .is_some_and(|receipt| receipt.result.is_some())
        {
            return Ok(false);
        }
        let record = views::read(connection, &request.project_id, &request.target.attempt_id)?;
        ensure!(
            Target::from_record(&record) == request.target,
            "OBJECT_ATTEMPT_STALE_TARGET"
        );
        let view = views::view(connection, &record)?;
        if record.state == State::Running {
            ensure!(
                view.task_revision == request.expected_task_revision,
                "OBJECT_ATTEMPT_REVISION_CONFLICT"
            );
            ensure!(live, "OBJECT_ATTEMPT_RECOVERY_REQUIRED");
        }
        let pending = record.state == State::Running;
        ensure!(
            pending || previous.is_none(),
            "OBJECT_ATTEMPT_RECEIPT_INCOMPLETE"
        );
        if previous.is_none() {
            storage::insert(
                connection,
                KIND,
                &request.request_id,
                &Pending {
                    request: request.clone(),
                    result: (!pending).then_some(view),
                },
            )?;
        }
        Ok(pending)
    })
}

pub(crate) fn result(runtime: &ProjectRuntime, request: &InterruptRequest) -> Result<Receipt> {
    validate(runtime, request)?;
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object attempt store lock poisoned"))?;
    let receipt =
        replay(&store.connection, request)?.context("OBJECT_ATTEMPT_INTERRUPT_NOT_FOUND")?;
    Ok(Receipt {
        request: receipt.request,
        result: receipt.result.context("OBJECT_ATTEMPT_INTERRUPT_PENDING")?,
    })
}

pub(crate) fn for_recovery(connection: &Connection, record: &Attempt) -> Result<Vec<Pending>> {
    let target = Target::from_record(record);
    let mut statement =
        connection.prepare("SELECT id,value FROM entities WHERE kind=? ORDER BY id")?;
    let receipts = statement.query_map([KIND], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut matches = Vec::new();
    for receipt in receipts {
        let (id, value) = receipt?;
        let receipt: Pending = serde_json::from_str(&value)?;
        ensure!(
            id == receipt.request.request_id,
            "OBJECT_ATTEMPT_RECEIPT_MISMATCH"
        );
        validate_result(&receipt)?;
        if receipt.request.target.attempt_id == record.id {
            ensure!(
                receipt.request.project_id == record.preparation.project_id
                    && receipt.request.target == target,
                "OBJECT_ATTEMPT_STALE_TARGET"
            );
            matches.push(receipt);
        }
    }
    Ok(matches)
}

pub(crate) fn requested(connection: &Connection, record: &Attempt) -> Result<bool> {
    Ok(for_recovery(connection, record)?
        .iter()
        .any(|receipt| receipt.result.is_none()))
}

pub(crate) fn complete(connection: &Connection, record: &Attempt) -> Result<()> {
    for mut receipt in for_recovery(connection, record)?
        .into_iter()
        .filter(|receipt| receipt.result.is_none())
    {
        receipt.result = Some(views::view(connection, record)?);
        storage::replace(connection, KIND, &receipt.request.request_id, &receipt)?;
    }
    Ok(())
}
