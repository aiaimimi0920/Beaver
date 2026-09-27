//! Snapshot all durable inputs under one transaction; file access belongs to the verifier.
use super::Target;
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_control::{self, Pending},
    object_attempt_view,
    object_catalog::{self, ObjectRecord},
    object_command_receipt,
    object_run_preparation::{self, Preparation, PreparationState},
    object_task_dispatch::{self, Control},
    object_task_queue::{self, claim, QueueEntry},
    object_task_storage::{self as storage, TASK_KIND},
    object_task_types::{RunRecord, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Records {
    pub object: ObjectRecord,
    pub accepted_version_id: Option<String>,
    pub medium: TaskRecord,
    pub run: RunRecord,
    pub queue: QueueEntry,
    pub preparation: Preparation,
    pub control: Control,
    pub attempt: Option<Attempt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<Attempt>,
    pub fine: Option<TaskRecord>,
    pub interrupts: Vec<Pending>,
    pub baseline_sources: Vec<ObjectRecord>,
}

impl Records {
    pub(super) fn target(&self, generation: u64) -> Target {
        Target {
            task_id: self.medium.id.clone(),
            object_id: self.object.id.clone(),
            run_id: self.run.id.clone(),
            owner: self.preparation.owner.clone(),
            claim_token: self.preparation.claim_token.clone(),
            writer_generation: self.preparation.generation,
            task_revision: self.medium.revision,
            run_revision: self.run.revision,
            object_revision: self.object.revision,
            control_revision: self.control.revision,
            recovery_generation: generation,
        }
    }
}

pub(super) fn read(connection: &Connection, project: &str, task: &str) -> Result<Option<Records>> {
    let (_, run) = claim::read_medium(connection, project, task)?;
    let Some(preparation) = object_run_preparation::read_record(connection, project, &run.id)?
    else {
        return Ok(None);
    };
    let (queue, medium, run) = claim::owned_claim(
        connection,
        project,
        task,
        &preparation.owner,
        &preparation.claim_token,
        preparation.generation,
    )?;
    let object = object_catalog::read(connection, &run.object_id)?.context("OBJECT_NOT_FOUND")?;
    ensure!(object.project_id == project, "OBJECT_TASK_PROJECT_MISMATCH");
    let mut owners = 0;
    for entry in object_task_queue::list_in(connection, project)?
        .iter()
        .filter(|entry| entry.holds_object())
    {
        let (owner, owned_run) = claim::read_medium(connection, project, &entry.task_id)?;
        claim::validate_state(entry, &owner, &owned_run)?;
        if owned_run.object_id == object.id {
            owners += 1;
        }
    }
    ensure!(owners == 1, "OBJECT_TASK_QUEUE_MULTIPLE_OWNERS");
    let control = object_task_dispatch::read_in(connection, &medium, &run)?;
    let accepted_version_id = object_command_receipt::latest_accepted(connection, &object)?;
    let mut baseline_sources = Vec::new();
    if let Some(baseline) = &preparation.baseline {
        let ids: std::collections::BTreeSet<_> = baseline
            .versions
            .iter()
            .map(|version| &version.object_id)
            .collect();
        for id in ids {
            let source = object_catalog::read(connection, id)?
                .context("OBJECT_RECOVERY_BASELINE_OBJECT_MISSING")?;
            ensure!(source.project_id == project, "OBJECT_TASK_PROJECT_MISMATCH");
            baseline_sources.push(source);
        }
    }
    let mut statement = connection.prepare("SELECT id FROM entities WHERE kind=? AND json_extract(value,'$.preparation.run.id')=? ORDER BY id")?;
    let ids = statement
        .query_map(
            rusqlite::params![crate::object_attempt::KIND, &run.id],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let attempts = ids
        .iter()
        .map(|id| object_attempt_view::read(connection, project, id))
        .collect::<Result<Vec<_>>>()?;
    let mut history = super::resume::chain(connection, attempts)?;
    let attempt = history.pop();
    let (fine, interrupts) = if let Some(attempt) = &attempt {
        ensure!(
            attempt.preparation == preparation && preparation.state == PreparationState::Ready,
            "OBJECT_RUN_STALE_PREPARATION"
        );
        object_attempt_view::view(connection, attempt)?;
        ensure!(
            (attempt.state == State::Running) == attempt.output.is_none(),
            "OBJECT_ATTEMPT_STATE_MISMATCH"
        );
        let fine: TaskRecord = storage::read(connection, TASK_KIND, &attempt.fine.id)?
            .context("OBJECT_ATTEMPT_IDENTITY_MISMATCH")?;
        let mut expected_medium = preparation.medium.clone();
        let mut expected_run = preparation.run.clone();
        let mut expected_fine = attempt.fine.clone();
        let delta = if attempt.state == State::Running {
            1
        } else {
            2
        };
        expected_medium.status = medium.status.clone();
        expected_run.status = run.status.clone();
        expected_fine.status = fine.status.clone();
        expected_medium.revision = expected_medium
            .revision
            .checked_add(delta + 2 * history.len() as u64)
            .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
        expected_run.revision = expected_run
            .revision
            .checked_add(delta + 2 * history.len() as u64)
            .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
        expected_fine.revision = expected_fine
            .revision
            .checked_add(delta)
            .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
        ensure!(
            medium == expected_medium && run == expected_run && fine == expected_fine,
            "OBJECT_RECOVERY_ATTEMPT_REVISION_MISMATCH"
        );
        (
            Some(fine),
            object_attempt_control::for_recovery(connection, attempt)?,
        )
    } else {
        let mut expected_medium = preparation.medium.clone();
        let mut expected_run = preparation.run.clone();
        if preparation.state == PreparationState::Failed {
            expected_medium.status = "failed".into();
            expected_run.status = "failed".into();
            expected_medium.revision = expected_medium
                .revision
                .checked_add(1)
                .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
            expected_run.revision = expected_run
                .revision
                .checked_add(1)
                .context("OBJECT_TASK_REVISION_EXHAUSTED")?;
        }
        ensure!(
            medium == expected_medium && run == expected_run,
            "OBJECT_RUN_PREPARATION_REVISION_CONFLICT"
        );
        (None, vec![])
    };
    Ok(Some(Records {
        object,
        accepted_version_id,
        medium,
        run,
        queue,
        preparation,
        control,
        attempt,
        history,
        fine,
        interrupts,
        baseline_sources,
    }))
}
