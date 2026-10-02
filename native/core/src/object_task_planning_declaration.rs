//! Owner acknowledgement of a concrete planning scope, never task acceptance.
use crate::{
    object_task_definition::{DefinitionAdopter, TaskDefinition},
    object_task_storage as storage,
    object_task_types::{valid_id, Granularity, TaskRecord, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const KIND: &str = "object_task_planning_declaration";
const RECEIPT_KIND: &str = "object_task_planning_declaration_receipt";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub task_id: String,
    pub request_id: String,
    pub expected_plan_revision: u64,
    pub expected_scope_hash: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declaration {
    pub request: Request,
    pub declared_by: DefinitionAdopter,
    pub created_at: String,
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub task_id: String,
    pub scope_hash: String,
    pub blockers: Vec<String>,
    pub declaration: Option<Declaration>,
    pub current: bool,
}

pub(crate) fn scope<'a>(tasks: &'a [TaskRecord], root: &TaskRecord) -> Vec<&'a TaskRecord> {
    let mut ids = BTreeSet::from([root.id.clone()]);
    loop {
        let count = ids.len();
        for task in tasks {
            if task.project_id == root.project_id
                && task
                    .parent_task_id
                    .as_ref()
                    .is_some_and(|id| ids.contains(id))
            {
                ids.insert(task.id.clone());
            }
        }
        if count == ids.len() {
            break;
        }
    }
    let mut result: Vec<_> = tasks.iter().filter(|task| ids.contains(&task.id)).collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

pub(crate) fn scope_state(tasks: &[TaskRecord], root: &TaskRecord) -> Result<State> {
    let members = scope(tasks, root);
    // Ignore execution revisions and non-cancellation status transitions.
    let definitions: Vec<_> = members
        .iter()
        .map(|task| {
            serde_json::json!({
                "task":TaskDefinition::from_task(task).proposal(task),
                "runId":task.run_id,"cancelled":task.status == "cancelled"
            })
        })
        .collect();
    let scope_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&definitions)?));
    let mut blockers = Vec::new();
    if root.status == "cancelled" {
        blockers.push("目标任务已撤销".into());
    }
    for task in members.iter().filter(|task| task.status != "cancelled") {
        if !task.pending_planning.trim().is_empty() {
            blockers.push(format!("{}：仍有待规划事项", task.id));
        }
        if task.granularity != Granularity::Fine
            && !members.iter().any(|child| {
                child.parent_task_id.as_deref() == Some(&task.id) && child.status != "cancelled"
            })
        {
            blockers.push(format!("{}：尚无有效子任务", task.id));
        }
    }
    Ok(State {
        task_id: root.id.clone(),
        scope_hash,
        blockers,
        declaration: None,
        current: false,
    })
}

fn project(connection: &Connection, tasks: &[TaskRecord], root: &TaskRecord) -> Result<State> {
    let mut state = scope_state(tasks, root)?;
    state.declaration = storage::read(connection, KIND, &root.id)?;
    state.current = state.declaration.as_ref().is_some_and(|value| {
        value.request.project_id == root.project_id
            && value.request.task_id == root.id
            && value.request.expected_scope_hash == state.scope_hash
            && state.blockers.is_empty()
    });
    Ok(state)
}

pub(crate) fn snapshot_in(connection: &Connection, tasks: &[TaskRecord]) -> Result<Vec<State>> {
    tasks
        .iter()
        .filter(|task| task.granularity != Granularity::Fine)
        .map(|root| project(connection, tasks, root))
        .collect()
}

pub fn declare(runtime: &ProjectRuntime, request: &Request) -> Result<Declaration> {
    ensure!(
        runtime.project_id() == request.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        [&request.project_id, &request.task_id, &request.request_id]
            .into_iter()
            .all(|id| valid_id(id)),
        "INVALID_OBJECT_PLANNING_DECLARATION_ID"
    );
    ensure!(
        request.expected_plan_revision <= MAX_REVISION
            && request.expected_scope_hash.len() == 64
            && request
                .expected_scope_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit()),
        "INVALID_OBJECT_PLANNING_DECLARATION_REVISION"
    );
    ensure!(
        !request.reason.trim().is_empty() && request.reason.len() <= 2000,
        "INVALID_OBJECT_PLANNING_DECLARATION_REASON"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) =
            storage::read::<Declaration>(connection, RECEIPT_KIND, &request.request_id)?
        {
            ensure!(
                receipt.request == *request,
                "OBJECT_PLANNING_DECLARATION_REQUEST_CONFLICT"
            );
            return Ok(receipt);
        }
        ensure!(
            storage::plan_state(connection, &request.project_id)?.revision
                == request.expected_plan_revision,
            "OBJECT_TASK_PLAN_REVISION_CONFLICT"
        );
        let tasks: Vec<_> = storage::all_tasks(connection)?
            .into_iter()
            .filter(|task| task.project_id == request.project_id)
            .collect();
        let root = tasks
            .iter()
            .find(|task| task.id == request.task_id)
            .context("OBJECT_TASK_NOT_FOUND")?;
        ensure!(
            root.granularity != Granularity::Fine,
            "OBJECT_PLANNING_DECLARATION_PARENT_REQUIRED"
        );
        let state = project(connection, &tasks, root)?;
        ensure!(
            state.scope_hash == request.expected_scope_hash,
            "OBJECT_PLANNING_DECLARATION_SCOPE_CONFLICT"
        );
        ensure!(
            state.blockers.is_empty(),
            "OBJECT_PLANNING_DECLARATION_INCOMPLETE: {}",
            state.blockers.join("; ")
        );
        let receipt = Declaration {
            request: request.clone(),
            declared_by: DefinitionAdopter::Owner,
            created_at: chrono::Utc::now().to_rfc3339(),
            task_ids: scope(&tasks, root)
                .iter()
                .map(|task| task.id.clone())
                .collect(),
        };
        storage::replace(connection, KIND, &request.task_id, &receipt)?;
        storage::insert(connection, RECEIPT_KIND, &request.request_id, &receipt)?;
        Ok(receipt)
    })
}
