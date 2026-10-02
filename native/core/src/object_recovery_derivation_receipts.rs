//! Validate completed retry/advance/text-rework receipts; never reconstruct a writer lease.
use super::{records, storage, verification, Outcome};
use crate::{
    object_attempt::Attempt, project_derivation_plan_validation::records as entities_of,
    project_derivation_queue_records::receipt_key, project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "object_recovery_derivation_rewrite.rs"]
mod conversion;
#[path = "object_recovery_derivation_rework.rs"]
pub(crate) mod rework;
#[path = "object_recovery_derivation_stages.rs"]
pub(in crate::object_run_recovery) mod stages;
pub(crate) use conversion::{rewrite, target_key};

pub(crate) fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "object_recovery_verification"
            | "object_recovery_head"
            | "object_recovery_resume"
            | "object_recovery_resume_head"
            | "object_recovery_attempt_successor"
    )
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Head {
    project_id: String,
    task_id: String,
    run_id: String,
    request_id: String,
    generation: u64,
}

pub(crate) fn validate(
    connection: &Connection,
    entities: &[Entity],
    project: &str,
    attempts: &BTreeMap<String, Attempt>,
) -> Result<()> {
    let mut chains = BTreeMap::new();
    for attempt in attempts.values() {
        chains
            .entry(attempt.preparation.run.id.clone())
            .or_insert_with(Vec::new)
            .push(attempt.clone());
    }
    let mut contexts = BTreeMap::new();
    for (run, chain) in &mut chains {
        *chain = super::chain(connection, std::mem::take(chain))?;
        let current = records::read(connection, project, &chain[0].preparation.medium.id)?
            .context("DERIVATION_RECOVERY_RECORDS_MISSING")?;
        contexts.insert(run.clone(), current);
    }
    let mut generations = BTreeMap::<String, BTreeMap<u64, (String, usize, u64, u64)>>::new();
    for (key, saved) in
        entities_of::<verification::Stored>(entities, "object_recovery_verification")?
    {
        let op = &saved.operation;
        ensure!(
            key == receipt_key(project, &op.request.request_id)?
                && verification::read(connection, project, &op.request.request_id)?.as_ref()
                    == Some(&saved)
                && op.result.is_some(),
            "DERIVATION_RECOVERY_VERIFICATION_INCOMPLETE"
        );
        let run = &op.request.target.run_id;
        let current = contexts
            .get(run)
            .context("DERIVATION_RECOVERY_ORPHAN_VERIFICATION")?;
        let index = super::super::derivation::snapshot::validate(
            connection,
            entities,
            current,
            &chains[run],
            &saved.records,
        )?;
        ensure!(
            generations
                .entry(run.clone())
                .or_default()
                .insert(
                    op.generation,
                    (
                        op.request.request_id.clone(),
                        index,
                        saved.records.control.revision,
                        saved.records.object.revision
                    )
                )
                .is_none(),
            "DERIVATION_RECOVERY_DUPLICATE_GENERATION"
        );
    }
    let mut heads = entities_of::<Head>(entities, "object_recovery_head")?;
    for (run, history) in &generations {
        let head = heads
            .remove(run)
            .context("DERIVATION_RECOVERY_HEAD_MISSING")?;
        let last = history.last_key_value().unwrap();
        let progression: Vec<_> = history.values().map(|v| (v.1, v.2, v.3)).collect();
        ensure!(
            head.project_id == project
                && head.run_id == *run
                && head.task_id == contexts[run].medium.id
                && head.generation == *last.0
                && head.request_id == last.1 .0
                && history.keys().enumerate().all(|(i, g)| *g == i as u64 + 1)
                && progression
                    .windows(2)
                    .all(|p| p[0].0 <= p[1].0 && p[0].1 <= p[1].1 && p[0].2 <= p[1].2),
            "DERIVATION_RECOVERY_HEAD_MISMATCH"
        );
        verification::latest(connection, project, &head.task_id, run)?;
    }
    ensure!(heads.is_empty(), "DERIVATION_RECOVERY_ORPHAN_HEAD");
    let mut edges = entities_of::<String>(entities, storage::HISTORY_KIND)?;
    let mut resume_heads = entities_of::<String>(entities, "object_recovery_resume_head")?;
    let mut latest = BTreeMap::<String, (u64, String)>::new();
    let mut used = BTreeSet::new();
    let mut resume_generations = BTreeSet::new();
    for (key, saved) in entities_of::<storage::Stored>(entities, "object_recovery_resume")? {
        let op = &saved.operation;
        if let Some(approval) = &op.request.rework {
            rework::text_only(approval)?;
        }
        ensure!(
            key == receipt_key(project, &op.request.request_id)?
                && storage::read(connection, project, &op.request.request_id)?.as_ref()
                    == Some(&saved)
                && op.result.is_some(),
            "DERIVATION_RECOVERY_RESUME_INCOMPLETE"
        );
        let chain = chains
            .get(&op.request.target.run_id)
            .context("DERIVATION_RECOVERY_ORPHAN_RESUME")?;
        let previous = saved
            .verification
            .records
            .attempt
            .as_ref()
            .context("DERIVATION_RECOVERY_PREFIX_MISMATCH")?;
        let index = chain
            .iter()
            .position(|a| a == previous)
            .context("DERIVATION_RECOVERY_PREFIX_MISMATCH")?;
        stages::validate(connection, chain, index, &saved)?;
        if let Some(Outcome::Started { attempt_id, .. }) = &op.result {
            ensure!(
                chain.get(index + 1).is_some_and(|a| a.id == *attempt_id)
                    && edges.remove(&previous.id).as_ref() == Some(&op.request.request_id)
                    && used.insert(previous.id.clone()),
                "DERIVATION_RECOVERY_SUCCESSOR_MISMATCH"
            );
        }
        let candidate = (
            op.request.target.recovery_generation,
            op.request.request_id.clone(),
        );
        ensure!(
            resume_generations.insert((op.request.target.task_id.clone(), candidate.0)),
            "DERIVATION_RECOVERY_DUPLICATE_RESUME_GENERATION"
        );
        let entry = latest
            .entry(op.request.target.task_id.clone())
            .or_insert_with(|| candidate.clone());
        if candidate.0 > entry.0 {
            *entry = candidate;
        }
    }
    for (task, (_, request)) in latest {
        ensure!(
            resume_heads.remove(&task).as_ref() == Some(&request),
            "DERIVATION_RECOVERY_RESUME_HEAD_MISMATCH"
        );
        storage::latest(connection, project, &task)?;
    }
    ensure!(
        edges.is_empty() && resume_heads.is_empty(),
        "DERIVATION_RECOVERY_ORPHAN_SUCCESSOR_OR_HEAD"
    );
    Ok(())
}
