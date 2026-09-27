//! A durable retention journal and its immutable completion receipt share one project store.
use super::super::{records, Report};
use super::{cancel, permits_new, verification, Operation, Outcome, Request};
use crate::object_recovery_resource;
use crate::{object_task_storage as store, object_task_types::TaskRecord};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_recovery_disposition";
const HEAD_KIND: &str = "object_recovery_disposition_head";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Stored {
    pub operation: Operation,
    pub verification: verification::Stored,
    pub children: Vec<TaskRecord>,
    #[serde(default)]
    pub resource: Option<object_recovery_resource::Journal>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Head {
    project_id: String,
    task_id: String,
    request_id: String,
}

fn key(project: &str, request: &str) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(project, request))?)
    ))
}

fn read(connection: &Connection, project: &str, request_id: &str) -> Result<Option<Stored>> {
    let saved: Option<Stored> = store::read(connection, KIND, &key(project, request_id)?)?;
    if let Some(saved) = &saved {
        let request = &saved.operation.request;
        let verification = &saved.verification;
        super::validate(request)?;
        ensure!(
            saved.operation.schema_version == 1
                && request.project_id == project
                && request.request_id == request_id
                && verification.operation.request.project_id == project
                && verification.operation.request.request_id == request.verification_request_id
                && verification
                    .records
                    .target(verification.operation.generation)
                    == request.target
                && verification
                    .operation
                    .result
                    .as_ref()
                    .is_some_and(Report::can_dispose)
                && verification::read(connection, project, &request.verification_request_id)?
                    .as_ref()
                    == Some(verification),
            "OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH"
        );
        cancel::validate_receipt(saved)?;
        match (&request.choice, &saved.resource) {
            (super::Choice::CancelAndKeep, None) => {}
            (super::Choice::CancelAndRemoveWorkspace, Some(journal)) => ensure!(
                journal.request_id == request.request_id
                    && journal.owner_id == request.target.owner
                    && journal.workspace == verification.records.preparation.workspace,
                "OBJECT_RECOVERY_RESOURCE_MISMATCH"
            ),
            _ => anyhow::bail!("OBJECT_RECOVERY_RESOURCE_MISMATCH"),
        }
    }
    Ok(saved)
}

pub(super) fn latest(connection: &Connection, project: &str, task: &str) -> Result<Option<Stored>> {
    let Some(head) = store::read::<Head>(connection, HEAD_KIND, task)? else {
        return Ok(None);
    };
    ensure!(
        head.project_id == project && head.task_id == task,
        "OBJECT_RECOVERY_DISPOSITION_HEAD_MISMATCH"
    );
    let saved = read(connection, project, &head.request_id)?
        .context("OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISSING")?;
    ensure!(
        saved.operation.request.target.task_id == task,
        "OBJECT_RECOVERY_DISPOSITION_HEAD_MISMATCH"
    );
    Ok(Some(saved))
}

pub(super) fn begin(connection: &Connection, request: &Request) -> Result<Stored> {
    if let Some(saved) = read(connection, &request.project_id, &request.request_id)? {
        ensure!(
            saved.operation.request == *request,
            "OBJECT_RECOVERY_DISPOSITION_REQUEST_CONFLICT"
        );
        if saved.operation.result.is_none() {
            ensure!(
                latest(connection, &request.project_id, &request.target.task_id)?.as_ref()
                    == Some(&saved),
                "OBJECT_RECOVERY_DISPOSITION_HEAD_MISMATCH"
            );
        }
        return Ok(saved);
    }
    super::super::candidate::publication::require_task_idle(
        connection,
        &request.project_id,
        &request.target.task_id,
    )?;
    super::super::resume::require_idle(connection, &request.project_id, &request.target.task_id)?;
    let previous = latest(connection, &request.project_id, &request.target.task_id)?;
    ensure!(
        previous
            .as_ref()
            .is_none_or(|saved| saved.operation.result.is_some()),
        "OBJECT_RECOVERY_DISPOSITION_PENDING"
    );
    ensure!(
        permits_new(
            previous.as_ref().map(|saved| &saved.operation),
            request.target.recovery_generation
        ),
        "OBJECT_RECOVERY_DISPOSITION_REVERIFY_REQUIRED"
    );
    let verification = verification::latest(
        connection,
        &request.project_id,
        &request.target.task_id,
        &request.target.run_id,
    )?
    .context("OBJECT_RECOVERY_DISPOSITION_REVERIFY_REQUIRED")?;
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
                .is_some_and(Report::can_dispose),
        "OBJECT_RECOVERY_DISPOSITION_REVERIFY_REQUIRED"
    );
    ensure!(
        records::read(connection, &request.project_id, &request.target.task_id)?.as_ref()
            == Some(&verification.records),
        "OBJECT_RECOVERY_STALE_TARGET"
    );
    let children = cancel::scope(connection, &verification.records)?;
    let workspace = verification.records.preparation.workspace.clone();
    let saved = Stored {
        operation: Operation {
            schema_version: 1,
            request: request.clone(),
            result: None,
        },
        verification,
        children,
        resource: match &request.choice {
            super::Choice::CancelAndRemoveWorkspace => {
                Some(object_recovery_resource::Journal::new(
                    request.request_id.clone(),
                    request.target.owner.clone(),
                    workspace.clone(),
                )?)
            }
            super::Choice::CancelAndKeep => None,
        },
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
        &Head {
            project_id: request.project_id.clone(),
            task_id: request.target.task_id.clone(),
            request_id: request.request_id.clone(),
        },
    )?;
    Ok(saved)
}

pub(super) fn persist_pending(
    connection: &Connection,
    previous: &Stored,
    saved: &Stored,
) -> Result<()> {
    let request = &saved.operation.request;
    ensure!(
        latest(connection, &request.project_id, &request.target.task_id)?.as_ref()
            == Some(previous),
        "OBJECT_RECOVERY_DISPOSITION_HEAD_MISMATCH"
    );
    store::replace(
        connection,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        saved,
    )
}
pub(super) fn finish(
    connection: &Connection,
    mut saved: Stored,
    mut report: Report,
) -> Result<Operation> {
    let request = &saved.operation.request;
    ensure!(
        latest(connection, &request.project_id, &request.target.task_id)?.as_ref() == Some(&saved),
        "OBJECT_RECOVERY_DISPOSITION_HEAD_MISMATCH"
    );
    if let Err(error) = cancel::validate_current(connection, &saved) {
        report.records_current = false;
        report.issues.push(format!(
            "OBJECT_RECOVERY_DISPOSITION_RECORDS_CHANGED: {error:#}"
        ));
    }
    let outcome = if report.can_dispose() {
        cancel::commit(
            connection,
            &saved,
            report,
            matches!(request.choice, super::Choice::CancelAndRemoveWorkspace),
        )?
    } else {
        Outcome::Blocked { report }
    };
    saved.operation.result = Some(outcome);
    let request = &saved.operation.request;
    store::replace(
        connection,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        &saved,
    )?;
    Ok(saved.operation)
}
