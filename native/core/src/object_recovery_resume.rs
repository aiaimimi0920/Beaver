//! Explicit retry authorization; only the committing caller receives a worker lease.
use super::{files, records, storage as verification, transact, Report, Target};
use crate::{
    object_attempt::{Lease, State},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "object_stage_advance.rs"]
pub mod advance;
#[path = "object_recovery_attempt_history.rs"]
mod history;
#[path = "object_candidate_rework.rs"]
pub mod rework;
#[path = "object_recovery_resume_store.rs"]
mod storage;
pub(crate) use history::{chain, retained_attempt};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
    pub verification_request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance: Option<advance::Approval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rework: Option<rework::Approval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Outcome {
    Started {
        report: Report,
        attempt_id: String,
        fine_task_id: String,
    },
    Blocked {
        report: Report,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub schema_version: u32,
    pub request: Request,
    pub result: Option<Outcome>,
}

fn validate(runtime: &ProjectRuntime, request: &Request) -> Result<()> {
    ensure!(
        runtime.project_id() == request.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    verification::validate_target(&request.project_id, &request.request_id, &request.target)?;
    ensure!(
        request.advance.is_none() || request.rework.is_none(),
        "INVALID_OBJECT_RESUME_PURPOSE"
    );
    if let Some(approval) = &request.rework {
        rework::validate(approval)?;
    }
    if let Some(approval) = &request.advance {
        advance::validate(approval)?;
    }
    ensure!(
        crate::object_task_types::valid_id(&request.verification_request_id)
            && request.target.recovery_generation > 0,
        "INVALID_OBJECT_RECOVERY_REQUEST"
    );
    Ok(())
}

pub(crate) fn replay(runtime: &ProjectRuntime, request: &Request) -> Result<Option<Operation>> {
    validate(runtime, request)?;
    transact(runtime, |connection| {
        let saved = storage::read(connection, &request.project_id, &request.request_id)?;
        if let Some(saved) = &saved {
            ensure!(
                saved.operation.request == *request,
                "OBJECT_RECOVERY_RESUME_REQUEST_CONFLICT"
            );
        }
        Ok(saved
            .filter(|saved| saved.operation.result.is_some())
            .map(|saved| saved.operation))
    })
}

pub(super) fn latest(
    connection: &rusqlite::Connection,
    project: &str,
    task: &str,
) -> Result<Option<Operation>> {
    Ok(storage::latest(connection, project, task)?.map(|saved| saved.operation))
}

pub(super) fn require_idle(
    connection: &rusqlite::Connection,
    project: &str,
    task: &str,
) -> Result<()> {
    ensure!(
        latest(connection, project, task)?.is_none_or(|op| op.result.is_some()),
        "OBJECT_RECOVERY_RESUME_PENDING"
    );
    Ok(())
}

pub(super) fn eligible(records: &records::Records) -> bool {
    records
        .attempt
        .as_ref()
        .is_some_and(|attempt| matches!(attempt.state, State::Failed | State::Interrupted))
}

pub(crate) fn execute(
    runtime: &ProjectRuntime,
    request: &Request,
    cancelled: &AtomicBool,
) -> Result<(Operation, Option<Lease>)> {
    execute_with(runtime, request, cancelled, || Ok(()))
}

pub(crate) fn execute_with(
    runtime: &ProjectRuntime,
    request: &Request,
    cancelled: &AtomicBool,
    after_files: impl FnOnce() -> Result<()>,
) -> Result<(Operation, Option<Lease>)> {
    validate(runtime, request)?;
    let _guard = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let mut saved = transact(runtime, |connection| storage::begin(connection, request))?;
    if saved.operation.result.is_some() {
        return Ok((saved.operation, None));
    }
    let mut report = files::verify(runtime, &saved.verification.records, false);
    if let Some(image) = request
        .rework
        .as_ref()
        .and_then(|approval| approval.image.as_ref())
    {
        if let Err(error) = image.verify(
            runtime,
            saved.verification.records.attempt.as_ref().unwrap(),
        ) {
            report.issues.push(error.to_string());
        }
    }
    if request.advance.is_some() {
        let attempt = saved.verification.records.attempt.as_ref().unwrap();
        let rules = crate::object_attempt_checks::recheck(runtime, attempt);
        if !rules.iter().all(|rule| rule.passed) {
            report
                .issues
                .push("OBJECT_STAGE_TECHNICAL_CHECK_FAILED".into());
        }
        // Persist the fresh rules in the same receipt that records owner authorization.
        saved.fresh_checks = Some(rules);
    }
    after_files()?;
    if cancelled.load(Ordering::SeqCst) {
        report
            .issues
            .push("OBJECT_RECOVERY_SCHEDULER_STOPPED".into());
    }
    transact(runtime, |connection| {
        storage::finish(connection, saved, report)
    })
}
