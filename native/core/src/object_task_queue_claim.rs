use super::{read_entry, Claim, QueueEntry, QUEUE_KIND};
use crate::{
    object_framework::{Identity, VERSION},
    object_task_cancel,
    object_task_storage::{self, RUN_KIND, STATE_KIND, TASK_KIND},
    object_task_types::{valid_id, Granularity, RunRecord, TaskRecord, MAX_REVISION},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

pub(crate) fn read_medium(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<(TaskRecord, RunRecord)> {
    let task = object_task_storage::read::<TaskRecord>(connection, TASK_KIND, task_id)?
        .context("OBJECT_TASK_NOT_FOUND")?;
    ensure!(task.id == task_id, "OBJECT_TASK_IDENTITY_MISMATCH");
    ensure!(
        task.project_id == project_id,
        "OBJECT_TASK_PROJECT_MISMATCH"
    );
    ensure!(
        task.granularity == Granularity::Medium,
        "OBJECT_TASK_QUEUE_REQUIRES_MEDIUM"
    );
    ensure!(
        matches!(&task.identity, Identity::Medium { schema_version, object_id, .. }
            if *schema_version == VERSION && valid_id(object_id)
                && task.object_id.as_deref() == Some(object_id.as_str()))
            && task.stage_id.is_none(),
        "OBJECT_TASK_IDENTITY_MISMATCH"
    );
    let run_id = task
        .run_id
        .as_deref()
        .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
    let run = object_task_storage::read::<RunRecord>(connection, RUN_KIND, run_id)?
        .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
    ensure!(
        run.id == run_id
            && run.project_id == project_id
            && run.medium_task_id == task.id
            && Some(run.object_id.as_str()) == task.object_id.as_deref(),
        "OBJECT_TASK_MEDIUM_RUN_MISMATCH"
    );
    Ok((task, run))
}

pub(crate) fn validate_state(entry: &QueueEntry, task: &TaskRecord, run: &RunRecord) -> Result<()> {
    ensure!(
        entry.task_status()? == task.status && run.status == task.status,
        "OBJECT_TASK_QUEUE_STATE_MISMATCH"
    );
    if entry.holds_object() {
        ensure!(
            entry
                .owner
                .as_ref()
                .is_some_and(|owner| !owner.trim().is_empty())
                && entry
                    .claim_token
                    .as_ref()
                    .is_some_and(|token| !token.is_empty())
                && entry.generation > 0,
            "OBJECT_TASK_STALE_CLAIM"
        );
    }
    Ok(())
}

pub(super) fn begin_claim(
    connection: &Connection,
    mut entry: QueueEntry,
    mut task: TaskRecord,
    mut run: RunRecord,
    owner: &str,
) -> Result<Claim> {
    ensure!(
        task.revision < MAX_REVISION && run.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    ensure!(
        entry.generation < MAX_REVISION,
        "OBJECT_TASK_QUEUE_GENERATION_EXHAUSTED"
    );
    let token = uuid::Uuid::new_v4().to_string();
    entry.state = "claimed".into();
    entry.claim_token = Some(token.clone());
    entry.owner = Some(owner.to_owned());
    entry.generation += 1;
    task.status = "queued".into();
    task.revision += 1;
    run.status = "queued".into();
    run.revision += 1;
    object_task_storage::replace(connection, QUEUE_KIND, &entry.id, &entry)?;
    object_task_storage::replace(connection, TASK_KIND, &task.id, &task)?;
    object_task_storage::replace(connection, RUN_KIND, &run.id, &run)?;
    Ok(Claim {
        task,
        claim_token: token,
        generation: entry.generation,
    })
}

pub(crate) fn owned_claim(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
) -> Result<(QueueEntry, TaskRecord, RunRecord)> {
    let entry = read_entry(connection, project_id, task_id)?
        .context("OBJECT_TASK_QUEUE_ENTRY_NOT_FOUND")?;
    ensure!(
        entry.holds_object()
            && entry.owner.as_deref() == Some(owner)
            && entry.claim_token.as_deref() == Some(claim_token)
            && entry.generation == generation,
        "OBJECT_TASK_STALE_CLAIM"
    );
    let (task, run) = read_medium(connection, project_id, task_id)?;
    validate_state(&entry, &task, &run)?;
    Ok((entry, task, run))
}

pub(super) fn finish_claim_in(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
    success: bool,
) -> Result<TaskRecord> {
    let (mut entry, mut task, mut run) = owned_claim(
        connection,
        project_id,
        task_id,
        owner,
        claim_token,
        generation,
    )?;
    ensure!(entry.state == "claimed", "OBJECT_TASK_STALE_CLAIM");
    crate::object_run_preparation::require_unprepared(connection, &run.id)?;
    ensure!(
        task.revision < MAX_REVISION && run.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    let status = if success {
        "awaitingAcceptance"
    } else {
        "failed"
    };
    entry.state = status.into();
    task.status = status.into();
    task.revision += 1;
    run.status = status.into();
    run.revision += 1;
    object_task_storage::replace(connection, QUEUE_KIND, &entry.id, &entry)?;
    object_task_storage::replace(connection, TASK_KIND, task_id, &task)?;
    object_task_storage::replace(connection, RUN_KIND, &run.id, &run)?;
    Ok(task)
}

pub(super) fn cancel_claim_in(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
    owner: &str,
    claim_token: &str,
    generation: u64,
) -> Result<TaskRecord> {
    let (_, task, _) = owned_claim(
        connection,
        project_id,
        task_id,
        owner,
        claim_token,
        generation,
    )?;
    let mut state = object_task_storage::plan_state(connection, project_id)?;
    ensure!(
        state.project_id == project_id,
        "OBJECT_TASK_PROJECT_MISMATCH"
    );
    ensure!(
        state.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    let tasks = object_task_cancel::cancel_owned_work(connection, &task)?;
    state.revision += 1;
    object_task_storage::replace(connection, STATE_KIND, project_id, &state)?;
    tasks
        .into_iter()
        .find(|task| task.id == task_id)
        .context("OBJECT_TASK_NOT_FOUND")
}
