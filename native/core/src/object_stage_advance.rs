//! Explicit owner approval of one frozen fine; publication remains a separate boundary.
use super::{records::Records, Request};
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_checks,
    object_framework::{Identity, VERSION},
    object_task_storage::{self as store, TASK_KIND},
    object_task_types::{valid_id, Granularity, TaskRecord, MAX_REVISION},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub attempt_id: String,
    pub check_request_id: String,
    pub next_fine_task_id: String,
    pub next_fine_revision: u64,
    pub acceptance_note: String,
}

pub(super) fn validate(approval: &Approval) -> Result<()> {
    ensure!(
        valid_id(&approval.attempt_id)
            && valid_id(&approval.check_request_id)
            && valid_id(&approval.next_fine_task_id)
            && approval.next_fine_revision < MAX_REVISION
            && !approval.acceptance_note.trim().is_empty()
            && approval.acceptance_note.len() <= 4000,
        "INVALID_OBJECT_STAGE_APPROVAL"
    );
    Ok(())
}

pub(super) fn select(
    connection: &Connection,
    records: &Records,
    request: &Request,
) -> Result<TaskRecord> {
    let approval = request
        .advance
        .as_ref()
        .context("OBJECT_STAGE_APPROVAL_REQUIRED")?;
    let previous = records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    ensure!(
        previous.id == approval.attempt_id && previous.state == State::AwaitingGate,
        "OBJECT_STAGE_SUCCESS_REQUIRED"
    );
    object_attempt_checks::require_passed(connection, previous, &approval.check_request_id)?;
    let mut fines: Vec<_> = store::all_tasks(connection)?
        .into_iter()
        .filter(|task| {
            task.parent_task_id.as_deref() == Some(&records.medium.id)
                && task.granularity == Granularity::Fine
                && task.status != "cancelled"
        })
        .collect();
    fines.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
    let index = fines
        .iter()
        .position(|fine| fine.id == previous.fine.id)
        .context("OBJECT_STAGE_CURRENT_MISSING")?;
    ensure!(
        fines[..index].iter().all(|fine| fine.status == "accepted"),
        "OBJECT_STAGE_ORDER_MISMATCH"
    );
    let next = fines.get(index + 1).context("OBJECT_STAGE_NO_SUCCESSOR")?;
    ensure!(
        next.id == approval.next_fine_task_id
            && next.revision == approval.next_fine_revision
            && next.status == "planned"
            && next.project_id == request.project_id
            && next.run_id.as_deref() == Some(&records.run.id)
            && next.object_id == records.medium.object_id
            && matches!(&next.identity, Identity::Fine { schema_version, object_id, medium_task_id, run_id, stage_id }
            if *schema_version == VERSION && object_id == &records.run.object_id
                && medium_task_id == &records.medium.id && run_id == &records.run.id
                && next.stage_id.as_deref() == Some(stage_id.as_str())),
        "OBJECT_STAGE_SUCCESSOR_MISMATCH"
    );
    for id in &next.depends_on {
        if id == &previous.fine.id {
            continue;
        }
        ensure!(
            store::read::<TaskRecord>(connection, TASK_KIND, id)?
                .is_some_and(|task| task.id == *id
                    && task.project_id == request.project_id
                    && task.status == "accepted"),
            "OBJECT_STAGE_DEPENDENCY_NOT_ACCEPTED"
        );
    }
    Ok(next.clone())
}

pub(crate) fn accepted(attempt: &Attempt) -> Result<TaskRecord> {
    ensure!(
        attempt.state == State::AwaitingGate,
        "OBJECT_STAGE_SUCCESS_REQUIRED"
    );
    let mut fine = attempt.fine.clone();
    fine.revision = fine
        .revision
        .checked_add(3)
        .filter(|rev| *rev <= MAX_REVISION)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    fine.status = "accepted".into();
    Ok(fine)
}

pub(super) fn commit(connection: &Connection, records: &Records) -> Result<()> {
    let fine = accepted(
        records
            .attempt
            .as_ref()
            .context("OBJECT_ATTEMPT_NOT_FOUND")?,
    )?;
    store::replace(connection, TASK_KIND, &fine.id, &fine)
}

pub(super) fn validate_receipt(saved: &super::storage::Stored) -> Result<()> {
    let Some(approval) = &saved.operation.request.advance else {
        ensure!(
            saved.next.is_none() && saved.fresh_checks.is_none(),
            "OBJECT_STAGE_RECEIPT_MISMATCH"
        );
        return Ok(());
    };
    validate(approval)?;
    let previous = saved
        .verification
        .records
        .attempt
        .as_ref()
        .context("OBJECT_STAGE_RECEIPT_MISMATCH")?;
    let next = saved
        .next
        .as_ref()
        .context("OBJECT_STAGE_RECEIPT_MISMATCH")?;
    ensure!(
        previous.id == approval.attempt_id
            && previous.state == State::AwaitingGate
            && next.id == approval.next_fine_task_id
            && next.revision == approval.next_fine_revision
            && next.id != previous.fine.id
            && next.status == "planned",
        "OBJECT_STAGE_RECEIPT_MISMATCH"
    );
    if saved.operation.result.is_none() {
        ensure!(
            saved.fresh_checks.is_none(),
            "OBJECT_STAGE_RECEIPT_MISMATCH"
        );
    } else {
        let rules = saved
            .fresh_checks
            .as_ref()
            .context("OBJECT_STAGE_RECEIPT_MISMATCH")?;
        ensure!(
            rules.len() == 2
                && rules[0].id == "checkpoint-integrity"
                && rules[1].id == "code-structure"
                && rules.iter().all(|rule| rule.version == 1)
                && (saved.started.is_none()
                    || rules
                        .iter()
                        .all(|rule| rule.passed && rule.issues.is_empty())),
            "OBJECT_STAGE_RECEIPT_MISMATCH"
        );
    }
    Ok(())
}
