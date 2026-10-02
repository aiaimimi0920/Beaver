//! Typed queue/control conversion and an additive pause, never a rewrite of command history.
use crate::{
    object_task_coarse_dispatch as coarse, object_task_dispatch as dispatch,
    object_task_queue::{self as queue, QueueEntry},
    object_task_queue_reorder as reorder,
    project_derivation_copy::Request,
    project_derivation_identity::{generated, IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "object_task_queue"
            | "object_task_queue_order_revision"
            | "object_task_queue_reorder_receipt"
            | "object_task_dispatch_control"
            | "object_task_dispatch_receipt"
            | "object_task_coarse_dispatch_control"
            | "object_task_coarse_dispatch_receipt"
    )
}

pub(crate) fn receipt_key(project: &str, request: &str) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(project, request))?)
    ))
}

fn request_namespace(kind: &str) -> Result<&str> {
    match kind {
        "object_task_queue_reorder_receipt" => Ok("object_queue_request"),
        "object_task_dispatch_receipt" => Ok("object_dispatch_request"),
        "object_task_coarse_dispatch_receipt" => Ok("object_coarse_dispatch_request"),
        _ => bail!("unsupported derivation receipt: {kind}"),
    }
}

pub(crate) fn target_key(kind: &str, value: &Value, request: &Request) -> Result<String> {
    let field = |name: &str| {
        value[name]
            .as_str()
            .context("derivation queue identity missing")
    };
    match kind {
        "object_task_queue" => Ok(format!(
            "{}:{}",
            request.target_project_id,
            generated(request, "object_task", field("taskId")?)?
        )),
        "object_task_queue_order_revision" => Ok(request.target_project_id.clone()),
        "object_task_dispatch_control" => generated(request, "object_run", field("runId")?),
        "object_task_coarse_dispatch_control" => {
            generated(request, "object_task", field("taskId")?)
        }
        _ => receipt_key(
            &request.target_project_id,
            &generated(
                request,
                request_namespace(kind)?,
                value["request"]["requestId"]
                    .as_str()
                    .context("derivation queue request missing")?,
            )?,
        ),
    }
}

fn historical_token(request: &Request, token: &str) -> Result<String> {
    // Receipts lack the historical hashing inputs. This is a provenance-scoped CAS alias,
    // NOT a recomputation of a historical queue snapshot. Current views use normal hashing.
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            "beaver-derived-queue-history-token-v1",
            &request.source_project_id,
            &request.target_project_id,
            &request.request_id,
            token,
        ))?)
    ))
}

fn medium(ids: &PlanIds<'_>, control: &mut dispatch::Control) -> Result<()> {
    control.project_id = ids.0.target_project_id.clone();
    ids.id(&mut control.task_id, "object_task")?;
    ids.id(&mut control.object_id, "object")?;
    ids.id(&mut control.run_id, "object_run")
}

pub(crate) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let ids = PlanIds(request);
    let key = Rewrite(map).key(kind, id)?;
    let value = match kind {
        "object_task_queue" => {
            let mut entry: QueueEntry = serde_json::from_value(value.clone())?;
            entry.project_id = request.target_project_id.clone();
            ids.id(&mut entry.task_id, "object_task")?;
            ids.optional(&mut entry.owner, "object_writer_owner")?;
            ids.optional(&mut entry.claim_token, "object_writer_claim")?;
            entry.id = key.id.clone();
            serde_json::to_value(entry)?
        }
        "object_task_queue_order_revision" => value.clone(),
        "object_task_queue_reorder_receipt" => {
            let mut receipt: reorder::Receipt = serde_json::from_value(value.clone())?;
            let command = &mut receipt.request;
            command.project_id = request.target_project_id.clone();
            ids.id(&mut command.request_id, request_namespace(kind)?)?;
            ids.id(&mut command.task_id, "object_task")?;
            ids.optional(&mut command.previous_task_id, "object_task")?;
            ids.optional(&mut command.next_task_id, "object_task")?;
            command.expected_version = historical_token(request, &command.expected_version)?;
            receipt.result.project_id = request.target_project_id.clone();
            receipt.result.version = historical_token(request, &receipt.result.version)?;
            for item in &mut receipt.result.items {
                ids.id(&mut item.task_id, "object_task")?;
                ids.id(&mut item.object_id, "object")?;
            }
            serde_json::to_value(receipt)?
        }
        "object_task_dispatch_control" => {
            let mut control: dispatch::Control = serde_json::from_value(value.clone())?;
            medium(&ids, &mut control)?;
            serde_json::to_value(control)?
        }
        "object_task_dispatch_receipt" => {
            let mut receipt: dispatch::Receipt = serde_json::from_value(value.clone())?;
            let command = &mut receipt.request;
            command.project_id = request.target_project_id.clone();
            ids.id(&mut command.request_id, request_namespace(kind)?)?;
            ids.id(&mut command.task_id, "object_task")?;
            ids.id(&mut command.object_id, "object")?;
            ids.id(&mut command.run_id, "object_run")?;
            medium(&ids, &mut receipt.result)?;
            serde_json::to_value(receipt)?
        }
        "object_task_coarse_dispatch_control" => {
            let mut control: coarse::Control = serde_json::from_value(value.clone())?;
            control.project_id = request.target_project_id.clone();
            ids.id(&mut control.task_id, "object_task")?;
            serde_json::to_value(control)?
        }
        "object_task_coarse_dispatch_receipt" => {
            let mut receipt: coarse::Receipt = serde_json::from_value(value.clone())?;
            receipt.request.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.request.request_id, request_namespace(kind)?)?;
            ids.id(&mut receipt.request.task_id, "object_task")?;
            receipt.result.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.result.task_id, "object_task")?;
            serde_json::to_value(receipt)?
        }
        _ => bail!("unsupported derivation queue kind: {kind}"),
    };
    Ok((key, value))
}

pub(crate) fn stage_copied(
    source: &Connection,
    connection: &Connection,
    request: &Request,
) -> Result<()> {
    // Enqueue permits tied positions. Preserve the source's full order independently of new IDs.
    for (position, original) in queue::list_in(source, &request.source_project_id)?
        .iter()
        .enumerate()
    {
        let task = generated(request, "object_task", &original.task_id)?;
        let mut entry = queue::read_entry(connection, &request.target_project_id, &task)?
            .context("DERIVATION_QUEUE_TARGET_ENTRY_MISSING")?;
        entry.position = position as u64;
        crate::object_task_storage::replace(connection, queue::QUEUE_KIND, &entry.id, &entry)?;
    }
    for entry in queue::list_in(connection, &request.target_project_id)? {
        if !matches!(
            entry.state.as_str(),
            "queued" | "failed" | "awaitingAcceptance"
        ) {
            continue;
        }
        let (task, run) = queue::claim::read_medium(connection, &entry.project_id, &entry.task_id)?;
        let control = dispatch::read_in(connection, &task, &run)?;
        if control.paused {
            continue;
        }
        dispatch::set_paused_in(
            connection,
            &dispatch::SetPausedRequest {
                project_id: task.project_id.clone(),
                task_id: task.id.clone(),
                object_id: run.object_id.clone(),
                run_id: run.id.clone(),
                request_id: generated(request, "object_derivation_safety_pause", &task.id)?,
                expected_task_revision: task.revision,
                expected_control_revision: control.revision,
                paused: true,
            },
        )?;
    }
    Ok(())
}
