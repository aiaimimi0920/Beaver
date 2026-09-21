use crate::{asset_task::State, asset_work_contract::Definition};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub subtasks: Vec<Subtask>,
    pub attempts: Vec<Attempt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub stage_id: String,
    pub definition: Definition,
    pub status: String,
    pub last_attempt_id: Option<String>,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub id: String,
    pub subtask_id: String,
    pub stage_id: String,
    pub definition: Definition,
    pub status: String,
    pub thread_id: String,
    pub turn_id: String,
    pub session_id: Option<String>,
    pub checkpoint: Option<String>,
    pub end_checkpoint: Option<String>,
    pub asset_revision: u64,
    pub input_candidates: Vec<String>,
    #[serde(default)]
    pub input_files: Option<Vec<crate::asset_work_inputs::Captured>>,
    pub inputs: Value,
    pub outputs: Value,
    pub tools: Vec<Value>,
    pub summary: String,
    pub recovery_note: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

pub fn current(state: &State, revision: u64, stage: &str) -> Result<()> {
    ensure!(state.revision == revision, "Asset revision changed");
    let flow = state
        .delivery
        .as_ref()
        .context("Work requires a delivery template")?;
    ensure!(flow.pending.is_none(), "Work is paused for owner review");
    ensure!(
        state
            .stages
            .get(flow.approved.len())
            .is_some_and(|s| s.id == stage),
        "Only the current linear stage may execute work"
    );
    Ok(())
}

/// Completed here means reported by Codex; candidate approval remains a separate gate.
pub fn completed_attempts(state: &State, stage: &str) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for item in state.work.subtasks.iter().filter(|s| s.stage_id == stage) {
        if item.status == "cancelled" {
            continue;
        }
        ensure!(
            item.status == "completed",
            "Finish or explicitly cancel all stage subtasks before submitting"
        );
        let id = item
            .last_attempt_id
            .as_ref()
            .context("Subtask has no execution evidence")?;
        let attempt = state
            .work
            .attempts
            .iter()
            .find(|a| &a.id == id)
            .context("Missing execution attempt")?;
        ensure!(
            attempt.status == "completed"
                && state
                    .delivery
                    .as_ref()
                    .is_some_and(|f| f.approved == attempt.input_candidates),
            "Subtask inputs changed; inspect and execute again"
        );
        ids.push(id.clone());
    }
    Ok(ids)
}

/// Called at existing interruption/restart boundaries, never replaying scene operations.
pub fn interrupt(state: &mut State, reason: &str) -> bool {
    let mut changed = false;
    for attempt in &mut state.work.attempts {
        if attempt.status != "running" {
            continue;
        }
        attempt.status = "interrupted".into();
        attempt.ended_at = Some(crate::asset_task::now());
        attempt.summary = reason.into();
        if let Some(item) = state
            .work
            .subtasks
            .iter_mut()
            .find(|s| s.id == attempt.subtask_id)
        {
            item.status = "interrupted".into();
            item.note = reason.into();
        }
        changed = true;
    }
    if changed {
        state.revision += 1;
    }
    changed
}

pub fn invalidate(state: &mut State, index: usize, reason: &str) {
    interrupt(state, reason);
    for item in &mut state.work.subtasks {
        if item.status != "cancelled"
            && state
                .stages
                .iter()
                .skip(index)
                .any(|s| s.id == item.stage_id)
        {
            item.status = "stale".into();
            item.note = reason.into();
        }
    }
}

/// Turn boundaries retire unfinished attempts, even without an attached Blender session.
pub fn end_turn(store: &crate::store::Store, id: &str) -> Result<()> {
    if let Some(mut state) = store.get::<State>("asset-task", id)? {
        if interrupt(
            &mut state,
            "Execution turn ended; inspect saved files and scene before retrying",
        ) {
            crate::asset_task::save(store, &state)?;
        }
    }
    Ok(())
}
