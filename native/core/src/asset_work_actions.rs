use crate::{
    asset_task::{self, State},
    asset_work::{self, Attempt, Subtask},
    asset_work_contract::{Action, Definition, Outcome},
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

fn text(value: &str, max: usize) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= max,
        "Supply nonempty text within the field size limit"
    );
    Ok(())
}

fn definition(value: &Definition) -> Result<()> {
    text(&value.title, 300)?;
    text(&value.goal, 12000)?;
    text(&value.acceptance, 12000)
}

fn editable<'a>(state: &'a mut State, stage: &str, id: &str) -> Result<&'a mut Subtask> {
    let item = state
        .work
        .subtasks
        .iter_mut()
        .find(|s| s.id == id && s.stage_id == stage)
        .context("Unknown stage subtask")?;
    ensure!(
        !["running", "completed", "cancelled"].contains(&item.status.as_str()),
        "Running, completed or cancelled subtasks cannot be revised or cancelled"
    );
    Ok(item)
}

pub fn apply(
    state: &mut State,
    revision: u64,
    stage: &str,
    thread: &str,
    turn: &str,
    action: &Action,
    captured: Option<&[crate::asset_work_inputs::Captured]>,
) -> Result<Value> {
    asset_work::current(state, revision, stage)?;
    let result = match action {
        Action::Create { definition: spec } => {
            definition(spec)?;
            ensure!(state.work.subtasks.len() < 100, "Subtask limit reached");
            let id = uuid::Uuid::new_v4().to_string();
            state.work.subtasks.push(Subtask {
                id: id.clone(),
                stage_id: stage.into(),
                definition: spec.clone(),
                status: "pending".into(),
                last_attempt_id: None,
                note: String::new(),
            });
            json!({"subtaskId":id})
        }
        Action::Revise {
            subtask_id,
            definition: spec,
        } => {
            definition(spec)?;
            editable(state, stage, subtask_id)?.definition = spec.clone();
            json!({"subtaskId":subtask_id})
        }
        Action::Cancel { subtask_id, reason } => {
            text(reason, 12000)?;
            let item = editable(state, stage, subtask_id)?;
            item.status = "cancelled".into();
            item.note = reason.clone();
            json!({"subtaskId":subtask_id})
        }
        Action::Reopen { subtask_id, reason } => {
            text(reason, 12000)?;
            ensure!(
                !state.work.attempts.iter().any(|a| a.status == "running"),
                "Finish or interrupt the running attempt before reopening work"
            );
            let index = state
                .work
                .subtasks
                .iter()
                .position(|s| s.id == *subtask_id && s.stage_id == stage)
                .context("Unknown stage subtask")?;
            ensure!(
                state.work.subtasks[index].status == "completed",
                "Only completed work may be reopened"
            );
            for item in state
                .work
                .subtasks
                .iter_mut()
                .skip(index)
                .filter(|s| s.stage_id == stage && s.status != "cancelled")
            {
                item.status = "stale".into();
                item.note = reason.clone();
            }
            json!({"subtaskId":subtask_id})
        }
        Action::Begin {
            subtask_id,
            inputs,
            recovery_note,
            ..
        } => begin(
            state,
            stage,
            subtask_id,
            thread,
            turn,
            inputs,
            recovery_note,
            captured.context("Work input capture requires the asynchronous callback adapter")?,
        )?,
        Action::Finish {
            attempt_id,
            outcome,
            summary,
            outputs,
            tools,
        } => {
            text(summary, 12000)?;
            ensure!(
                outputs.is_object() && tools.len() <= 100 && tools.iter().all(Value::is_object),
                "Outputs must be an object; tools must contain at most 100 objects"
            );
            let attempt = state
                .work
                .attempts
                .iter_mut()
                .find(|a| a.id == *attempt_id && a.stage_id == stage)
                .context("Unknown stage attempt")?;
            ensure!(
                attempt.status == "running"
                    && attempt.thread_id == thread
                    && attempt.turn_id == turn,
                "Attempt does not belong to this active execution; inspect before retrying"
            );
            let item = state
                .work
                .subtasks
                .iter_mut()
                .find(|s| s.id == attempt.subtask_id)
                .context("Missing subtask")?;
            ensure!(
                item.status == "running" && item.last_attempt_id.as_deref() == Some(attempt_id),
                "Subtask attempt changed"
            );
            attempt.status = match outcome {
                Outcome::Completed => "completed",
                Outcome::Failed => "failed",
            }
            .into();
            attempt.summary = summary.clone();
            attempt.outputs = outputs.clone();
            attempt.tools = tools.clone();
            attempt.ended_at = Some(asset_task::now());
            attempt.end_checkpoint = state.checkpoint.clone();
            item.status = attempt.status.clone();
            item.note = summary.clone();
            json!({"subtaskId":item.id,"attemptId":attempt_id})
        }
    };
    state.revision += 1;
    Ok(result)
}

fn begin(
    state: &mut State,
    stage: &str,
    id: &str,
    thread: &str,
    turn: &str,
    inputs: &Value,
    recovery_note: &str,
    captured: &[crate::asset_work_inputs::Captured],
) -> Result<Value> {
    ensure!(
        inputs.is_object() && recovery_note.len() <= 12000,
        "Inputs must be an object; recoveryNote exceeds limit"
    );
    ensure!(state.work.attempts.len() < 200, "Attempt limit reached");
    ensure!(
        !state.work.attempts.iter().any(|a| a.status == "running"),
        "Only one subtask may execute in this asset workspace"
    );
    let item = state
        .work
        .subtasks
        .iter_mut()
        .find(|s| s.stage_id == stage && !["completed", "cancelled"].contains(&s.status.as_str()))
        .context("No runnable stage subtask")?;
    ensure!(
        item.id == id,
        "Execute stage subtasks in their registered order"
    );
    if ["failed", "interrupted", "stale"].contains(&item.status.as_str()) {
        text(recovery_note, 12000)?;
    }
    let attempt = Attempt {
        id: uuid::Uuid::new_v4().to_string(),
        subtask_id: id.into(),
        stage_id: stage.into(),
        definition: item.definition.clone(),
        status: "running".into(),
        thread_id: thread.into(),
        turn_id: turn.into(),
        session_id: state.session_id.clone(),
        checkpoint: state.checkpoint.clone(),
        end_checkpoint: None,
        asset_revision: state.revision,
        input_candidates: state
            .delivery
            .as_ref()
            .context("Missing workflow")?
            .approved
            .clone(),
        inputs: inputs.clone(),
        input_files: Some(captured.to_vec()),
        outputs: json!({}),
        tools: vec![],
        summary: String::new(),
        recovery_note: recovery_note.into(),
        started_at: asset_task::now(),
        ended_at: None,
    };
    let result = json!({"subtaskId":id,"attemptId":attempt.id});
    item.status = "running".into();
    item.last_attempt_id = Some(attempt.id.clone());
    state.work.attempts.push(attempt);
    Ok(result)
}
