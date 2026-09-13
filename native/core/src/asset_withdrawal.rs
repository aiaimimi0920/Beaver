use crate::{
    asset_task::{self, Feedback},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

fn cancel_feedback(feedback: &mut Feedback) -> Result<()> {
    if feedback.status == "cancelled" {
        return Ok(());
    }
    if feedback.status != "received" || feedback.delivered_at.is_some() {
        bail!("Only undelivered feedback can be withdrawn; inspect its destination and processing history");
    }
    feedback.status = "cancelled".into();
    feedback
        .history
        .push(json!({"at":asset_task::now(),"status":"cancelled","source":"user"}));
    Ok(())
}

fn pause_empty_followup(store: &Store, state: &asset_task::State) -> Result<()> {
    let Some(mut task) = store.get::<Value>("task", &state.task_id)? else {
        return Ok(());
    };
    if task["status"] == "queued"
        && task["assetFeedbackSeed"].is_object()
        && state.feedback.iter().all(Feedback::terminal)
    {
        task["status"] = json!("interrupted");
        task["error"] = json!("反馈已撤回。后续任务尚未执行；继续需要显式操作。");
        store.put("task", &state.task_id, &task)?;
    }
    Ok(())
}

/// Uses the same Store serialization boundary as receipt and final delivery.
/// A tombstone also rejects a submit that reaches the server after cancellation.
pub fn cancel(store: &Store, task_id: &str, id: &str) -> Result<()> {
    asset_task::validate_id(task_id)?;
    asset_task::validate_id(id)?;
    let mut state = asset_task::get(store, task_id)?;
    if state.withdrawn.iter().any(|value| value == id) {
        return Ok(());
    }
    if state.withdrawn.len() >= asset_task::MAX_FEEDBACK {
        bail!("Withdrawal limit reached; keep the pending submission and inspect its status");
    }
    // A follow-up may exist even if a crash prevented its source receipt being saved.
    let destination = store.list::<Value>("task")?.into_iter().find(|task| {
        task["assetFeedbackSeed"]["sourceTaskId"] == task_id
            && task["assetFeedbackSeed"]["id"] == id
    });
    if let Some(task) = destination {
        let mut target = asset_task::enable(store, &task)?;
        let feedback = target
            .feedback
            .iter_mut()
            .find(|f| f.id == id && f.source_task_id == task_id)
            .context(
                "Follow-up feedback receipt is missing; inspect the destination before retrying",
            )?;
        cancel_feedback(feedback)?;
        // Save destination first. A crash can leave a stale source receipt, never live work.
        asset_task::save(store, &target)?;
        pause_empty_followup(store, &target)?;
        if let Some(source) = state.feedback.iter_mut().find(|f| f.id == id) {
            source.status = "cancelled".into();
            source.history.push(
                json!({"at":asset_task::now(),"status":"cancelled","destination":target.task_id}),
            );
        }
    } else if let Some(feedback) = state.feedback.iter_mut().find(|f| f.id == id) {
        cancel_feedback(feedback)?;
    }
    state.withdrawn.push(id.into());
    asset_task::save(store, &state)?;
    pause_empty_followup(store, &state)
}
