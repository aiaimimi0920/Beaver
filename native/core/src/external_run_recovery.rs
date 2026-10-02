//! An uncertain process lifetime fences its workspace until ownership is proven clean.
use crate::{external_run_records as records, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

pub(crate) fn blocked(task: &Value) -> bool {
    task.get("externalRecoveryRequired")
        .is_some_and(|value| !value.is_null())
}

pub(crate) fn require_clear(store: &Store, task: &Value) -> Result<()> {
    ensure!(
        !blocked(task),
        "External execution recovery required; workspace reuse is blocked"
    );
    if !crate::external_run_contract::enabled(task) {
        return Ok(());
    }
    for run in store.list::<Value>(records::RUN)? {
        ensure!(
            run["taskId"] != task["id"] || run["cleanupConfirmed"] == true,
            "External process cleanup is unconfirmed; this task/workspace cannot be resumed"
        );
    }
    Ok(())
}

pub(crate) fn quarantine(store: &mut Store, run_id: &str, reason: &str) -> Result<()> {
    let mut run: Value = store
        .get(records::RUN, run_id)?
        .context("External run missing")?;
    let task_id = run["taskId"]
        .as_str()
        .context("Run task missing")?
        .to_owned();
    let mut task: Value = store.get("task", &task_id)?.context("Task missing")?;
    run["status"] = json!("recoveryRequired");
    run["cleanupConfirmed"] = json!(false);
    run["recoveryReason"] = json!(reason);
    task["externalRecoveryRequired"] = json!({"runId":run_id,"reason":reason});
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind=? AND id=?",
            rusqlite::params![run.to_string(), records::RUN, run_id],
        )?;
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), task_id],
        )?;
        Ok(())
    })
}

pub(crate) fn confirmed(store: &mut Store, run_id: &str, status: &str) -> Result<()> {
    let mut run: Value = store
        .get(records::RUN, run_id)?
        .context("External run missing")?;
    let task_id = run["taskId"]
        .as_str()
        .context("Run task missing")?
        .to_owned();
    let mut task: Value = store.get("task", &task_id)?.context("Task missing")?;
    run["status"] = json!(status);
    run["cleanupConfirmed"] = json!(true);
    if task["externalRecoveryRequired"]["runId"] == run_id {
        task.as_object_mut()
            .context("Invalid task")?
            .remove("externalRecoveryRequired");
    }
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind=? AND id=?",
            rusqlite::params![run.to_string(), records::RUN, run_id],
        )?;
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), task_id],
        )?;
        Ok(())
    })
}

pub(crate) fn recover(store: &mut Store) -> Result<()> {
    for run in store.list::<Value>(records::RUN)? {
        if run["cleanupConfirmed"] != true {
            quarantine(store, run["runId"].as_str().context("Run ID missing")?,
                "Host stopped without confirmed process cleanup; do not reuse this task/workspace or replay its tools")?;
        }
    }
    Ok(())
}
