//! Atomic verifier claims and immutable receipts; no filesystem or process work.
use super::{
    records::{self, Records},
    Operation, Report, Target, VerifyRequest, View, MAX_GENERATION,
};
use crate::{
    object_task_storage as store, object_task_types::valid_id, project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_recovery_verification";
const HEAD_KIND: &str = "object_recovery_head";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Stored {
    pub operation: Operation,
    pub records: Records,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Head {
    project_id: String,
    task_id: String,
    run_id: String,
    request_id: String,
    generation: u64,
}

pub(super) fn validate(runtime: &ProjectRuntime, request: &VerifyRequest) -> Result<()> {
    ensure!(
        runtime.project_id() == request.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    validate_request(request)
}

fn validate_request(request: &VerifyRequest) -> Result<()> {
    validate_target(&request.project_id, &request.request_id, &request.target)?;
    ensure!(
        request.target.recovery_generation < MAX_GENERATION,
        "INVALID_OBJECT_RECOVERY_REQUEST"
    );
    Ok(())
}

pub(super) fn validate_target(project: &str, request: &str, target: &Target) -> Result<()> {
    ensure!(
        [
            project,
            request,
            &target.task_id,
            &target.object_id,
            &target.run_id,
        ]
        .into_iter()
        .all(|id| valid_id(id))
            && !target.owner.trim().is_empty()
            && target.owner.len() <= 128
            && !target.claim_token.is_empty()
            && target.claim_token.len() <= 128
            && target.writer_generation > 0
            && [
                target.writer_generation,
                target.task_revision,
                target.run_revision,
                target.object_revision,
                target.control_revision,
                target.recovery_generation
            ]
            .into_iter()
            .all(|value| value <= MAX_GENERATION),
        "INVALID_OBJECT_RECOVERY_REQUEST"
    );
    Ok(())
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
        let operation = &saved.operation;
        validate_request(&operation.request)?;
        ensure!(
            operation.schema_version == 1
                && operation.request.project_id == project
                && operation.request.request_id == request
                && operation.generation == operation.request.target.recovery_generation + 1
                && saved.records.medium.project_id == project
                && saved
                    .records
                    .target(operation.request.target.recovery_generation)
                    == operation.request.target,
            "OBJECT_RECOVERY_RECEIPT_MISMATCH"
        );
        if let Some(report) = &operation.result {
            ensure!(
                report.paused == saved.records.control.paused,
                "OBJECT_RECOVERY_RECEIPT_MISMATCH"
            );
        }
    }
    Ok(saved)
}

pub(super) fn latest(
    connection: &Connection,
    project: &str,
    task: &str,
    run: &str,
) -> Result<Option<Stored>> {
    let Some(head) = store::read::<Head>(connection, HEAD_KIND, run)? else {
        return Ok(None);
    };
    ensure!(
        head.project_id == project && head.task_id == task && head.run_id == run,
        "OBJECT_RECOVERY_HEAD_MISMATCH"
    );
    let saved =
        read(connection, project, &head.request_id)?.context("OBJECT_RECOVERY_RECEIPT_MISSING")?;
    ensure!(
        saved.operation.generation == head.generation
            && saved.operation.request.target.task_id == task
            && saved.operation.request.target.run_id == run,
        "OBJECT_RECOVERY_HEAD_MISMATCH"
    );
    Ok(Some(saved))
}

pub(super) fn view(connection: &Connection, project: &str, task: &str) -> Result<Option<View>> {
    if let Some(view) = super::disposition::retained_view(connection, project, task)? {
        return Ok(Some(view));
    }
    if super::candidate::publication::published_task(connection, project, task)? {
        return Ok(None);
    }
    let Some(records) = records::read(connection, project, task)? else {
        return Ok(None);
    };
    let saved = latest(connection, project, task, &records.run.id)?;
    let report_matches_records = saved
        .as_ref()
        .is_some_and(|saved| saved.operation.result.is_some() && saved.records == records);
    let operation = saved.map(|saved| saved.operation);
    let disposition = super::disposition::latest(connection, project, task)?;
    let resume = super::resume::latest(connection, project, task)?;
    let can_dispose = !super::candidate::publication::task_pending(connection, project, task)?
        && resume.as_ref().is_none_or(|op| op.result.is_some())
        && report_matches_records
        && super::disposition::permits_new(
            disposition.as_ref(),
            operation
                .as_ref()
                .map_or(0, |operation| operation.generation),
        )
        && operation
            .as_ref()
            .and_then(|operation| operation.result.as_ref())
            .is_some_and(Report::can_dispose);
    let can_resume = can_dispose
        && super::resume::eligible(&records)
        && resume.as_ref().is_none_or(|op| {
            op.request.target.recovery_generation < operation.as_ref().map_or(0, |op| op.generation)
        })
        && !crate::object_task_coarse_dispatch::parent_paused(connection, &records.medium)?;
    Ok(Some(View {
        target: records.target(
            operation
                .as_ref()
                .map_or(0, |operation| operation.generation),
        ),
        preparation_state: records.preparation.state,
        paused: records.control.paused,
        operation,
        disposition,
        report_matches_records,
        can_dispose,
        resume,
        can_resume,
    }))
}

pub(super) fn begin(connection: &Connection, request: &VerifyRequest) -> Result<Stored> {
    if let Some(saved) = read(connection, &request.project_id, &request.request_id)? {
        ensure!(
            saved.operation.request == *request,
            "OBJECT_RECOVERY_REQUEST_CONFLICT"
        );
        // Finished receipts remain replayable after later control/lifecycle changes.
        if saved.operation.result.is_none() {
            ensure!(
                latest(
                    connection,
                    &request.project_id,
                    &request.target.task_id,
                    &request.target.run_id
                )?
                .as_ref()
                    == Some(&saved),
                "OBJECT_RECOVERY_HEAD_MISMATCH"
            );
        }
        return Ok(saved);
    }
    super::candidate::publication::require_task_idle(
        connection,
        &request.project_id,
        &request.target.task_id,
    )?;
    super::resume::require_idle(connection, &request.project_id, &request.target.task_id)?;
    super::disposition::require_verifiable(
        connection,
        &request.project_id,
        &request.target.task_id,
    )?;
    let records = records::read(connection, &request.project_id, &request.target.task_id)?
        .context("OBJECT_RECOVERY_NOT_REQUIRED")?;
    let previous = latest(
        connection,
        &request.project_id,
        &request.target.task_id,
        &records.run.id,
    )?;
    ensure!(
        previous
            .as_ref()
            .is_none_or(|saved| saved.operation.result.is_some()),
        "OBJECT_RECOVERY_PENDING"
    );
    let generation = previous
        .as_ref()
        .map_or(0, |saved| saved.operation.generation);
    ensure!(
        records.target(generation) == request.target,
        "OBJECT_RECOVERY_STALE_TARGET"
    );
    let saved = Stored {
        operation: Operation {
            schema_version: 1,
            request: request.clone(),
            generation: generation + 1,
            result: None,
        },
        records,
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
        &request.target.run_id,
        &Head {
            project_id: request.project_id.clone(),
            task_id: request.target.task_id.clone(),
            run_id: request.target.run_id.clone(),
            request_id: request.request_id.clone(),
            generation: generation + 1,
        },
    )?;
    Ok(saved)
}

pub(super) fn finish(
    connection: &Connection,
    mut saved: Stored,
    mut report: Report,
) -> Result<Operation> {
    let request = &saved.operation.request;
    ensure!(
        latest(
            connection,
            &request.project_id,
            &request.target.task_id,
            &request.target.run_id
        )?
        .as_ref()
            == Some(&saved),
        "OBJECT_RECOVERY_HEAD_MISMATCH"
    );
    // Complete stale or damaged inputs as evidence, so a crashed verifier can
    // always be resolved using its original request without stranding the head.
    match records::read(connection, &request.project_id, &request.target.task_id) {
        Ok(Some(current)) if current == saved.records => {}
        result => {
            report.records_current = false;
            report.issues.push(match result {
                Err(error) => format!("OBJECT_RECOVERY_RECORDS_INVALID: {error:#}"),
                _ => "OBJECT_RECOVERY_RECORDS_CHANGED".into(),
            });
        }
    }
    saved.operation.result = Some(report);
    store::replace(
        connection,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        &saved,
    )?;
    Ok(saved.operation)
}
