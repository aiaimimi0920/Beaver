//! Identity-only conversion of stopped execution evidence; never grant or reconstruct a lease.
use crate::{
    object_attempt::Attempt,
    object_attempt_checks as checks, object_attempt_control as control,
    object_attempt_trace as trace,
    object_attempt_view::Target,
    object_run_preparation::Preparation,
    object_run_recovery::resume::derivation::rework::Prompts,
    project_derivation_copy::Request,
    project_derivation_identity::{generated, IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use serde_json::Value;

pub(crate) fn target_key(kind: &str, id: &str, value: &Value, request: &Request) -> Result<String> {
    if crate::object_run_recovery::resume::derivation::supports(kind) {
        return crate::object_run_recovery::resume::derivation::target_key(
            kind, id, value, request,
        );
    }
    generated(
        request,
        match kind {
            "object_run_preparation" => "object_run",
            "object_attempt" | "object_attempt_trace" => "object_attempt",
            "object_attempt_check_report" => "object_check_request",
            "object_attempt_interrupt_receipt" => "object_interrupt_request",
            "object_candidate_review" => "object_candidate_request",
            _ => bail!("unsupported execution identity: {kind}"),
        },
        id,
    )
}

pub(crate) fn preparation(
    map: &IdentityMap,
    request: &Request,
    prepared: &mut Preparation,
) -> Result<()> {
    let ids = PlanIds(request);
    prepared.project_id = request.target_project_id.clone();
    ids.id(&mut prepared.id, "object_run")?;
    ids.id(&mut prepared.owner, "object_writer_owner")?;
    ids.id(&mut prepared.claim_token, "object_writer_claim")?;
    ids.task(&mut prepared.medium)?;
    ids.run(&mut prepared.run)?;
    prepared.workspace = format!(".beaver/workspaces/{}", prepared.run.id);
    let frozen = prepared
        .baseline
        .as_mut()
        .context("OBJECT_RUN_BASELINE_MISSING")?;
    ids.baseline(&mut frozen.policy)?;
    ids.optional(&mut frozen.resolved_version_id, "object_version")?;
    ids.optional(&mut frozen.accepted_version_id_at_claim, "object_version")?;
    for version in &mut frozen.versions {
        crate::project_derivation_object_records::rewrite_manifest(map, request, version)?;
    }
    // Selection canonicalizes by object/version identity, which changes in a copy.
    frozen
        .versions
        .sort_by(|a, b| (&a.object_id, &a.version_id).cmp(&(&b.object_id, &b.version_id)));
    frozen.content_digest = crate::object_import_snapshot::digest(&frozen.versions)?;
    Ok(())
}

pub(crate) fn attempt(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    record: &mut Attempt,
) -> Result<()> {
    PlanIds(request).id(&mut record.id, "object_attempt")?;
    preparation(map, request, &mut record.preparation)?;
    prompts.task(&mut record.fine);
    PlanIds(request).task(&mut record.fine)
}

pub(crate) fn target(request: &Request, value: &mut Target) -> Result<()> {
    let ids = PlanIds(request);
    ids.id(&mut value.task_id, "object_task")?;
    ids.id(&mut value.fine_task_id, "object_task")?;
    ids.id(&mut value.object_id, "object")?;
    ids.id(&mut value.run_id, "object_run")?;
    ids.id(&mut value.attempt_id, "object_attempt")?;
    ids.id(&mut value.owner, "object_writer_owner")?;
    ids.id(&mut value.claim_token, "object_writer_claim")
}

pub(crate) fn rewrite(
    db: &Connection,
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    if crate::object_run_recovery::resume::derivation::supports(kind) {
        return crate::object_run_recovery::resume::derivation::rewrite(
            map, request, prompts, kind, id, value,
        );
    }
    if kind == "object_candidate_review" {
        return crate::object_run_recovery::candidate::derivation::rewrite(
            map, request, prompts, id, value,
        );
    }
    let key = Rewrite(map).key(kind, id)?;
    let ids = PlanIds(request);
    let value = match kind {
        "object_run_preparation" => {
            let mut record: Preparation = serde_json::from_value(value.clone())?;
            preparation(map, request, &mut record)?;
            serde_json::to_value(record)?
        }
        "object_attempt" => {
            let mut record: Attempt = serde_json::from_value(value.clone())?;
            attempt(map, request, prompts, &mut record)?;
            serde_json::to_value(record)?
        }
        "object_attempt_trace" => {
            let mut record: trace::Trace = serde_json::from_value(value.clone())?;
            record.request.project_id = request.target_project_id.clone();
            ids.id(&mut record.request.run_id, "object_run")?;
            ids.id(&mut record.request.attempt_id, "object_attempt")?;
            serde_json::to_value(record)?
        }
        "object_attempt_check_report" => {
            let mut report: checks::Report = serde_json::from_value(value.clone())?;
            let mut rewritten = crate::object_attempt_view::read(
                db,
                &request.source_project_id,
                &report.request.target.attempt_id,
            )?;
            attempt(map, request, prompts, &mut rewritten)?;
            report.request.project_id = request.target_project_id.clone();
            ids.id(&mut report.request.request_id, "object_check_request")?;
            target(request, &mut report.request.target)?;
            // Content, time, rules and results remain source evidence. Only identity digest changes.
            report.attempt_digest = crate::framework_checks::digest(&rewritten)?;
            checks::verify_report(&report, &rewritten)?;
            serde_json::to_value(report)?
        }
        "object_attempt_interrupt_receipt" => {
            let mut receipt: control::Pending = serde_json::from_value(value.clone())?;
            receipt.request.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request.request_id, "object_interrupt_request")?;
            target(request, &mut receipt.request.target)?;
            let result = receipt
                .result
                .as_mut()
                .context("DERIVATION_EXECUTION_INTERRUPT_INCOMPLETE")?;
            result.project_id = request.target_project_id.clone();
            target(request, &mut result.target)?;
            control::validate_result(&receipt)?;
            serde_json::to_value(receipt)?
        }
        _ => bail!("unsupported execution kind: {kind}"),
    };
    Ok((key, value))
}

#[cfg(all(test, windows))]
#[path = "project_derivation_execution_tests.rs"]
mod tests;
