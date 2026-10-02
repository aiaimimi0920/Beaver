//! AI planning records retain history; only explicit target commands may launch a new round.
use crate::{
    object_task_planning_types::{
        AnswerRequest, Head, Receipt, Record, SessionRequest, StartRequest,
    },
    project_derivation_copy::Request,
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_planning_context::PlanningContext,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, ensure, Result};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub(crate) fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "object_task_planning" | "object_task_planning_head" | "object_task_planning_receipt"
    )
}

// The production wrapper types are permissive. Derivation must never discard future fields.
pub(crate) fn read<T: DeserializeOwned>(value: &Value, fields: &[&str]) -> Result<T> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("DERIVATION_PLANNING_INVALID_RECORD"))?;
    ensure!(
        object.len() == fields.len() && object.keys().all(|key| fields.contains(&key.as_str())),
        "DERIVATION_PLANNING_UNKNOWN_FIELD"
    );
    Ok(serde_json::from_value(value.clone())?)
}

pub(crate) fn record(value: &Value) -> Result<Record> {
    read(value, &["projectId", "session", "baseline", "context"])
}

pub(crate) fn head(value: &Value) -> Result<Head> {
    read(value, &["projectId", "sessionId"])
}

pub(crate) fn receipt(value: &Value) -> Result<Receipt> {
    read(value, &["projectId", "sessionId", "operation", "request"])
}

fn request_value<T: DeserializeOwned + Serialize>(
    value: &Value,
    action: impl FnOnce(&mut T) -> Result<()>,
) -> Result<Value> {
    let mut input: T = serde_json::from_value(value.clone())?;
    action(&mut input)?;
    Ok(serde_json::to_value(input)?)
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
        "object_task_planning" => {
            let mut record = record(value)?;
            record.project_id = request.target_project_id.clone();
            let session = &mut record.session;
            session.project_id = request.target_project_id.clone();
            ids.id(&mut session.id, "object_planning_request")?;
            ids.id(&mut session.round_id, "object_planning_round")?;
            session.input.project_id = request.target_project_id.clone();
            ids.id(&mut session.input.request_id, "object_planning_request")?;
            ids.plan(&mut record.baseline)?;
            if let Some(proposal) = &mut session.proposal {
                ids.plan(proposal)?;
            }
            if let Some(draft) = &mut session.adopted_draft {
                ids.draft(draft)?;
            }
            let mut context: PlanningContext = serde_json::from_value(record.context)?;
            context.rewrite(map, request)?;
            record.context = serde_json::to_value(context)?;
            // External thread/turn IDs are diagnostic history, never target execution handles.
            // Narrative text and session-local question/decision/assumption keys stay untouched.
            serde_json::to_value(record)?
        }
        "object_task_planning_head" => {
            let mut head = head(value)?;
            head.project_id = request.target_project_id.clone();
            ids.id(&mut head.session_id, "object_planning_request")?;
            serde_json::to_value(head)?
        }
        "object_task_planning_receipt" => {
            let mut receipt = receipt(value)?;
            receipt.project_id = request.target_project_id.clone();
            ids.id(&mut receipt.session_id, "object_planning_request")?;
            receipt.request = match receipt.operation.as_str() {
                "start" => request_value::<StartRequest>(&receipt.request, |input| {
                    input.project_id = request.target_project_id.clone();
                    ids.id(&mut input.request_id, "object_planning_request")
                })?,
                "answer" => request_value::<AnswerRequest>(&receipt.request, |input| {
                    input.project_id = request.target_project_id.clone();
                    ids.id(&mut input.session_id, "object_planning_request")?;
                    ids.id(&mut input.request_id, "object_planning_request")
                })?,
                "adopt" | "cancel" => request_value::<SessionRequest>(&receipt.request, |input| {
                    input.project_id = request.target_project_id.clone();
                    ids.id(&mut input.session_id, "object_planning_request")?;
                    ids.id(&mut input.request_id, "object_planning_request")
                })?,
                _ => bail!("DERIVATION_PLANNING_UNKNOWN_OPERATION"),
            };
            serde_json::to_value(receipt)?
        }
        _ => bail!("unsupported derivation planning kind: {kind}"),
    };
    Ok((key, value))
}

#[cfg(all(test, windows))]
#[path = "project_derivation_planning_tests.rs"]
mod tests;
