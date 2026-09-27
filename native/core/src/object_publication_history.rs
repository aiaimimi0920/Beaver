//! Published attempts retain immutable evidence after releasing queue ownership.
use super::{commit, storage, Operation, State, Stored};
use crate::{
    object_attempt::Attempt,
    object_attempt_view::{Target, View},
    object_task_storage as store,
};
use anyhow::{ensure, Result};
use rusqlite::Connection;

pub(super) fn list(db: &Connection, project: &str, task: &str) -> Result<Vec<Operation>> {
    let task_record: Option<crate::object_task_types::TaskRecord> =
        store::read(db, store::TASK_KIND, task)?;
    ensure!(
        task_record.is_some_and(|record| record.id == task && record.project_id == project),
        "OBJECT_TASK_NOT_FOUND"
    );
    let mut entries: Vec<_> = storage::all(db, project)?
        .into_iter()
        .filter(|saved| saved.operation.request.target.task_id == task)
        .map(|s| s.operation)
        .collect();
    entries.sort_by(|a, b| a.request.request_id.cmp(&b.request.request_id));
    Ok(entries)
}

fn published(db: &Connection, project: &str, task: &str) -> Result<Option<Stored>> {
    let mut entries = storage::all(db, project)?.into_iter().filter(|saved| {
        saved.operation.request.target.task_id == task && saved.operation.state == State::Published
    });
    let saved = entries.next();
    ensure!(
        entries.next().is_none(),
        "OBJECT_PUBLICATION_HISTORY_MISMATCH"
    );
    if let Some(saved) = &saved {
        let (fine, medium, run, queue) = commit::records(saved)?;
        ensure!(
            store::read(db, store::TASK_KIND, &fine.id)?.as_ref() == Some(&fine)
                && store::read(db, store::TASK_KIND, &medium.id)?.as_ref() == Some(&medium)
                && store::read(db, store::RUN_KIND, &run.id)?.as_ref() == Some(&run)
                && store::read(db, crate::object_task_queue::QUEUE_KIND, &queue.id)?.as_ref()
                    == Some(&queue),
            "OBJECT_PUBLICATION_HISTORY_MISMATCH"
        );
    }
    Ok(saved)
}

pub(crate) fn published_task(db: &Connection, project: &str, task: &str) -> Result<bool> {
    Ok(published(db, project, task)?.is_some())
}

pub(crate) fn retained_attempt(db: &Connection, attempt: &Attempt) -> Result<Option<View>> {
    let Some(saved) = published(
        db,
        &attempt.preparation.project_id,
        &attempt.preparation.medium.id,
    )?
    else {
        return Ok(None);
    };
    ensure!(
        saved.source.records.attempt.as_ref() == Some(attempt),
        "OBJECT_PUBLICATION_HISTORY_MISMATCH"
    );
    Ok(Some(View {
        project_id: attempt.preparation.project_id.clone(),
        target: Target::from_record(attempt),
        task_revision: saved.operation.result.unwrap().task_revision,
        state: attempt.state.clone(),
        output_captured: attempt.output.is_some(),
        error: attempt.error.clone(),
    }))
}
