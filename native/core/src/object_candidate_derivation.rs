//! Copy frozen review evidence, not present-day authority. No review can launch execution.
use super::{checked, checks, summary, ObjectRecord, Stored, Target, KIND};
use crate::{
    object_attempt::{Attempt, State},
    object_run_recovery::resume::derivation::rework::Prompts,
    object_run_recovery::{derivation, records},
    object_task_types::{valid_id, Granularity, TaskRecord},
    project_derivation_copy::Request,
    project_derivation_execution_records as execution,
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_object_records::rewrite_snapshot,
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_plan_validation::records as entities_of,
    project_derivation_validation_records::Rewrite,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn validate(
    connection: &Connection,
    entities: &[Entity],
    project: &str,
    stopped: &BTreeMap<String, Attempt>,
) -> Result<()> {
    // Iterate every entity, including orphan requests which attempt-scoped readers skip.
    for (id, saved) in entities_of::<Stored>(entities, KIND)? {
        let request = &saved.report.request;
        ensure!(
            id == request.request_id
                && valid_id(&id)
                && valid_id(&request.check_request_id)
                && request.project_id == project,
            "DERIVATION_CANDIDATE_IDENTITY_MISMATCH"
        );
        let attempt = stopped
            .get(&request.target.attempt_id)
            .context("DERIVATION_CANDIDATE_ORPHAN_REVIEW")?;
        ensure!(
            request.target == Target::from_record(attempt) && attempt.state == State::AwaitingGate,
            "DERIVATION_CANDIDATE_TERMINAL_GATE_REQUIRED"
        );
        let current = records::read(connection, project, &request.target.task_id)?
            .context("DERIVATION_CANDIDATE_RECORDS_MISSING")?;
        let mut chain = current.history.clone();
        chain.push(
            current
                .attempt
                .clone()
                .context("DERIVATION_CANDIDATE_RECORDS_MISSING")?,
        );
        derivation::snapshot::validate(
            connection,
            entities,
            &current,
            &chain,
            &saved.source.records,
        )?;
        let fine = saved
            .source
            .records
            .fine
            .as_ref()
            .context("DERIVATION_CANDIDATE_FINE_MISSING")?;
        let mut fines: Vec<_> = entities_of::<TaskRecord>(entities, "object_task")?
            .into_values()
            .filter(|task| {
                task.parent_task_id.as_deref() == Some(&current.medium.id)
                    && task.granularity == Granularity::Fine
                    && task.status != "cancelled"
            })
            .collect();
        fines.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
        ensure!(
            fines.last().is_some_and(|last| last.id == fine.id)
                && fines[..fines.len() - 1]
                    .iter()
                    .all(|task| task.status == "accepted"),
            "DERIVATION_CANDIDATE_FINAL_FINE_HISTORY_MISMATCH"
        );
        // The final fine may have been reworked later. Its historical revision and
        // prompt were just proven against the receipt-backed chain, not inferred.
        *fines.last_mut().unwrap() = fine.clone();
        ensure!(
            saved.source.fines == fines,
            "DERIVATION_CANDIDATE_FINAL_FINE_HISTORY_MISMATCH"
        );
        let objects = entities_of::<ObjectRecord>(entities, "object")?;
        // No catalog-membership journal exists. Reject changed membership instead of
        // silently inventing owners or omitting an object from a historical review.
        ensure!(
            saved
                .source
                .objects
                .iter()
                .map(|o| &o.id)
                .eq(objects.keys()),
            "DERIVATION_CANDIDATE_CATALOG_MEMBERSHIP_MISMATCH"
        );
        for object in &saved.source.objects {
            derivation::snapshot::object(connection, entities, object)?;
        }
        for object in std::iter::once(&saved.source.records.object)
            .chain(&saved.source.records.baseline_sources)
        {
            ensure!(
                saved.source.objects.iter().find(|o| o.id == object.id) == Some(object),
                "DERIVATION_CANDIDATE_OBJECT_SNAPSHOT_MISMATCH"
            );
        }
        checks::require_passed(connection, attempt, &request.check_request_id)?;
        ensure!(
            checks::valid_rules(&saved.report.rules),
            "OBJECT_CANDIDATE_RULES_MISMATCH"
        );
        checked(saved)?;
    }
    Ok(())
}

pub(crate) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let mut saved: Stored = serde_json::from_value(value.clone())?;
    checked(saved.clone())?;
    let ids = PlanIds(request);
    let command = &mut saved.report.request;
    command.project_id = request.target_project_id.clone();
    ids.id(&mut command.request_id, "object_candidate_request")?;
    ids.id(&mut command.check_request_id, "object_check_request")?;
    execution::target(request, &mut command.target)?;
    derivation::conversion_records::rewrite(map, request, prompts, &mut saved.source.records)?;
    for fine in &mut saved.source.fines {
        prompts.task(fine);
        ids.task(fine)?;
    }
    saved
        .source
        .fines
        .sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
    for object in &mut saved.source.objects {
        rewrite_snapshot(map, request, object)?;
    }
    saved.source.objects.sort_by(|a, b| a.id.cmp(&b.id));
    // Rebuild identity-derived digests, references and ownership using the ORIGINAL
    // snapshot, rules, timestamp and schema. Never recheck or read current Records.
    saved.report = summary::build(
        &saved.source,
        &saved.report.request,
        saved.report.rules,
        saved.report.time,
        saved.report.schema_version,
    )?;
    checked(saved.clone())?;
    Ok((Rewrite(map).key(KIND, id)?, serde_json::to_value(saved)?))
}
