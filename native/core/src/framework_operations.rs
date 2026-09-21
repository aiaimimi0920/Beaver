use crate::{framework_contract::Job, store::Store};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const KIND: &str = "framework-operation";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub task_id: String,
    pub request_id: String,
    pub job: Job,
    pub source: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub ended_at: Option<String>,
    pub result: Option<Value>,
    pub error: Option<String>,
}

pub fn terminal(status: &str) -> bool {
    matches!(
        status,
        "succeeded" | "failed" | "cancelled" | "interrupted" | "stale"
    )
}

pub fn get(store: &Store, task: &str, id: &str) -> Result<Operation> {
    crate::asset_task::validate_id(id)?;
    let op: Operation = store
        .get(KIND, id)?
        .context("Unknown framework operation")?;
    ensure!(op.task_id == task, "Operation belongs to another task");
    Ok(op)
}

pub fn idle(store: &Store, task: &str) -> Result<()> {
    ensure!(
        !store
            .list::<Operation>(KIND)?
            .iter()
            .any(|o| o.task_id == task && !terminal(&o.status)),
        "Wait for or cancel the active framework operation"
    );
    Ok(())
}

pub fn cancel_task(store: &Store, task: &str, model_only: bool) -> Result<()> {
    for mut op in store.list::<Operation>(KIND)? {
        if op.task_id == task && !terminal(&op.status) && (!model_only || op.source == "model") {
            op.status = "cancelRequested".into();
            store.put(KIND, &op.id, &op)?;
        }
    }
    Ok(())
}

pub fn recover(store: &Store) -> Result<()> {
    for mut op in store.list::<Operation>(KIND)? {
        if !terminal(&op.status) {
            // The callback receipt is atomic with its state transition. Repair only
            // this local receipt/operation gap, without replaying external work.
            if let Job::Callback { request } = &op.job {
                if let Some(id) = request["requestId"].as_str() {
                    if let Some(receipt) =
                        store.get::<Value>(&format!("task-callback/{}", op.task_id), id)?
                    {
                        if receipt["request"] == *request
                            && receipt["threadId"] == json!(op.thread_id)
                            && receipt["turnId"] == json!(op.turn_id)
                        {
                            op.status = "succeeded".into();
                            op.result = Some(receipt["response"].clone());
                            op.ended_at = receipt["time"].as_str().map(String::from);
                            store.put(KIND, &op.id, &op)?;
                            continue;
                        }
                    }
                }
            }
            op.status = "interrupted".into();
            op.ended_at = Some(crate::asset_task::now());
            op.error = Some("Beaver restarted; no side effects are replayed. Re-probe plugins and inspect the saved scene before retrying.".into());
            store.put(KIND, &op.id, &op)?;
            crate::framework_evidence::recovery(
                store,
                &op.task_id,
                "Framework operation interrupted by host restart",
                Some(&op.id),
            )?;
        }
    }
    Ok(())
}

pub fn view(op: &Operation) -> Value {
    let mut value = json!(op);
    value["paused"] =
        json!(op.status == "succeeded" && op.result.as_ref().is_some_and(|v| v["paused"] == true));
    value
}

pub fn require_running(store: &Store, task: &str, id: &str) -> Result<()> {
    ensure!(
        get(store, task, id)?.status == "running",
        "Operation cancelled or interrupted before commit"
    );
    Ok(())
}
