//! Frozen predecessors remain queryable without borrowing the current writer's state.
use super::storage;
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_view::{self, View},
    object_task_storage as store,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

fn successor(connection: &Connection, previous: &Attempt) -> Result<Option<storage::Stored>> {
    let Some(request) = store::read::<String>(connection, storage::HISTORY_KIND, &previous.id)?
    else {
        return Ok(None);
    };
    let saved = storage::read(connection, &previous.preparation.project_id, &request)?
        .context("OBJECT_RECOVERY_HISTORY_MISSING")?;
    ensure!(
        saved.verification.records.attempt.as_ref() == Some(previous) && saved.started.is_some(),
        "OBJECT_RECOVERY_HISTORY_MISMATCH"
    );
    if saved.operation.request.advance.is_some() {
        ensure!(
            store::read::<crate::object_task_types::TaskRecord>(
                connection,
                store::TASK_KIND,
                &previous.fine.id
            )? == Some(super::advance::accepted(previous)?),
            "OBJECT_STAGE_ACCEPTED_RECORD_MISMATCH"
        );
    }
    Ok(Some(saved))
}

pub(crate) fn chain(connection: &Connection, attempts: Vec<Attempt>) -> Result<Vec<Attempt>> {
    if attempts.is_empty() {
        return Ok(attempts);
    }
    let count = attempts.len();
    let mut remaining: BTreeMap<_, _> = attempts
        .into_iter()
        .map(|attempt| (attempt.id.clone(), attempt))
        .collect();
    ensure!(remaining.len() == count, "OBJECT_RECOVERY_HISTORY_MISMATCH");
    let mut edges = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for previous in remaining.values() {
        if let Some(saved) = successor(connection, previous)? {
            let initial = saved.started.as_ref().unwrap();
            let current = remaining
                .get(&initial.id)
                .context("OBJECT_RECOVERY_HISTORY_MISSING")?;
            validate_edge(previous, current, &saved)?;
            ensure!(
                incoming.insert(initial.id.clone()),
                "OBJECT_RECOVERY_HISTORY_MISMATCH"
            );
            edges.insert(previous.id.clone(), initial.id.clone());
        }
    }
    let roots: Vec<_> = remaining
        .keys()
        .filter(|id| !incoming.contains(*id))
        .cloned()
        .collect();
    ensure!(roots.len() == 1, "OBJECT_RECOVERY_HISTORY_MISMATCH");
    let mut id = roots[0].clone();
    let mut ordered = Vec::with_capacity(count);
    loop {
        ordered.push(
            remaining
                .remove(&id)
                .context("OBJECT_RECOVERY_HISTORY_MISMATCH")?,
        );
        let Some(next) = edges.remove(&id) else {
            break;
        };
        id = next;
    }
    ensure!(
        remaining.is_empty() && edges.is_empty(),
        "OBJECT_RECOVERY_HISTORY_MISMATCH"
    );
    Ok(ordered)
}

fn validate_edge(previous: &Attempt, current: &Attempt, saved: &storage::Stored) -> Result<()> {
    let initial = saved.started.as_ref().unwrap();
    ensure!(
        current.preparation == previous.preparation
            && previous.output.as_ref() == Some(&current.input)
            && current.id == initial.id
            && current.fine == initial.fine
            && current.input == initial.input
            && current.preparation == initial.preparation,
        "OBJECT_RECOVERY_HISTORY_MISMATCH"
    );
    if saved.operation.request.advance.is_some() {
        ensure!(
            previous.state == State::AwaitingGate
                && current.fine.id != previous.fine.id
                && saved.next.as_ref() == Some(&current.fine),
            "OBJECT_RECOVERY_HISTORY_MISMATCH"
        );
    } else {
        ensure!(
            (matches!(previous.state, State::Failed | State::Interrupted)
                || (saved.operation.request.rework.is_some()
                    && previous.state == State::AwaitingGate))
                && current.fine.id == previous.fine.id
                && current.fine.revision
                    == previous
                        .fine
                        .revision
                        .checked_add(2)
                        .context("OBJECT_TASK_REVISION_EXHAUSTED")?,
            "OBJECT_RECOVERY_HISTORY_MISMATCH"
        );
    }
    Ok(())
}

pub(crate) fn retained_attempt(connection: &Connection, attempt: &Attempt) -> Result<Option<View>> {
    let Some(saved) = successor(connection, attempt)? else {
        return Ok(None);
    };
    Ok(Some(View {
        project_id: attempt.preparation.project_id.clone(),
        target: object_attempt_view::Target::from_record(attempt),
        task_revision: saved.verification.records.medium.revision,
        state: attempt.state.clone(),
        output_captured: attempt.output.is_some(),
        error: attempt.error.clone(),
    }))
}
