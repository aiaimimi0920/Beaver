//! The sole acceptance transaction for object, stages, queue and command history.
use super::{Operation, Published, State, Stored};
use crate::{
    object_catalog::{self, ObjectRecord, ObjectVersion},
    object_command_receipt::{Command, MAX_REVISION},
    object_task_queue::{QueueEntry, QUEUE_KIND},
    object_task_storage::{self as store, RUN_KIND, STATE_KIND, TASK_KIND},
    object_task_types::{RunRecord, TaskRecord},
    object_version_acceptance::AcceptanceRequest,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

fn increment(value: u64) -> Result<u64> {
    value
        .checked_add(1)
        .filter(|value| *value <= MAX_REVISION)
        .context("OBJECT_PUBLICATION_REVISION_EXHAUSTED")
}

pub(super) fn acceptance(saved: &Stored) -> AcceptanceRequest {
    let request = &saved.operation.request;
    AcceptanceRequest {
        project_id: request.project_id.clone(),
        request_id: request.request_id.clone(),
        object_id: saved.source.records.object.id.clone(),
        version_id: saved.operation.version_id.clone(),
        expected_revision: saved.source.records.object.revision,
    }
}

pub(super) fn command(runtime: &ProjectRuntime, saved: &Stored) -> Result<Command> {
    let request = acceptance(saved);
    Command::new(
        runtime,
        &request.project_id,
        &request.request_id,
        "accept",
        &request,
    )
}

pub(super) fn object(saved: &Stored) -> Result<ObjectRecord> {
    let mut object = saved.source.records.object.clone();
    object.revision = increment(object.revision)?;
    object.files = saved.operation.preview.files.clone();
    object.versions.push(ObjectVersion {
        version_id: saved.operation.version_id.clone(),
        manifest: serde_json::to_value(&saved.manifest)?,
    });
    Ok(object)
}

pub(super) fn records(saved: &Stored) -> Result<(TaskRecord, TaskRecord, RunRecord, QueueEntry)> {
    let source = &saved.source.records;
    let fine = super::super::super::resume::advance::accepted(
        source
            .attempt
            .as_ref()
            .context("OBJECT_ATTEMPT_NOT_FOUND")?,
    )?;
    ensure!(
        fine.revision <= MAX_REVISION,
        "OBJECT_PUBLICATION_REVISION_EXHAUSTED"
    );
    let mut medium = source.medium.clone();
    let mut run = source.run.clone();
    let mut queue = source.queue.clone();
    medium.status = "accepted".into();
    medium.revision = increment(medium.revision)?;
    run.status = "accepted".into();
    run.revision = increment(run.revision)?;
    queue.state = "accepted".into();
    queue.owner = None;
    queue.claim_token = None;
    Ok((fine, medium, run, queue))
}

pub(super) fn validate_headroom(db: &Connection, saved: &Stored) -> Result<()> {
    object(saved)?;
    records(saved)?;
    let plan = store::plan_state(db, &saved.operation.request.project_id)?;
    ensure!(
        plan.project_id == saved.operation.request.project_id,
        "OBJECT_TASK_PROJECT_MISMATCH"
    );
    let mut revision = increment(plan.revision)?;
    for _ in saved
        .operation
        .preview
        .feedback
        .iter()
        .filter(|f| f.later.is_some())
    {
        revision = increment(revision)?;
    }
    Ok(())
}

pub(super) fn commit(
    runtime: &ProjectRuntime,
    db: &Connection,
    mut saved: Stored,
) -> Result<Operation> {
    let command = command(runtime, &saved)?;
    ensure!(command.replay(db)?.is_none(), "OBJECT_REQUEST_CONFLICT");
    let object = object(&saved)?;
    let (fine, medium, run, queue) = records(&saved)?;
    let mut plan = store::plan_state(db, &object.project_id)?;
    plan.revision = increment(plan.revision)?;
    object_catalog::validate_write(db, &object)?;
    let version = object.versions.last().context("OBJECT_VERSION_NOT_FOUND")?;
    store::insert(
        db,
        "object_version",
        &version.version_id,
        &(&object.id, version),
    )?;
    object_catalog::update_projection(db, &object)?;
    store::replace(db, TASK_KIND, &fine.id, &fine)?;
    store::replace(db, TASK_KIND, &medium.id, &medium)?;
    store::replace(db, RUN_KIND, &run.id, &run)?;
    store::replace(db, QUEUE_KIND, &queue.id, &queue)?;
    store::replace(db, STATE_KIND, &plan.project_id, &plan)?;
    command.save(db, object.clone(), Some(version.version_id.clone()))?;
    super::deferred::commit(db, &saved.operation)?;
    plan = store::plan_state(db, &object.project_id)?;
    saved.operation.state = State::Published;
    saved.operation.error = None;
    saved.operation.result = Some(Published {
        version_id: version.version_id.clone(),
        object_revision: object.revision,
        task_revision: medium.revision,
        run_revision: run.revision,
        plan_revision: plan.revision,
    });
    super::storage::save(db, &saved)?;
    Ok(saved.operation)
}
