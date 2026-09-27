//! Retry journal and atomic attempt creation. Replay never reconstructs the lease.
use super::{eligible, records, verification, Operation, Outcome, Request};
use crate::{
    object_attempt::{self, Attempt, Lease},
    object_task_storage as store,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_recovery_resume";
const HEAD_KIND: &str = "object_recovery_resume_head";
pub(super) const HISTORY_KIND: &str = "object_recovery_attempt_successor";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Stored {
    pub operation: Operation,
    pub verification: verification::Stored,
    pub started: Option<Attempt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<crate::object_task_types::TaskRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fresh_checks: Option<Vec<crate::object_attempt_checks::Rule>>,
}

fn key(project: &str, request: &str) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(project, request))?)
    ))
}

pub(super) fn read(
    connection: &Connection,
    project: &str,
    request: &str,
) -> Result<Option<Stored>> {
    let saved: Option<Stored> = store::read(connection, KIND, &key(project, request)?)?;
    if let Some(saved) = &saved {
        let op = &saved.operation;
        ensure!(
            op.schema_version == 1
                && op.request.project_id == project
                && op.request.request_id == request
                && verification::read(connection, project, &op.request.verification_request_id)?
                    .as_ref()
                    == Some(&saved.verification)
                && saved
                    .verification
                    .records
                    .target(saved.verification.operation.generation)
                    == op.request.target
                && saved
                    .verification
                    .operation
                    .result
                    .as_ref()
                    .is_some_and(super::Report::can_dispose),
            "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH"
        );
        match (&op.result, &saved.started) {
            (
                Some(Outcome::Started {
                    report,
                    attempt_id,
                    fine_task_id,
                }),
                Some(started),
            ) => ensure!(
                report.can_dispose()
                    && started.id == *attempt_id
                    && started.fine.id == *fine_task_id
                    && started.schema_version == 1
                    && started.state == object_attempt::State::Running
                    && started.thread_id.is_none()
                    && started.turn_id.is_none()
                    && started.output.is_none()
                    && started.error.is_none()
                    && saved
                        .next
                        .as_ref()
                        .or(saved.verification.records.fine.as_ref())
                        == Some(&started.fine)
                    && saved
                        .verification
                        .records
                        .attempt
                        .as_ref()
                        .is_some_and(|previous| previous.preparation == started.preparation
                            && previous.output.as_ref() == Some(&started.input)),
                "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH"
            ),
            (Some(Outcome::Blocked { report }), None) => ensure!(
                !report.can_dispose(),
                "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH"
            ),
            (None, None) => {}
            _ => anyhow::bail!("OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH"),
        }
        if op.request.rework.is_some() {
            super::rework::validate_receipt(connection, saved)?;
        } else {
            super::advance::validate_receipt(saved)?;
        }
    }
    Ok(saved)
}

pub(super) fn latest(connection: &Connection, project: &str, task: &str) -> Result<Option<Stored>> {
    let Some(id) = store::read::<String>(connection, HEAD_KIND, task)? else {
        return Ok(None);
    };
    let saved =
        read(connection, project, &id)?.context("OBJECT_RECOVERY_RESUME_RECEIPT_MISSING")?;
    ensure!(
        saved.operation.request.target.task_id == task,
        "OBJECT_RECOVERY_RESUME_HEAD_MISMATCH"
    );
    Ok(Some(saved))
}

pub(super) fn begin(connection: &Connection, request: &Request) -> Result<Stored> {
    if let Some(saved) = read(connection, &request.project_id, &request.request_id)? {
        ensure!(
            saved.operation.request == *request,
            "OBJECT_RECOVERY_RESUME_REQUEST_CONFLICT"
        );
        return Ok(saved);
    }
    super::super::candidate::publication::require_task_idle(
        connection,
        &request.project_id,
        &request.target.task_id,
    )?;
    super::super::disposition::require_verifiable(
        connection,
        &request.project_id,
        &request.target.task_id,
    )?;
    if let Some(previous) = latest(connection, &request.project_id, &request.target.task_id)? {
        ensure!(
            previous.operation.result.is_some(),
            "OBJECT_RECOVERY_RESUME_PENDING"
        );
        ensure!(
            previous.operation.request.target.recovery_generation
                < request.target.recovery_generation,
            "OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED"
        );
    }
    let verification = verification::latest(
        connection,
        &request.project_id,
        &request.target.task_id,
        &request.target.run_id,
    )?
    .context("OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED")?;
    ensure!(
        verification.operation.request.request_id == request.verification_request_id
            && verification
                .records
                .target(verification.operation.generation)
                == request.target
            && verification
                .operation
                .result
                .as_ref()
                .is_some_and(super::Report::can_dispose),
        "OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED"
    );
    ensure!(
        records::read(connection, &request.project_id, &request.target.task_id)?.as_ref()
            == Some(&verification.records),
        "OBJECT_RECOVERY_STALE_TARGET"
    );
    let next = if request.advance.is_some() {
        Some(super::advance::select(
            connection,
            &verification.records,
            request,
        )?)
    } else if request.rework.is_some() {
        Some(super::rework::select(
            connection,
            &verification.records,
            request,
        )?)
    } else {
        ensure!(
            eligible(&verification.records),
            "OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED"
        );
        None
    };
    let saved = Stored {
        operation: Operation {
            schema_version: 1,
            request: request.clone(),
            result: None,
        },
        verification,
        started: None,
        next,
        fresh_checks: None,
    };
    store::insert(
        connection,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        &saved,
    )?;
    store::replace(
        connection,
        HEAD_KIND,
        &request.target.task_id,
        &request.request_id,
    )?;
    Ok(saved)
}

pub(super) fn finish(
    connection: &Connection,
    mut saved: Stored,
    mut report: super::Report,
) -> Result<(Operation, Option<Lease>)> {
    let request = &saved.operation.request;
    let mut pending = saved.clone();
    pending.fresh_checks = None;
    ensure!(
        latest(connection, &request.project_id, &request.target.task_id)?.as_ref()
            == Some(&pending),
        "OBJECT_RECOVERY_RESUME_HEAD_MISMATCH"
    );
    let records = &saved.verification.records;
    if !matches!(records::read(connection, &request.project_id, &request.target.task_id), Ok(Some(current)) if current == *records)
    {
        report.records_current = false;
        report
            .issues
            .push("OBJECT_RECOVERY_RESUME_RECORDS_CHANGED".into());
    }
    if crate::object_task_coarse_dispatch::parent_paused(connection, &records.medium)? {
        report.issues.push("OBJECT_RECOVERY_COARSE_PAUSED".into());
    }
    if request.advance.is_some()
        && !matches!(super::advance::select(connection, records, request), Ok(next) if Some(&next) == saved.next.as_ref())
    {
        report.issues.push("OBJECT_STAGE_SUCCESSOR_CHANGED".into());
    }
    if request.rework.is_some()
        && !matches!(super::rework::select(connection, records, request), Ok(next) if Some(&next) == saved.next.as_ref())
    {
        report.issues.push("OBJECT_CANDIDATE_RECORD_CHANGED".into());
    }
    let lease = if report.can_dispose() {
        let previous = records
            .attempt
            .as_ref()
            .context("OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED")?;
        if request.advance.is_some() {
            super::advance::commit(connection, records)?;
        }
        let lease = object_attempt::successor(
            connection,
            previous,
            records.medium.clone(),
            records.run.clone(),
            saved
                .next
                .clone()
                .or(records.fine.clone())
                .context("OBJECT_ATTEMPT_NOT_FOUND")?,
        )?;
        let started = lease.record().clone();
        saved.operation.result = Some(Outcome::Started {
            report,
            attempt_id: started.id.clone(),
            fine_task_id: started.fine.id.clone(),
        });
        saved.started = Some(started);
        store::insert(connection, HISTORY_KIND, &previous.id, &request.request_id)?;
        Some(lease)
    } else {
        saved.operation.result = Some(Outcome::Blocked { report });
        None
    };
    store::replace(
        connection,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        &saved,
    )?;
    Ok((saved.operation, lease))
}
