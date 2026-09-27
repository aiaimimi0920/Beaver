//! Compact, project-local execution identity for queries and user controls.
use crate::{
    object_attempt::{Attempt, State, KIND},
    object_framework::{Identity, VERSION},
    object_run_preparation::Preparation,
    object_task_queue::claim,
    object_task_storage::{self as storage, TASK_KIND},
    object_task_types::{valid_id, Granularity, TaskRecord},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    pub task_id: String,
    pub fine_task_id: String,
    pub object_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub owner: String,
    pub claim_token: String,
    pub generation: u64,
    // Explicit null identifies an unbound target; omission must not do so.
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub thread_id: Option<String>,
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub turn_id: Option<String>,
}

impl Target {
    pub(crate) fn from_record(record: &Attempt) -> Self {
        let prepared = &record.preparation;
        Self {
            task_id: prepared.medium.id.clone(),
            fine_task_id: record.fine.id.clone(),
            object_id: prepared.run.object_id.clone(),
            run_id: prepared.run.id.clone(),
            attempt_id: record.id.clone(),
            owner: prepared.owner.clone(),
            claim_token: prepared.claim_token.clone(),
            generation: prepared.generation,
            thread_id: record.thread_id.clone(),
            turn_id: record.turn_id.clone(),
        }
    }

    pub(crate) fn matches_writer(&self, project: &str, prepared: &Preparation) -> bool {
        prepared.project_id == project
            && prepared.medium.id == self.task_id
            && prepared.run.id == self.run_id
            && prepared.run.object_id == self.object_id
            && prepared.owner == self.owner
            && prepared.claim_token == self.claim_token
            && prepared.generation == self.generation
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub project_id: String,
    pub target: Target,
    pub task_revision: u64,
    pub state: State,
    pub output_captured: bool,
    pub error: Option<String>,
}

pub(crate) fn read(connection: &Connection, project: &str, id: &str) -> Result<Attempt> {
    let record: Attempt =
        storage::read(connection, KIND, id)?.context("OBJECT_ATTEMPT_NOT_FOUND")?;
    let prepared = &record.preparation;
    let fine = &record.fine;
    ensure!(
        record.id == id
            && record.schema_version == 1
            && prepared.schema_version == 1
            && prepared.project_id == project
            && prepared.medium.project_id == project
            && prepared.run.project_id == project
            && fine.project_id == project
            && prepared.medium.granularity == Granularity::Medium
            && prepared.medium.run_id.as_deref() == Some(prepared.run.id.as_str())
            && prepared.run.medium_task_id == prepared.medium.id
            && prepared.medium.object_id.as_deref() == Some(prepared.run.object_id.as_str())
            && fine.granularity == Granularity::Fine
            && fine.run_id.as_deref() == Some(prepared.run.id.as_str())
            && fine.parent_task_id.as_deref() == Some(prepared.medium.id.as_str())
            && fine.object_id == prepared.medium.object_id
            && matches!(&fine.identity, Identity::Fine { schema_version, object_id, medium_task_id, run_id, stage_id }
                if *schema_version == VERSION && object_id == &prepared.run.object_id
                    && medium_task_id == &prepared.medium.id && run_id == &prepared.run.id
                    && fine.stage_id.as_deref() == Some(stage_id.as_str())),
        "OBJECT_ATTEMPT_IDENTITY_MISMATCH"
    );
    Ok(record)
}

pub(crate) fn view(connection: &Connection, record: &Attempt) -> Result<View> {
    if let Some(view) = crate::object_run_recovery::resume::retained_attempt(connection, record)? {
        return Ok(view);
    }
    if let Some(view) =
        crate::object_run_recovery::candidate::publication::retained_attempt(connection, record)?
    {
        return Ok(view);
    }
    let prepared = &record.preparation;
    let (_, medium, run) = claim::owned_claim(
        connection,
        &prepared.project_id,
        &prepared.medium.id,
        &prepared.owner,
        &prepared.claim_token,
        prepared.generation,
    )?;
    let fine: TaskRecord = storage::read(connection, TASK_KIND, &record.fine.id)?
        .context("OBJECT_ATTEMPT_IDENTITY_MISMATCH")?;
    ensure!(
        run.id == prepared.run.id
            && fine.id == record.fine.id
            && fine.project_id == prepared.project_id
            && fine.identity == record.fine.identity
            && fine.run_id == record.fine.run_id
            && fine.object_id == record.fine.object_id
            && fine.parent_task_id == record.fine.parent_task_id
            && fine.status == medium.status,
        "OBJECT_ATTEMPT_IDENTITY_MISMATCH"
    );
    let status = match record.state {
        State::Running => "running",
        State::AwaitingGate => "awaitingAcceptance",
        State::Failed | State::Interrupted => "failed",
    };
    ensure!(medium.status == status, "OBJECT_ATTEMPT_STATE_MISMATCH");
    Ok(View {
        project_id: prepared.project_id.clone(),
        target: Target::from_record(record),
        task_revision: medium.revision,
        state: record.state.clone(),
        output_captured: record.output.is_some(),
        error: record.error.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub title: String,
    pub prompt: String,
    pub acceptance: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checkpoints {
    pub input: crate::files::Snapshot,
    pub output: Option<crate::files::Snapshot>,
}

pub fn list(runtime: &ProjectRuntime, run_id: &str) -> Result<Vec<View>> {
    Ok(list_with_details(runtime, run_id)?
        .into_iter()
        .map(|(view, _, _)| view)
        .collect())
}

pub(crate) fn list_with_details(
    runtime: &ProjectRuntime,
    run_id: &str,
) -> Result<Vec<(View, Definition, Checkpoints)>> {
    ensure!(valid_id(run_id), "INVALID_OBJECT_RUN_ID");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object attempt store lock poisoned"))?;
    let mut views = Vec::new();
    for (id, record) in store.list_with_ids::<Attempt>(KIND)? {
        if record.preparation.run.id == run_id {
            let record = read(&store.connection, runtime.project_id(), &id)?;
            let checkpoints = Checkpoints {
                input: record.input.clone(),
                output: record.output.clone(),
            };
            // Read the frozen fine, never the current task or an interruption receipt.
            let definition = Definition {
                title: record.fine.title.clone(),
                prompt: record.fine.prompt.clone(),
                acceptance: record.fine.acceptance.clone(),
                revision: record.fine.revision,
            };
            if let Some(history) =
                crate::object_run_recovery::resume::retained_attempt(&store.connection, &record)?
            {
                views.push((history, definition, checkpoints));
                continue;
            }
            views.push((
                match crate::object_run_recovery::disposition::retained_attempt(
                    &store.connection,
                    &record,
                )? {
                    Some(history) => history,
                    None => view(&store.connection, &record)?,
                },
                definition,
                checkpoints,
            ));
        }
    }
    Ok(views)
}
