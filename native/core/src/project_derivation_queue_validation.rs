//! Derive untouched queues or fully validated stopped owners, without restoring writer leases.
use crate::{
    object_task_dispatch,
    object_task_queue::{self as queue, QueueEntry},
    object_task_queue_reorder as reorder, object_task_queue_view,
    object_task_types::valid_id,
    project_derivation_plan_validation::{records, Plans},
    project_derivation_queue_records::receipt_key,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

fn token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn receipt(
    plans: &Plans,
    entries: &BTreeMap<String, QueueEntry>,
    key: &str,
    receipt: &reorder::Receipt,
) -> Result<()> {
    let request = &receipt.request;
    let result = &receipt.result;
    ensure!(
        [&request.project_id, &request.request_id, &request.task_id]
            .into_iter()
            .chain(request.previous_task_id.iter())
            .chain(request.next_task_id.iter())
            .all(|s| valid_id(s))
            && request.project_id == plans.project
            && result.project_id == plans.project
            && key == receipt_key(&plans.project, &request.request_id)?
            && token(&request.expected_version)
            && token(&result.version)
            && result.version != request.expected_version,
        "DERIVATION_QUEUE_RECEIPT_MISMATCH"
    );
    let mut seen = BTreeSet::new();
    let mut heads = BTreeSet::new();
    for item in &result.items {
        let task = plans.task(&item.task_id)?;
        let entry = entries
            .get(&format!("{}:{}", plans.project, task.id))
            .context("DERIVATION_QUEUE_HISTORY_ENTRY_MISSING")?;
        ensure!(
            seen.insert(&item.task_id)
                && task.object_id.as_ref() == Some(&item.object_id)
                && !item.title.trim().is_empty()
                && matches!(item.state.as_str(), "queued" | "cancelled")
                && (item.state != "cancelled" || entry.state == "cancelled"),
            "DERIVATION_QUEUE_HISTORY_IDENTITY_MISMATCH"
        );
        let blockers: BTreeSet<_> = item.blockers.iter().map(String::as_str).collect();
        ensure!(
            blockers.len() == item.blockers.len()
                && blockers.iter().all(|b| matches!(
                    *b,
                    "earlierQueued" | "paused" | "coarsePaused" | "dependencies"
                ))
                && item.blockers.windows(2).all(|pair| {
                    let rank = |value: &str| {
                        ["earlierQueued", "paused", "coarsePaused", "dependencies"]
                            .iter()
                            .position(|b| *b == value)
                    };
                    rank(&pair[0]) < rank(&pair[1])
                })
                && if item.state == "queued" {
                    blockers.contains("earlierQueued") != heads.insert(&item.object_id)
                } else {
                    blockers.is_empty()
                },
            "DERIVATION_QUEUE_HISTORY_BLOCKERS_MISMATCH"
        );
    }
    let ids: Vec<_> = result
        .items
        .iter()
        .filter(|i| i.state == "queued")
        .map(|i| i.task_id.clone())
        .collect();
    ensure!(
        reorder::move_ids(result, request)? == ids,
        "DERIVATION_QUEUE_HISTORY_ANCHOR_MISMATCH"
    );
    Ok(())
}

pub(crate) fn validate(connection: &Connection, entities: &[Entity], project: &str) -> Result<()> {
    let plans = Plans::current_entities(entities, project)?;
    crate::project_derivation_dispatch_validation::validate(entities, &plans)?;
    let entries = records::<QueueEntry>(entities, "object_task_queue")?;
    for (key, entry) in &entries {
        ensure!(
            entry.project_id == project
                && valid_id(&entry.task_id)
                && *key == entry.id
                && entry.id == format!("{project}:{}", entry.task_id),
            "DERIVATION_QUEUE_IDENTITY_MISMATCH"
        );
        if matches!(entry.state.as_str(), "failed" | "awaitingAcceptance") {
            // A stopped owner requires the complete same-fine chain, optionally ending at a gate.
            crate::object_run_recovery::derivation::read(connection, project, &entry.task_id)?;
        } else {
            ensure!(
                matches!(entry.state.as_str(), "queued" | "cancelled")
                    && entry.claim_token.is_none()
                    && entry.owner.is_none()
                    && entry.generation == 0,
                "DERIVATION_QUEUE_EXECUTED_ENTRY"
            );
        }
        let (task, run) = queue::claim::read_medium(connection, project, &entry.task_id)?;
        queue::claim::validate_state(entry, &task, &run)?;
        let control = object_task_dispatch::read_in(connection, &task, &run)?;
        ensure!(
            !matches!(
                entry.state.as_str(),
                "queued" | "failed" | "awaitingAcceptance"
            ) || control.paused
                || control.revision < object_task_dispatch::MAX_CONTROL_REVISION,
            "DERIVATION_QUEUE_SAFETY_PAUSE_EXHAUSTED"
        );
    }
    let orders = records::<u64>(entities, object_task_queue_view::ORDER_KIND)?;
    ensure!(
        orders.len() <= 1
            && orders.keys().all(|key| key == project)
            && orders.values().all(|revision| *revision > 0),
        "DERIVATION_QUEUE_ORDER_MISMATCH"
    );
    let receipts = records::<reorder::Receipt>(entities, "object_task_queue_reorder_receipt")?;
    ensure!(
        orders.get(project).copied().unwrap_or(0) == receipts.len() as u64,
        "DERIVATION_QUEUE_ORDER_HISTORY_GAP"
    );
    for (key, value) in &receipts {
        receipt(&plans, &entries, key, value)?;
    }
    // Exercise the normal consumer too; validates current state, controls and eligibility together.
    object_task_queue_view::read_in(connection, project)?;
    Ok(())
}
