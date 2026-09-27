//! Transaction-only preparation log and claim revalidation; no file operations.
use super::{Preparation, PreparationState};
use crate::{
    object_run_baseline,
    object_task_queue::{self, claim, QUEUE_KIND},
    object_task_storage::{self, RUN_KIND, TASK_KIND},
    object_task_types::MAX_REVISION,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

const KIND: &str = "object_run_preparation";

pub(crate) fn require_unprepared(connection: &Connection, run_id: &str) -> Result<()> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM entities WHERE kind=? AND id=?)",
        [KIND, run_id],
        |row| row.get(0),
    )?;
    ensure!(!exists, "OBJECT_RUN_DISPOSITION_REQUIRED");
    Ok(())
}

pub(crate) fn read(
    connection: &Connection,
    project_id: &str,
    run_id: &str,
) -> Result<Option<Preparation>> {
    let record = object_task_storage::read::<Preparation>(connection, KIND, run_id)?;
    if let Some(record) = &record {
        ensure!(
            record.schema_version == 1
                && record.id == run_id
                && record.run.id == run_id
                && record.project_id == project_id
                && record.run.project_id == project_id
                && record.medium.project_id == project_id
                && record.medium.id == record.run.medium_task_id
                && record.medium.run_id.as_deref() == Some(run_id)
                && record.medium.object_id.as_deref() == Some(record.run.object_id.as_str()),
            "OBJECT_RUN_PREPARATION_IDENTITY_MISMATCH"
        );
    }
    Ok(record)
}

pub(super) fn begin(
    connection: &Connection,
    project_id: &str,
    owner: &str,
) -> Result<Option<Preparation>> {
    let Some(claimed) = object_task_queue::claim_next_in(connection, project_id, owner)? else {
        return Ok(None);
    };
    let (medium, mut run) = claim::read_medium(connection, project_id, &claimed.task.id)?;
    ensure!(
        !run.id.is_empty()
            && run
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)),
        "OBJECT_RUN_INVALID_WORKSPACE_ID"
    );
    require_unprepared(connection, &run.id)?;
    ensure!(
        run.baseline_version_id.is_none(),
        "OBJECT_RUN_BASELINE_ALREADY_BOUND"
    );
    let baseline = object_run_baseline::resolve(connection, &medium);
    if let Ok(baseline) = &baseline {
        run.baseline_version_id = baseline.resolved_version_id.clone();
        object_task_storage::replace(connection, RUN_KIND, &run.id, &run)?;
    }
    let mut record = Preparation {
        schema_version: 1,
        id: run.id.clone(),
        project_id: project_id.into(),
        owner: owner.into(),
        claim_token: claimed.claim_token,
        generation: claimed.generation,
        workspace: format!(".beaver/workspaces/{}", run.id),
        medium,
        run,
        baseline: None,
        state: PreparationState::Pending,
        error: None,
    };
    match baseline {
        Ok(baseline) => record.baseline = Some(baseline),
        Err(error) => {
            fail_claim(connection, &record)?;
            record.state = PreparationState::Failed;
            record.error = Some(format!("{error:#}"));
        }
    }
    object_task_storage::insert(connection, KIND, &record.id, &record)?;
    Ok(Some(record))
}

fn current(connection: &Connection, record: &Preparation) -> Result<()> {
    ensure!(
        read(connection, &record.project_id, &record.id)?.as_ref() == Some(record),
        "OBJECT_RUN_STALE_PREPARATION"
    );
    let (entry, medium, run) = claim::owned_claim(
        connection,
        &record.project_id,
        &record.medium.id,
        &record.owner,
        &record.claim_token,
        record.generation,
    )?;
    ensure!(entry.state == "claimed", "OBJECT_TASK_STALE_CLAIM");
    ensure!(
        medium == record.medium && run == record.run,
        "OBJECT_RUN_PREPARATION_REVISION_CONFLICT"
    );
    Ok(())
}

pub(crate) fn require_ready(connection: &Connection, record: &Preparation) -> Result<()> {
    ensure!(
        record.state == PreparationState::Ready,
        "OBJECT_RUN_NOT_READY"
    );
    current(connection, record)
}

pub(super) fn start(connection: &Connection, record: &mut Preparation) -> Result<()> {
    ensure!(
        record.state == PreparationState::Pending,
        "OBJECT_RUN_STALE_PREPARATION"
    );
    current(connection, record)?;
    record.state = PreparationState::Preparing;
    object_task_storage::replace(connection, KIND, &record.id, record)
}

pub(super) fn finish(
    connection: &Connection,
    mut record: Preparation,
    error: Option<String>,
) -> Result<Preparation> {
    ensure!(
        record.state == PreparationState::Preparing,
        "OBJECT_RUN_STALE_PREPARATION"
    );
    current(connection, &record)?;
    record.state = if error.is_some() {
        fail_claim(connection, &record)?;
        PreparationState::Failed
    } else {
        PreparationState::Ready
    };
    record.error = error;
    object_task_storage::replace(connection, KIND, &record.id, &record)?;
    Ok(record)
}

fn fail_claim(connection: &Connection, record: &Preparation) -> Result<()> {
    let (mut entry, mut medium, mut run) = claim::owned_claim(
        connection,
        &record.project_id,
        &record.medium.id,
        &record.owner,
        &record.claim_token,
        record.generation,
    )?;
    ensure!(entry.state == "claimed", "OBJECT_TASK_STALE_CLAIM");
    ensure!(
        medium == record.medium && run == record.run,
        "OBJECT_RUN_PREPARATION_REVISION_CONFLICT"
    );
    ensure!(
        medium.revision < MAX_REVISION && run.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    entry.state = "failed".into();
    medium.status = "failed".into();
    medium.revision = medium
        .revision
        .checked_add(1)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    run.status = "failed".into();
    run.revision = run
        .revision
        .checked_add(1)
        .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
    object_task_storage::replace(connection, QUEUE_KIND, &entry.id, &entry)?;
    object_task_storage::replace(connection, TASK_KIND, &medium.id, &medium)?;
    object_task_storage::replace(connection, RUN_KIND, &run.id, &run)
}
