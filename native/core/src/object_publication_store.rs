//! Durable operation identity and exclusion; no project files are read here.
use super::{commit, integrity, prepare, Operation, Request, State, Stored};
use crate::{object_task_storage as store, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

const KIND: &str = "object_publication";

fn key(project: &str, request: &str) -> Result<String> {
    crate::framework_checks::digest(&(project, request))
}

pub(super) fn read(db: &Connection, project: &str, request: &str) -> Result<Option<Stored>> {
    let saved: Option<Stored> = store::read(db, KIND, &key(project, request)?)?;
    if let Some(saved) = &saved {
        ensure!(
            saved.operation.request.project_id == project
                && saved.operation.request.request_id == request,
            "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
        );
        integrity::validate(db, saved)?;
    }
    Ok(saved)
}

pub(super) fn all(db: &Connection, project: &str) -> Result<Vec<Stored>> {
    let mut statement = db.prepare("SELECT id,value FROM entities WHERE kind=? ORDER BY id")?;
    let rows = statement.query_map([KIND], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut operations = Vec::new();
    for row in rows {
        let (id, json) = row?;
        let saved: Stored = serde_json::from_str(&json)?;
        let request = &saved.operation.request;
        ensure!(
            id == key(&request.project_id, &request.request_id)?,
            "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
        );
        if request.project_id == project {
            integrity::validate(db, &saved)?;
            operations.push(saved);
        }
    }
    Ok(operations)
}

pub(crate) fn blocked(db: &Connection, project: &str) -> Result<bool> {
    Ok(all(db, project)?
        .iter()
        .any(|saved| saved.operation.pending()))
}

pub(crate) fn require_task_idle(db: &Connection, project: &str, task: &str) -> Result<()> {
    ensure!(
        !task_pending(db, project, task)?,
        "OBJECT_PUBLICATION_PENDING"
    );
    Ok(())
}

pub(crate) fn task_pending(db: &Connection, project: &str, task: &str) -> Result<bool> {
    Ok(all(db, project)?
        .iter()
        .any(|saved| saved.operation.pending() && saved.operation.request.target.task_id == task))
}

pub(super) fn replay(db: &Connection, request: &Request) -> Result<Option<Stored>> {
    let saved = read(db, &request.project_id, &request.request_id)?;
    if let Some(saved) = &saved {
        ensure!(
            saved.operation.request == *request,
            "OBJECT_PUBLICATION_REQUEST_CONFLICT"
        );
    }
    Ok(saved)
}

pub(super) fn validate_current(db: &Connection, saved: &Stored) -> Result<()> {
    let (source, review, preview) = prepare::load(db, &saved.operation.request.review())?;
    ensure!(
        source == saved.source && review == saved.review && preview == saved.operation.preview,
        "OBJECT_PUBLICATION_SOURCE_CHANGED"
    );
    commit::validate_headroom(db, saved)
}

pub(super) fn begin(runtime: &ProjectRuntime, db: &Connection, saved: &Stored) -> Result<()> {
    let request = &saved.operation.request;
    ensure!(
        !blocked(db, &request.project_id)?,
        "OBJECT_PUBLICATION_PENDING"
    );
    let legacy: Vec<crate::journal::FileOperation> = store::read_all(db, "operation")?;
    ensure!(
        !legacy.iter().any(|op| op.project_id == request.project_id
            && matches!(
                op.state,
                crate::journal::OperationState::Applying | crate::journal::OperationState::Aborting
            )),
        "OBJECT_PUBLICATION_PROJECT_RECOVERY_PENDING"
    );
    validate_current(db, saved)?;
    integrity::validate(db, saved)?;
    ensure!(
        commit::command(runtime, saved)?.replay(db)?.is_none(),
        "OBJECT_REQUEST_CONFLICT"
    );
    store::insert(
        db,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        saved,
    )
}

pub(super) fn save(db: &Connection, saved: &Stored) -> Result<()> {
    let request = &saved.operation.request;
    let previous = read(db, &request.project_id, &request.request_id)?
        .context("OBJECT_PUBLICATION_NOT_FOUND")?;
    ensure!(
        previous.operation.pending()
            && previous.source == saved.source
            && previous.review == saved.review
            && previous.manifest == saved.manifest
            && previous.operation.request == *request
            && previous.operation.preview == saved.operation.preview
            && saved.writes.starts_with(&previous.writes),
        "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
    );
    ensure!(
        previous.operation.state != State::Aborting
            || matches!(saved.operation.state, State::Aborting | State::Aborted),
        "OBJECT_PUBLICATION_ABORT_PENDING"
    );
    integrity::validate(db, saved)?;
    store::replace(
        db,
        KIND,
        &key(&request.project_id, &request.request_id)?,
        saved,
    )
}

pub(super) fn record_error(db: &Connection, mut saved: Stored, error: String) -> Result<Operation> {
    saved.operation.error = Some(error);
    save(db, &saved)?;
    Ok(saved.operation)
}
