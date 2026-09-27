//! Retention's final transaction closes future work without rewriting a frozen attempt.
use super::super::{
    records::{self, Records},
    Report, MAX_GENERATION as MAX_REVISION,
};
use super::{storage::Stored, verification, Outcome, Retention};
use crate::{
    object_attempt_view, object_run_preparation,
    object_task_queue::{self, claim, QUEUE_KIND},
    object_task_storage::{self as store, RUN_KIND, STATE_KIND, TASK_KIND},
    object_task_types::{Granularity, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

pub(super) fn scope(connection: &Connection, records: &Records) -> Result<Vec<TaskRecord>> {
    let mut children: Vec<_> = store::all_tasks(connection)?
        .into_iter()
        .filter(|task| {
            task.parent_task_id.as_deref() == Some(&records.medium.id)
                || (task.id != records.medium.id && task.run_id.as_deref() == Some(&records.run.id))
        })
        .collect();
    children.sort_by(|a, b| a.id.cmp(&b.id));
    validate_scope(records, &children)?;
    Ok(children)
}

fn validate_scope(records: &Records, children: &[TaskRecord]) -> Result<()> {
    let fine = records
        .fine
        .as_ref()
        .context("OBJECT_RECOVERY_STOP_UNCONFIRMED")?;
    ensure!(
        children.windows(2).all(|pair| pair[0].id < pair[1].id),
        "OBJECT_RECOVERY_DISPOSITION_SCOPE_MISMATCH"
    );
    ensure!(
        children.iter().any(|task| task == fine),
        "OBJECT_RECOVERY_DISPOSITION_SCOPE_MISMATCH"
    );
    for task in children {
        ensure!(
            task.project_id == records.medium.project_id
                && task.parent_task_id.as_deref() == Some(&records.medium.id)
                && task.granularity == Granularity::Fine
                && task.run_id.as_deref() == Some(&records.run.id)
                && task.object_id == records.medium.object_id
                && (task == fine
                    || matches!(task.status.as_str(), "planned" | "cancelled")
                    || records.history.iter().any(
                        |attempt| super::super::resume::advance::accepted(attempt)
                            .as_ref()
                            .ok()
                            == Some(task)
                    )),
            "OBJECT_RECOVERY_DISPOSITION_SCOPE_MISMATCH"
        );
    }
    Ok(())
}

pub(super) fn validate_receipt(saved: &Stored) -> Result<()> {
    let records = &saved.verification.records;
    validate_scope(records, &saved.children)?;
    match (&saved.operation.request.choice, &saved.operation.result) {
        (super::Choice::CancelAndKeep, Some(Outcome::CancelledAndWorkspaceRemoved { .. }))
        | (super::Choice::CancelAndRemoveWorkspace, Some(Outcome::CancelledAndRetained { .. })) => {
            anyhow::bail!("OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH")
        }
        (_, Some(Outcome::CancelledAndWorkspaceRemoved { .. })) => ensure!(
            saved.resource.as_ref().is_some_and(
                |journal| journal.state == crate::object_recovery_resource::State::Completed
            ),
            "OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH"
        ),
        _ => {}
    }
    let Some(outcome) = &saved.operation.result else {
        return Ok(());
    };
    let report = match outcome {
        Outcome::CancelledAndRetained {
            report,
            retained,
            task_revision,
            run_revision,
            plan_revision,
        }
        | Outcome::CancelledAndWorkspaceRemoved {
            report,
            removed: retained,
            task_revision,
            run_revision,
            plan_revision,
        } => {
            let attempt = records
                .attempt
                .as_ref()
                .context("OBJECT_RECOVERY_STOP_UNCONFIRMED")?;
            let output = attempt
                .output
                .as_ref()
                .context("OBJECT_RECOVERY_STOP_UNCONFIRMED")?;
            ensure!(
                report.can_dispose()
                    && retained.workspace == records.preparation.workspace
                    && retained.attempt_id == attempt.id
                    && retained.fine_task_id == attempt.fine.id
                    && retained.output_file_count == output.len()
                    && records.medium.revision < MAX_REVISION
                    && records.run.revision < MAX_REVISION
                    && *task_revision == records.medium.revision + 1
                    && *run_revision == records.run.revision + 1
                    && (1..=MAX_REVISION).contains(plan_revision)
                    && saved
                        .children
                        .iter()
                        .all(|task| task.status != "planned" || task.revision < MAX_REVISION),
                "OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH"
            );
            report
        }
        Outcome::Blocked { report } => {
            ensure!(
                !report.can_dispose(),
                "OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH"
            );
            report
        }
    };
    ensure!(
        report.paused == records.control.paused,
        "OBJECT_RECOVERY_DISPOSITION_RECEIPT_MISMATCH"
    );
    Ok(())
}

pub(super) fn validate_current(connection: &Connection, saved: &Stored) -> Result<()> {
    let request = &saved.operation.request;
    let records = &saved.verification.records;
    ensure!(
        records::read(connection, &request.project_id, &request.target.task_id)?.as_ref()
            == Some(records)
            && scope(connection, records)? == saved.children
            && verification::latest(
                connection,
                &request.project_id,
                &request.target.task_id,
                &request.target.run_id
            )?
            .as_ref()
                == Some(&saved.verification),
        "OBJECT_RECOVERY_STALE_TARGET"
    );
    ensure!(
        records.medium.revision < MAX_REVISION
            && records.run.revision < MAX_REVISION
            && saved
                .children
                .iter()
                .all(|task| task.status != "planned" || task.revision < MAX_REVISION)
            && store::plan_state(connection, &request.project_id)?.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    Ok(())
}

pub(super) fn commit(
    connection: &Connection,
    saved: &Stored,
    report: Report,
    remove_workspace: bool,
) -> Result<Outcome> {
    let records = &saved.verification.records;
    let mut medium = records.medium.clone();
    let mut run = records.run.clone();
    let mut queue = records.queue.clone();
    let mut plan = store::plan_state(connection, &medium.project_id)?;
    medium.status = "cancelled".into();
    medium.revision += 1;
    run.status = "cancelled".into();
    run.revision += 1;
    queue.state = "cancelled".into();
    plan.revision += 1;
    for child in &saved.children {
        if child.status == "planned" {
            let mut child = child.clone();
            child.status = "cancelled".into();
            child.revision += 1;
            store::replace(connection, TASK_KIND, &child.id, &child)?;
        }
    }
    store::replace(connection, TASK_KIND, &medium.id, &medium)?;
    store::replace(connection, RUN_KIND, &run.id, &run)?;
    store::replace(connection, QUEUE_KIND, &queue.id, &queue)?;
    store::replace(connection, STATE_KIND, &medium.project_id, &plan)?;
    let attempt = records
        .attempt
        .as_ref()
        .context("OBJECT_RECOVERY_STOP_UNCONFIRMED")?;
    let output = attempt
        .output
        .as_ref()
        .context("OBJECT_RECOVERY_STOP_UNCONFIRMED")?;
    Ok(if remove_workspace {
        Outcome::CancelledAndWorkspaceRemoved {
            report,
            removed: Retention {
                workspace: records.preparation.workspace.clone(),
                attempt_id: attempt.id.clone(),
                fine_task_id: attempt.fine.id.clone(),
                output_file_count: output.len(),
            },
            task_revision: medium.revision,
            run_revision: run.revision,
            plan_revision: plan.revision,
        }
    } else {
        Outcome::CancelledAndRetained {
            report,
            retained: Retention {
                workspace: records.preparation.workspace.clone(),
                attempt_id: attempt.id.clone(),
                fine_task_id: attempt.fine.id.clone(),
                output_file_count: output.len(),
            },
            task_revision: medium.revision,
            run_revision: run.revision,
            plan_revision: plan.revision,
        }
    })
}

pub(super) fn validate_retained(connection: &Connection, saved: &Stored) -> Result<()> {
    let records = &saved.verification.records;
    let mut expected_medium = records.medium.clone();
    expected_medium.status = "cancelled".into();
    expected_medium.revision += 1;
    let mut expected_run = records.run.clone();
    expected_run.status = "cancelled".into();
    expected_run.revision += 1;
    let mut expected_queue = records.queue.clone();
    expected_queue.state = "cancelled".into();
    let project = &records.medium.project_id;
    let Some(
        Outcome::CancelledAndRetained { plan_revision, .. }
        | Outcome::CancelledAndWorkspaceRemoved { plan_revision, .. },
    ) = &saved.operation.result
    else {
        anyhow::bail!("OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH");
    };
    ensure!(
        store::plan_state(connection, project)?.revision >= *plan_revision,
        "OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH"
    );
    let (medium, run) = claim::read_medium(connection, project, &records.medium.id)?;
    ensure!(
        medium == expected_medium
            && run == expected_run
            && object_task_queue::read_entry(connection, project, &medium.id)?.as_ref()
                == Some(&expected_queue)
            && object_run_preparation::read_record(connection, project, &run.id)?.as_ref()
                == Some(&records.preparation),
        "OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH"
    );
    for child in &saved.children {
        let mut expected = child.clone();
        if expected.status == "planned" {
            expected.status = "cancelled".into();
            expected.revision += 1;
        }
        ensure!(
            store::read::<TaskRecord>(connection, TASK_KIND, &child.id)?.as_ref()
                == Some(&expected),
            "OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH"
        );
    }
    let attempt = records
        .attempt
        .as_ref()
        .context("OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH")?;
    ensure!(
        object_attempt_view::read(connection, project, &attempt.id)? == *attempt,
        "OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH"
    );
    Ok(())
}
