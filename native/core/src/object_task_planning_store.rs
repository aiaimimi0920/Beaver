use crate::{
    object_task_planning_types::{Head, Receipt, Record, Session, StartRequest, Status},
    object_task_storage::{self as storage, DRAFT_KIND},
    object_task_types::{valid_id, Draft, PlanProposal, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde_json::{json, Value};

pub(crate) const KIND: &str = "object_task_planning";
pub(crate) const HEAD_KIND: &str = "object_task_planning_head";
const RECEIPT_KIND: &str = "object_task_planning_receipt";

pub(crate) fn transact<T>(
    runtime: &ProjectRuntime,
    project_id: &str,
    action: impl FnOnce(&Connection) -> Result<T>,
) -> Result<T> {
    ensure!(
        runtime.project_id() == project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("planning store lock poisoned"))?;
    store.transaction(action)
}

pub(crate) fn read(db: &Connection, project_id: &str, id: &str) -> Result<Record> {
    ensure!(valid_id(id), "INVALID_PLANNING_ID");
    let record: Record = storage::read(db, KIND, id)?.context("OBJECT_PLANNING_NOT_FOUND")?;
    ensure!(
        record.project_id == project_id && record.session.project_id == project_id,
        "OBJECT_PLANNING_PROJECT_MISMATCH"
    );
    ensure!(
        record.session.id == id && record.session.input.project_id == project_id,
        "OBJECT_PLANNING_IDENTITY_MISMATCH"
    );
    Ok(record)
}

pub(crate) fn current(db: &Connection, project_id: &str, draft_id: &str) -> Result<Option<Record>> {
    ensure!(valid_id(draft_id), "INVALID_PLANNING_DRAFT_ID");
    let Some(head) = storage::read::<Head>(db, HEAD_KIND, draft_id)? else {
        return Ok(None);
    };
    ensure!(
        head.project_id == project_id,
        "OBJECT_PLANNING_PROJECT_MISMATCH"
    );
    let record = read(db, project_id, &head.session_id)?;
    ensure!(
        record.session.input.draft_id == draft_id,
        "OBJECT_PLANNING_IDENTITY_MISMATCH"
    );
    Ok(Some(record))
}

pub(crate) fn save(db: &Connection, record: &mut Record) -> Result<()> {
    ensure!(
        record.session.revision < MAX_REVISION,
        "OBJECT_PLANNING_REVISION_EXHAUSTED"
    );
    record.session.revision += 1;
    storage::replace(db, KIND, &record.session.id, record)
}

pub(crate) fn scope(db: &Connection, input: &StartRequest) -> Result<PlanProposal> {
    let state = storage::plan_state(db, &input.project_id)?;
    ensure!(
        state.project_id == input.project_id,
        "OBJECT_TASK_PLAN_PROJECT_MISMATCH"
    );
    ensure!(
        state.revision == input.expected_plan_revision,
        "OBJECT_TASK_PLAN_REVISION_CONFLICT"
    );
    let draft = storage::read::<Draft>(db, DRAFT_KIND, &input.draft_id)?;
    if let Some(draft) = &draft {
        ensure!(
            draft.committed_request_id.is_none(),
            "OBJECT_TASK_DRAFT_LOCKED"
        );
        ensure!(
            draft.project_id == input.project_id && draft.id == input.draft_id,
            "OBJECT_TASK_DRAFT_IDENTITY_MISMATCH"
        );
        ensure!(
            draft.plan_revision == state.revision,
            "OBJECT_TASK_PLAN_REVISION_CONFLICT"
        );
    }
    ensure!(
        draft.as_ref().map_or(0, |d| d.revision) == input.expected_draft_revision,
        "OBJECT_TASK_DRAFT_REVISION_CONFLICT"
    );
    Ok(draft.map(|d| d.plan).unwrap_or_default())
}

pub(crate) fn repeat(
    db: &Connection,
    project_id: &str,
    request_id: &str,
    operation: &str,
    request: &Value,
) -> Result<Option<Session>> {
    ensure!(valid_id(request_id), "INVALID_PLANNING_REQUEST_ID");
    let Some(receipt) = storage::read::<Receipt>(db, RECEIPT_KIND, request_id)? else {
        return Ok(None);
    };
    ensure!(
        receipt.project_id == project_id
            && receipt.operation == operation
            && receipt.request == *request,
        "OBJECT_PLANNING_REQUEST_ID_REUSED"
    );
    Ok(Some(read(db, project_id, &receipt.session_id)?.session))
}

pub(crate) fn receipt(
    db: &Connection,
    session: &Session,
    request_id: &str,
    operation: &str,
    request: Value,
) -> Result<()> {
    storage::insert(
        db,
        RECEIPT_KIND,
        request_id,
        &Receipt {
            project_id: session.project_id.clone(),
            session_id: session.id.clone(),
            operation: operation.into(),
            request,
        },
    )
}

pub(crate) fn active(
    db: &Connection,
    project_id: &str,
    session_id: &str,
    round_id: &str,
) -> Result<Record> {
    let record = read(db, project_id, session_id)?;
    ensure!(
        record.session.status == Status::Running && record.session.round_id == round_id,
        "OBJECT_PLANNING_STALE_ROUND"
    );
    Ok(record)
}

pub(crate) fn context(db: &Connection, project_id: &str) -> Result<Value> {
    let objects = storage::read_all::<crate::object_catalog::ObjectRecord>(db, "object")?
        .into_iter()
        .filter(|o| o.project_id == project_id)
        .collect::<Vec<_>>();
    let tasks = storage::all_tasks(db)?
        .into_iter()
        .filter(|t| t.project_id == project_id)
        .collect::<Vec<_>>();
    let runs = storage::all_runs(db)?
        .into_iter()
        .filter(|r| r.project_id == project_id)
        .collect::<Vec<_>>();
    let context = json!({"objects":objects,"tasks":tasks,"runs":runs,
        "assumptions":storage::plan_state(db, project_id)?.assumptions});
    ensure!(
        serde_json::to_vec(&context)?.len() <= 256 * 1024,
        "OBJECT_PLANNING_CONTEXT_LIMIT"
    );
    Ok(context)
}

pub(crate) fn merged(record: &Record, proposal: &PlanProposal) -> PlanProposal {
    let mut plan = record.baseline.clone();
    plan.objects.extend(proposal.objects.clone());
    plan.tasks.extend(proposal.tasks.clone());
    plan.assumptions.extend(proposal.assumptions.clone());
    plan
}
