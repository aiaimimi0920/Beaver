//! Historical Records retain revisions, queue positions and pause state; only identities change.
use super::super::records::Records;
use crate::{
    object_run_recovery::resume::derivation::rework::Prompts, project_derivation_copy::Request,
    project_derivation_execution_records as execution, project_derivation_identity::IdentityMap,
    project_derivation_object_records::rewrite_snapshot, project_derivation_plan_rewrite::PlanIds,
};
use anyhow::Result;

pub(in crate::object_run_recovery) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    records: &mut Records,
) -> Result<()> {
    let ids = PlanIds(request);
    rewrite_snapshot(map, request, &mut records.object)?;
    ids.optional(&mut records.accepted_version_id, "object_version")?;
    ids.task(&mut records.medium)?;
    ids.run(&mut records.run)?;
    let queue = &mut records.queue;
    queue.project_id = request.target_project_id.clone();
    ids.id(&mut queue.task_id, "object_task")?;
    ids.optional(&mut queue.owner, "object_writer_owner")?;
    ids.optional(&mut queue.claim_token, "object_writer_claim")?;
    queue.id = format!("{}:{}", queue.project_id, queue.task_id);
    execution::preparation(map, request, &mut records.preparation)?;
    let control = &mut records.control;
    control.project_id = request.target_project_id.clone();
    ids.id(&mut control.task_id, "object_task")?;
    ids.id(&mut control.object_id, "object")?;
    ids.id(&mut control.run_id, "object_run")?;
    if let Some(attempt) = &mut records.attempt {
        execution::attempt(map, request, prompts, attempt)?;
    }
    for attempt in &mut records.history {
        execution::attempt(map, request, prompts, attempt)?;
    }
    if let Some(fine) = &mut records.fine {
        prompts.task(fine);
        ids.task(fine)?;
    }
    for receipt in &mut records.interrupts {
        receipt.request.project_id = request.target_project_id.clone();
        ids.id(&mut receipt.request.request_id, "object_interrupt_request")?;
        execution::target(request, &mut receipt.request.target)?;
        if let Some(result) = &mut receipt.result {
            result.project_id = request.target_project_id.clone();
            execution::target(request, &mut result.target)?;
        }
    }
    records
        .interrupts
        .sort_by(|a, b| a.request.request_id.cmp(&b.request.request_id));
    for source in &mut records.baseline_sources {
        rewrite_snapshot(map, request, source)?;
    }
    // Production snapshots use BTreeSet object order, not the original source ID order.
    records.baseline_sources.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(())
}
