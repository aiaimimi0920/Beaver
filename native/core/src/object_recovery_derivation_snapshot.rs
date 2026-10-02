//! Anchor historical verifier inputs to a stopped chain prefix and immutable command evidence.
use super::super::records::Records;
use crate::{
    object_attempt::{Attempt, State},
    object_catalog::ObjectRecord,
    object_command_receipt::Receipt,
    object_task_dispatch as dispatch,
    project_derivation_plan_validation::records,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::object_run_recovery) fn object(
    connection: &Connection,
    entities: &[Entity],
    value: &ObjectRecord,
) -> Result<()> {
    let objects = records::<ObjectRecord>(entities, "object")?;
    crate::project_derivation_object_validation::snapshot(&objects, value)?;
    let current = objects
        .get(&value.id)
        .context("DERIVATION_RECOVERY_OBJECT_MISSING")?;
    if value.revision != current.revision {
        let projections: Vec<_> = records::<Receipt>(entities, "object_command_receipt")?
            .into_values()
            .filter(|r| {
                r.result.object.id == value.id && r.result.object.revision == value.revision
            })
            .collect();
        ensure!(
            projections.len() == 1 && projections[0].result.object == *value,
            "DERIVATION_RECOVERY_OBJECT_HISTORY_MISMATCH"
        );
    }
    // Exercise acceptance ordering at this exact revision, not today's accepted pointer.
    crate::object_command_receipt::accepted_at_revision(connection, current, value.revision)?;
    Ok(())
}

pub(in crate::object_run_recovery) fn validate(
    connection: &Connection,
    entities: &[Entity],
    current: &Records,
    chain: &[Attempt],
    saved: &Records,
) -> Result<usize> {
    let index = chain
        .iter()
        .position(|a| saved.attempt.as_ref() == Some(a))
        .context("DERIVATION_RECOVERY_PREFIX_MISMATCH")?;
    ensure!(
        saved.history == chain[..index] && saved.preparation == current.preparation,
        "DERIVATION_RECOVERY_PREFIX_MISMATCH"
    );
    let delta = u64::try_from(index)?
        .checked_add(1)
        .and_then(|n| n.checked_mul(2))
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    let mut medium = current.preparation.medium.clone();
    let mut run = current.preparation.run.clone();
    let mut fine = chain[index].fine.clone();
    let status = if chain[index].state == State::AwaitingGate {
        "awaitingAcceptance"
    } else {
        "failed"
    };
    medium.status = status.into();
    run.status = status.into();
    fine.status = status.into();
    medium.revision = medium
        .revision
        .checked_add(delta)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    run.revision = run
        .revision
        .checked_add(delta)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    fine.revision = fine
        .revision
        .checked_add(2)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    ensure!(
        saved.medium == medium && saved.run == run && saved.fine.as_ref() == Some(&fine),
        "DERIVATION_RECOVERY_TASK_SNAPSHOT_MISMATCH"
    );
    let mut queue = current.queue.clone();
    queue.state = status.into();
    // Derivation canonicalizes current queue order. Historical positions remain evidence,
    // never execution authority; all immutable claim/enqueue fields must still be exact.
    queue.position = saved.queue.position;
    ensure!(
        saved.queue == queue,
        "DERIVATION_RECOVERY_QUEUE_SNAPSHOT_MISMATCH"
    );
    let mut control = current.control.clone();
    control.revision = 0;
    control.paused = false;
    if saved.control.revision > 0 {
        let commands: Vec<_> =
            records::<dispatch::Receipt>(entities, "object_task_dispatch_receipt")?
                .into_values()
                .filter(|r| {
                    r.result.run_id == run.id && r.result.revision == saved.control.revision
                })
                .collect();
        ensure!(
            commands.len() == 1 && commands[0].request.expected_task_revision <= medium.revision,
            "DERIVATION_RECOVERY_CONTROL_HISTORY_MISMATCH"
        );
        control = commands[0].result.clone();
    }
    ensure!(
        saved.control == control && saved.control.revision <= current.control.revision,
        "DERIVATION_RECOVERY_CONTROL_HISTORY_MISMATCH"
    );
    ensure!(
        saved.object.id == current.object.id,
        "DERIVATION_RECOVERY_OBJECT_MISMATCH"
    );
    object(connection, entities, &saved.object)?;
    ensure!(
        saved.accepted_version_id
            == crate::object_command_receipt::accepted_at_revision(
                connection,
                &current.object,
                saved.object.revision
            )?,
        "DERIVATION_RECOVERY_ACCEPTANCE_MISMATCH"
    );
    let ids: BTreeSet<_> = current
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_RUN_BASELINE_MISSING")?
        .versions
        .iter()
        .map(|v| v.object_id.as_str())
        .collect();
    ensure!(
        saved
            .baseline_sources
            .iter()
            .map(|o| o.id.as_str())
            .collect::<Vec<_>>()
            == ids.into_iter().collect::<Vec<_>>(),
        "DERIVATION_RECOVERY_BASELINE_SOURCES_MISMATCH"
    );
    for source in &saved.baseline_sources {
        object(connection, entities, source)?;
    }
    let interrupts: BTreeMap<_, _> =
        crate::object_attempt_control::for_recovery(connection, &chain[index])?
            .into_iter()
            .map(|r| (r.request.request_id.clone(), r))
            .collect();
    let mut previous = None;
    for receipt in &saved.interrupts {
        let id = &receipt.request.request_id;
        ensure!(
            previous.is_none_or(|p: &String| p < id) && interrupts.get(id) == Some(receipt),
            "DERIVATION_RECOVERY_INTERRUPT_SNAPSHOT_MISMATCH"
        );
        previous = Some(id);
    }
    // A completed interrupt may be recorded after this verifier snapshot. Validate
    // retained entries exactly; do not invent a timestamp or require today's set.
    Ok(index)
}
