//! Transactional attempt identity, serial fine selection and state transitions.
use super::{Attempt, Lease, State, KIND};
use crate::{
    files::Snapshot,
    object_framework::{Identity, VERSION},
    object_run_baseline,
    object_run_preparation::{self, Preparation, PreparationState},
    object_task_queue::{claim, QUEUE_KIND},
    object_task_storage::{self as store, RUN_KIND, TASK_KIND},
    object_task_types::{Granularity, TaskRecord, MAX_REVISION},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

pub(super) fn start(connection: &Connection, preparation: Preparation) -> Result<Option<Lease>> {
    object_run_preparation::require_ready(connection, &preparation)?;
    let mut fines: Vec<_> = store::all_tasks(connection)?
        .into_iter()
        .filter(|task| {
            task.parent_task_id.as_deref() == Some(&preparation.medium.id)
                && task.granularity == Granularity::Fine
                && task.status != "cancelled"
        })
        .collect();
    fines.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
    let Some(fine) = fines.into_iter().next() else {
        return Ok(None);
    };
    ensure!(
        fine.project_id == preparation.project_id
            && fine.object_id == preparation.medium.object_id
            && fine.run_id.as_deref() == Some(&preparation.run.id)
            && matches!(&fine.identity, Identity::Fine { schema_version, object_id, medium_task_id, run_id, stage_id }
            if *schema_version == VERSION && object_id == &preparation.run.object_id
                && medium_task_id == &preparation.medium.id && run_id == &preparation.run.id
                && fine.stage_id.as_deref() == Some(stage_id.as_str())),
        "OBJECT_ATTEMPT_IDENTITY_MISMATCH"
    );
    ensure!(
        fine.status == "planned",
        "OBJECT_ATTEMPT_CHECKPOINT_REQUIRED"
    );
    for id in &fine.depends_on {
        let dependency = store::read::<TaskRecord>(connection, TASK_KIND, id)?;
        if !dependency.is_some_and(|task| {
            task.id == *id && task.project_id == fine.project_id && task.status == "accepted"
        }) {
            return Ok(None);
        }
    }
    let previous: Vec<Attempt> = store::read_all(connection, KIND)?;
    ensure!(
        !previous
            .iter()
            .any(|record| record.preparation.run.id == preparation.run.id),
        "OBJECT_ATTEMPT_ALREADY_STARTED"
    );
    let input = object_run_baseline::snapshot(
        &preparation
            .baseline
            .as_ref()
            .context("OBJECT_RUN_BASELINE_MISSING")?
            .versions,
    )?;
    let record = Attempt {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        preparation,
        fine: fine.clone(),
        input,
        state: State::Running,
        thread_id: None,
        turn_id: None,
        output: None,
        error: None,
    };
    let mut lease = Lease {
        medium: record.preparation.medium.clone(),
        run: record.preparation.run.clone(),
        fine,
        record,
    };
    transition(connection, &mut lease, "running", "running")?;
    store::insert(connection, KIND, &lease.record.id, &lease.record)?;
    Ok(Some(lease))
}

pub(super) fn current(connection: &Connection, lease: &Lease) -> Result<()> {
    let record = &lease.record;
    let preparation = &record.preparation;
    ensure!(
        record.state == State::Running
            && store::read::<Attempt>(connection, KIND, &record.id)?.as_ref() == Some(record),
        "OBJECT_ATTEMPT_STALE_CALLBACK"
    );
    ensure!(
        object_run_preparation::read_record(connection, &preparation.project_id, &preparation.id)?
            .as_ref()
            == Some(preparation)
            && preparation.state == PreparationState::Ready,
        "OBJECT_RUN_STALE_PREPARATION"
    );
    let (entry, medium, run) = claim::owned_claim(
        connection,
        &preparation.project_id,
        &preparation.medium.id,
        &preparation.owner,
        &preparation.claim_token,
        preparation.generation,
    )?;
    ensure!(
        entry.state == "running"
            && medium == lease.medium
            && run == lease.run
            && store::read::<TaskRecord>(connection, TASK_KIND, &lease.fine.id)?.as_ref()
                == Some(&lease.fine),
        "OBJECT_ATTEMPT_STALE_CALLBACK"
    );
    Ok(())
}

pub(super) fn finish(
    connection: &Connection,
    mut lease: Lease,
    state: State,
    error: Option<String>,
    output: Snapshot,
) -> Result<()> {
    current(connection, &lease)?;
    let status = if state == State::AwaitingGate {
        "awaitingAcceptance"
    } else {
        "failed"
    };
    transition(connection, &mut lease, status, status)?;
    lease.record.state = state;
    lease.record.error = error;
    lease.record.output = Some(output);
    store::replace(connection, KIND, &lease.record.id, &lease.record)?;
    crate::object_attempt_control::complete(connection, &lease.record)
}

pub(super) fn transition(
    connection: &Connection,
    lease: &mut Lease,
    queue_state: &str,
    status: &str,
) -> Result<()> {
    let preparation = &lease.record.preparation;
    let (mut entry, _, _) = claim::owned_claim(
        connection,
        &preparation.project_id,
        &preparation.medium.id,
        &preparation.owner,
        &preparation.claim_token,
        preparation.generation,
    )?;
    ensure!(
        lease.medium.revision < MAX_REVISION
            && lease.run.revision < MAX_REVISION
            && lease.fine.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    entry.state = queue_state.into();
    lease.medium.status = status.into();
    lease.run.status = status.into();
    lease.fine.status = status.into();
    lease.medium.revision += 1;
    lease.run.revision += 1;
    lease.fine.revision += 1;
    store::replace(connection, QUEUE_KIND, &entry.id, &entry)?;
    store::replace(connection, TASK_KIND, &lease.medium.id, &lease.medium)?;
    store::replace(connection, RUN_KIND, &lease.run.id, &lease.run)?;
    store::replace(connection, TASK_KIND, &lease.fine.id, &lease.fine)
}
