//! Reconstruct stage selection at a receipt's frozen boundary, never from later accepted tasks.
use super::super::{advance, storage};
use crate::{
    object_attempt::Attempt,
    object_task_storage,
    object_task_types::{Granularity, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::BTreeSet;

pub(in crate::object_run_recovery) fn initial_tasks(
    connection: &Connection,
    chain: &[Attempt],
) -> Result<Vec<TaskRecord>> {
    let mut tasks = object_task_storage::all_tasks(connection)?;
    let mut seen = BTreeSet::new();
    for attempt in chain {
        if seen.insert(&attempt.fine.id) {
            ensure!(
                attempt.fine.status == "planned",
                "DERIVATION_STAGE_INITIAL_FINE_MISMATCH"
            );
            let task = tasks
                .iter_mut()
                .find(|t| t.id == attempt.fine.id)
                .context("OBJECT_ATTEMPT_IDENTITY_MISMATCH")?;
            *task = attempt.fine.clone();
        }
    }
    // A new identity can change the id tie-break. Preserve positions, reject ambiguity.
    let mut positions = BTreeSet::new();
    for task in tasks.iter().filter(|t| {
        t.parent_task_id.as_deref() == Some(&chain[0].preparation.medium.id)
            && t.granularity == Granularity::Fine
            && t.status != "cancelled"
    }) {
        ensure!(
            positions.insert(task.position),
            "DERIVATION_EXECUTION_FINE_POSITION_AMBIGUOUS"
        );
    }
    Ok(tasks)
}

pub(super) fn validate(
    connection: &Connection,
    chain: &[Attempt],
    index: usize,
    saved: &storage::Stored,
) -> Result<()> {
    if saved.operation.request.advance.is_none() {
        return Ok(()); // storage::read has required absent next/fresh_checks for a plain retry.
    }
    let mut tasks = initial_tasks(connection, chain)?;
    for pair in chain[..=index].windows(2) {
        if pair[0].fine.id != pair[1].fine.id {
            let task = tasks.iter_mut().find(|t| t.id == pair[0].fine.id).unwrap();
            *task = advance::accepted(&pair[0])?;
        }
    }
    let current = saved
        .verification
        .records
        .fine
        .as_ref()
        .context("DERIVATION_STAGE_CURRENT_FINE_MISSING")?;
    *tasks
        .iter_mut()
        .find(|t| t.id == current.id)
        .context("DERIVATION_STAGE_CURRENT_FINE_MISSING")? = current.clone();
    let selected = advance::select_from_tasks(
        connection,
        &saved.verification.records,
        &saved.operation.request,
        &tasks,
    )?;
    ensure!(
        saved.next.as_ref() == Some(&selected),
        "DERIVATION_STAGE_SUCCESSOR_SNAPSHOT_MISMATCH"
    );
    Ok(())
}
