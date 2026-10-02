//! Owner planning acknowledgements retain historical scope, never execution authority.
use crate::{
    object_task_planning_declaration::{self as declaration, Declaration},
    object_task_types::{valid_id, Granularity, TaskRecord},
    project_derivation_copy::Request,
    project_derivation_identity::{IdentityMap, Key},
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_plan_validation::Plans,
    project_derivation_validation_records::Rewrite,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

const HEAD: &str = "object_task_planning_declaration";
const RECEIPT: &str = "object_task_planning_declaration_receipt";

pub(crate) fn supports(kind: &str) -> bool {
    matches!(kind, HEAD | RECEIPT)
}

fn root<'a>(tasks: &'a [TaskRecord], id: &str) -> Result<&'a TaskRecord> {
    let root = tasks
        .iter()
        .find(|t| t.id == id)
        .context("DERIVATION_DECLARATION_TASK_MISSING")?;
    ensure!(
        root.granularity != Granularity::Fine,
        "DERIVATION_DECLARATION_PARENT_REQUIRED"
    );
    Ok(root)
}

fn historical(plans: &Plans, value: &Declaration) -> Result<Vec<TaskRecord>> {
    let request = &value.request;
    ensure!(
        request.project_id == plans.project
            && [&request.project_id, &request.task_id, &request.request_id]
                .into_iter()
                .all(|id| valid_id(id)),
        "DERIVATION_DECLARATION_IDENTITY"
    );
    ensure!(
        !request.reason.trim().is_empty() && request.reason.len() <= 2000,
        "DERIVATION_DECLARATION_REASON"
    );
    ensure!(
        chrono::DateTime::parse_from_rfc3339(&value.created_at).is_ok(),
        "DERIVATION_DECLARATION_TIMESTAMP"
    );
    let tasks = plans.tasks_at(request.expected_plan_revision)?;
    let root = root(&tasks, &request.task_id)?;
    let state = declaration::scope_state(&tasks, root)?;
    ensure!(
        state.blockers.is_empty(),
        "DERIVATION_DECLARATION_INCOMPLETE"
    );
    ensure!(
        state.scope_hash == request.expected_scope_hash,
        "DERIVATION_DECLARATION_SCOPE_HASH"
    );
    let ids: Vec<_> = declaration::scope(&tasks, root)
        .iter()
        .map(|t| t.id.clone())
        .collect();
    ensure!(value.task_ids == ids, "DERIVATION_DECLARATION_TASK_SET");
    Ok(tasks)
}

pub(crate) fn validate(entities: &[Entity], plans: &Plans) -> Result<()> {
    let mut heads = BTreeMap::new();
    let mut receipts = BTreeMap::new();
    for entity in entities.iter().filter(|e| supports(&e.kind)) {
        let value: Declaration =
            serde_json::from_value(entity.value.clone().context("invalid declaration JSON")?)
                .with_context(|| format!("derivation {}/{}", entity.kind, entity.id))?;
        let expected_key = if entity.kind == HEAD {
            &value.request.task_id
        } else {
            &value.request.request_id
        };
        ensure!(expected_key == &entity.id, "DERIVATION_DECLARATION_KEY");
        historical(plans, &value)?;
        if entity.kind == HEAD {
            heads.insert(entity.id.clone(), value);
        } else {
            receipts.insert(entity.id.clone(), value);
        }
    }
    for head in heads.values() {
        ensure!(
            receipts.get(&head.request.request_id) == Some(head),
            "DERIVATION_DECLARATION_RECEIPT_MISSING_OR_MISMATCH"
        );
    }
    for receipt in receipts.values() {
        let head = heads
            .get(&receipt.request.task_id)
            .context("DERIVATION_DECLARATION_HEAD_MISSING")?;
        // Same-revision receipts are legal; wall-clock time does not define their order.
        ensure!(
            head.request.expected_plan_revision >= receipt.request.expected_plan_revision,
            "DERIVATION_DECLARATION_HEAD_REGRESSED"
        );
    }
    Ok(())
}

pub(crate) fn rewrite(
    plans: &Plans,
    map: &IdentityMap,
    request: &Request,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let key = Rewrite(map).key(kind, id)?;
    let mut value: Declaration = serde_json::from_value(value.clone())?;
    let mut tasks = historical(plans, &value)?;
    let ids = PlanIds(request);
    for task in &mut tasks {
        ids.task(task)?;
    }
    value.request.project_id = request.target_project_id.clone();
    ids.id(&mut value.request.task_id, "object_task")?;
    ids.id(&mut value.request.request_id, "object_task_request")?;
    let root = root(&tasks, &value.request.task_id)?;
    value.request.expected_scope_hash = declaration::scope_state(&tasks, root)?.scope_hash;
    // Mapping can change lexical order; hash and taskIds use the same target scope order.
    value.task_ids = declaration::scope(&tasks, root)
        .iter()
        .map(|t| t.id.clone())
        .collect();
    Ok((key, serde_json::to_value(value)?))
}
