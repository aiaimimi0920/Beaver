//! Validate every execution entity, including orphans, before any preparation directory exists.
use crate::{
    object_attempt::Attempt, object_attempt_checks as checks, object_attempt_control as control,
    object_attempt_trace as trace, object_attempt_view::Target,
    object_run_preparation::Preparation, project_derivation_plan_validation::records,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(crate) fn supports(kind: &str) -> bool {
    crate::object_run_recovery::resume::derivation::supports(kind)
        || matches!(
            kind,
            "object_run_preparation"
                | "object_attempt"
                | "object_attempt_trace"
                | "object_attempt_check_report"
                | "object_attempt_interrupt_receipt"
                | "object_candidate_review"
        )
}

pub(crate) fn validate(connection: &Connection, entities: &[Entity], project: &str) -> Result<()> {
    let preparations = records::<Preparation>(entities, "object_run_preparation")?;
    let mut attempts = records::<Attempt>(entities, "object_attempt")?;
    let mut stopped = BTreeMap::new();
    for (id, prepared) in preparations {
        ensure!(
            id == prepared.id && id == prepared.run.id && prepared.project_id == project,
            "DERIVATION_EXECUTION_PREPARATION_IDENTITY_MISMATCH"
        );
        let chain =
            crate::object_run_recovery::derivation::read(connection, project, &prepared.medium.id)?;
        for attempt in chain {
            ensure!(
                attempt.preparation == prepared
                    && attempts.remove(&attempt.id).as_ref() == Some(&attempt),
                "DERIVATION_EXECUTION_ATTEMPT_MISMATCH"
            );
            stopped.insert(attempt.id.clone(), attempt);
        }
    }
    ensure!(attempts.is_empty(), "DERIVATION_EXECUTION_ORPHAN_ATTEMPT");
    crate::object_run_recovery::resume::derivation::validate(
        connection, entities, project, &stopped,
    )?;
    for (id, report) in records::<checks::Report>(entities, "object_attempt_check_report")? {
        let attempt = stopped
            .get(&report.request.target.attempt_id)
            .context("DERIVATION_EXECUTION_ORPHAN_CHECK")?;
        ensure!(
            id == report.request.request_id && crate::object_task_types::valid_id(&id),
            "OBJECT_CHECK_REPORT_MISMATCH"
        );
        checks::verify_report(&report, attempt)?;
    }
    for (id, receipt) in records::<control::Pending>(entities, control::KIND)? {
        let attempt = stopped
            .get(&receipt.request.target.attempt_id)
            .context("DERIVATION_EXECUTION_ORPHAN_INTERRUPT")?;
        let view = crate::object_attempt_view::view(connection, attempt)?;
        ensure!(
            id == receipt.request.request_id
                && crate::object_task_types::valid_id(&id)
                && receipt.request.project_id == project
                && receipt.request.target == Target::from_record(attempt)
                && receipt.request.expected_task_revision <= view.task_revision
                && receipt.result == Some(view),
            "DERIVATION_EXECUTION_INTERRUPT_INCOMPLETE"
        );
        control::validate_result(&receipt)?;
    }
    crate::object_run_recovery::candidate::derivation::validate(
        connection, entities, project, &stopped,
    )?;
    for (id, value) in records::<trace::Trace>(entities, "object_attempt_trace")? {
        let attempt = stopped
            .get(&id)
            .context("DERIVATION_EXECUTION_ORPHAN_TRACE")?;
        trace::validate_trace(
            &value,
            &trace::Request {
                project_id: project.into(),
                run_id: attempt.preparation.run.id.clone(),
                attempt_id: id,
            },
        )?;
    }
    Ok(())
}

/// Used only after typed execution validation. These are historical planning projections;
/// durable failed records and their revisions are never reset.
pub(crate) fn project_plans(
    entities: &[Entity],
    plans: &mut crate::project_derivation_plan_validation::Plans,
) -> Result<()> {
    for (_, attempt) in records::<Attempt>(entities, "object_attempt")?
        .into_iter()
        .filter(|(_, a)| a.fine.status == "planned")
    {
        let mut medium = attempt.preparation.medium;
        let mut run = attempt.preparation.run;
        medium.status = "planned".into();
        medium.revision = medium
            .revision
            .checked_sub(1)
            .context("DERIVATION_EXECUTION_CLAIM_REVISION")?;
        run.status = "planned".into();
        run.revision = run
            .revision
            .checked_sub(1)
            .context("DERIVATION_EXECUTION_CLAIM_REVISION")?;
        run.baseline_version_id = None;
        plans.tasks.insert(medium.id.clone(), medium);
        plans.runs.insert(run.id.clone(), run);
        plans.tasks.insert(attempt.fine.id.clone(), attempt.fine);
    }
    Ok(())
}
