//! Read-only stopped retry and stage chains; never constructs a lease.
use super::{files, records};
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_view,
    object_framework::Baseline,
    object_run_preparation::PreparationState,
    object_task_types::Granularity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

#[path = "object_recovery_derivation_records.rs"]
pub(super) mod conversion_records;
#[path = "object_recovery_derivation_snapshot.rs"]
pub(in crate::object_run_recovery) mod snapshot;

pub(crate) fn read(connection: &Connection, project: &str, task: &str) -> Result<Vec<Attempt>> {
    let records = records::read(connection, project, task)?
        .context("DERIVATION_EXECUTION_PREPARATION_MISSING")?;
    let mut chain = records.history.clone();
    chain.push(
        records
            .attempt
            .clone()
            .context("DERIVATION_EXECUTION_ATTEMPT_REQUIRED")?,
    );
    let root = &chain[0];
    ensure!(
        records.preparation.state == PreparationState::Ready
            && records.preparation.error.is_none()
            && records.preparation.generation == 1
            && records.preparation.medium.status == "queued"
            && records.preparation.run.status == "queued"
            && records.preparation.medium.revision > 0
            && records.preparation.run.revision == 1
            && root.fine.status == "planned"
            // records::read validates every successor through the real retry/advance/rework
            // receipt. A same-fine Gate edge is valid only with that completed rework proof.
            && chain.iter().all(|attempt| {
                matches!(attempt.state, State::Failed | State::Interrupted | State::AwaitingGate)
                    && attempt.output.is_some()
                    && attempt.preparation == records.preparation
            }),
        "DERIVATION_EXECUTION_FIRST_STOPPED_ATTEMPT_REQUIRED"
    );
    ensure!(
        records.preparation.workspace == format!(".beaver/workspaces/{}", records.run.id),
        "OBJECT_RUN_WORKSPACE_MISMATCH"
    );
    let input = files::baseline_input(project, &records)?;
    let baseline = records
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_RUN_BASELINE_MISSING")?;
    let accepted = crate::object_command_receipt::accepted_at_revision(
        connection,
        &records.object,
        baseline.object_revision_at_claim,
    )?;
    ensure!(
        baseline.accepted_version_id_at_claim == accepted
            && (!matches!(baseline.policy, Baseline::LatestAccepted {})
                || baseline.resolved_version_id == accepted),
        "OBJECT_BASELINE_CLAIM_ACCEPTANCE_MISMATCH"
    );
    let tasks = super::resume::derivation::stages::initial_tasks(connection, &chain)?;
    // Identity remapping cannot preserve lexicographic tie-breaking. Reject that
    // ambiguous historical boundary instead of silently changing task positions.
    ensure!(
        tasks
            .iter()
            .filter(
                |task| task.parent_task_id.as_deref() == Some(&records.medium.id)
                    && task.granularity == Granularity::Fine
                    && task.status != "cancelled"
                    && task.position == root.fine.position
            )
            .count()
            == 1,
        "DERIVATION_EXECUTION_FINE_POSITION_AMBIGUOUS"
    );
    ensure!(
        crate::object_attempt::select_fine(connection, &records.preparation, &tasks)?.as_ref()
            == Some(&root.fine),
        "DERIVATION_EXECUTION_FIRST_FINE_MISMATCH"
    );
    ensure!(input == root.input, "OBJECT_RECOVERY_INPUT_MISMATCH");
    for attempt in &chain {
        files::valid_snapshot(&attempt.input)?;
        files::valid_snapshot(attempt.output.as_ref().unwrap())?;
        let view = object_attempt_view::view(connection, attempt)?;
        for interrupt in crate::object_attempt_control::for_recovery(connection, attempt)? {
            ensure!(
                interrupt.result.as_ref() == Some(&view),
                "DERIVATION_EXECUTION_INTERRUPT_INCOMPLETE"
            );
        }
    }
    Ok(chain)
}
