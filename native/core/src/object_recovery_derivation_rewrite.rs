//! Rewrite completed recovery journals without changing their historical authority or evidence.
use super::{
    rework::{self, Prompts},
    storage, verification, Head,
};
use crate::{
    object_run_recovery::{resume::Outcome, Target},
    project_derivation_copy::Request,
    project_derivation_identity::{generated, IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_queue_records::receipt_key,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, Result};
use serde_json::Value;

use crate::object_run_recovery::derivation::conversion_records as records;

const VERIFY_REQUEST: &str = "object_recovery_verify_request";
const RESUME_REQUEST: &str = "object_recovery_resume_request";

pub(crate) fn target_key(kind: &str, id: &str, value: &Value, request: &Request) -> Result<String> {
    let (namespace, source) = match kind {
        "object_recovery_verification" => {
            let saved: verification::Stored = serde_json::from_value(value.clone())?;
            return receipt_key(
                &request.target_project_id,
                &generated(request, VERIFY_REQUEST, &saved.operation.request.request_id)?,
            );
        }
        "object_recovery_resume" => {
            let saved: storage::Stored = serde_json::from_value(value.clone())?;
            return receipt_key(
                &request.target_project_id,
                &generated(request, RESUME_REQUEST, &saved.operation.request.request_id)?,
            );
        }
        "object_recovery_head" => ("object_run", id),
        "object_recovery_resume_head" => ("object_task", id),
        "object_recovery_attempt_successor" => ("object_attempt", id),
        _ => bail!("unsupported derivation recovery kind: {kind}"),
    };
    generated(request, namespace, source)
}

fn target(request: &Request, value: &mut Target) -> Result<()> {
    let ids = PlanIds(request);
    ids.id(&mut value.task_id, "object_task")?;
    ids.id(&mut value.object_id, "object")?;
    ids.id(&mut value.run_id, "object_run")?;
    ids.id(&mut value.owner, "object_writer_owner")?;
    ids.id(&mut value.claim_token, "object_writer_claim")
}

fn verification(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    saved: &mut verification::Stored,
) -> Result<()> {
    let command = &mut saved.operation.request;
    command.project_id = request.target_project_id.clone();
    PlanIds(request).id(&mut command.request_id, VERIFY_REQUEST)?;
    target(request, &mut command.target)?;
    records::rewrite(map, request, prompts, &mut saved.records)
}

pub(crate) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    prompts: &Prompts,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let key = Rewrite(map).key(kind, id)?;
    let ids = PlanIds(request);
    let value = match kind {
        "object_recovery_verification" => {
            let mut saved: verification::Stored = serde_json::from_value(value.clone())?;
            verification(map, request, prompts, &mut saved)?;
            serde_json::to_value(saved)?
        }
        "object_recovery_head" => {
            let mut head: Head = serde_json::from_value(value.clone())?;
            head.project_id = request.target_project_id.clone();
            ids.id(&mut head.task_id, "object_task")?;
            ids.id(&mut head.run_id, "object_run")?;
            ids.id(&mut head.request_id, VERIFY_REQUEST)?;
            serde_json::to_value(head)?
        }
        "object_recovery_resume" => {
            let mut saved: storage::Stored = serde_json::from_value(value.clone())?;
            let command = &mut saved.operation.request;
            command.project_id = request.target_project_id.clone();
            ids.id(&mut command.request_id, RESUME_REQUEST)?;
            ids.id(&mut command.verification_request_id, VERIFY_REQUEST)?;
            target(request, &mut command.target)?;
            if let Some(advance) = &mut command.advance {
                ids.id(&mut advance.attempt_id, "object_attempt")?;
                ids.id(&mut advance.check_request_id, "object_check_request")?;
                ids.id(&mut advance.next_fine_task_id, "object_task")?;
            }
            if let Some(next) = &mut saved.next {
                prompts.task(next);
                ids.task(next)?;
            }
            if let Some(approval) = &mut command.rework {
                rework::approval(request, approval)?;
            }
            verification(map, request, prompts, &mut saved.verification)?;
            if let Some(Outcome::Started {
                attempt_id,
                fine_task_id,
                ..
            }) = &mut saved.operation.result
            {
                ids.id(attempt_id, "object_attempt")?;
                ids.id(fine_task_id, "object_task")?;
            }
            if let Some(started) = &mut saved.started {
                crate::project_derivation_execution_records::attempt(
                    map, request, prompts, started,
                )?;
            }
            serde_json::to_value(saved)?
        }
        "object_recovery_resume_head" | "object_recovery_attempt_successor" => {
            let mut request_id: String = serde_json::from_value(value.clone())?;
            ids.id(&mut request_id, RESUME_REQUEST)?;
            serde_json::to_value(request_id)?
        }
        _ => bail!("unsupported derivation recovery kind: {kind}"),
    };
    Ok((key, value))
}
