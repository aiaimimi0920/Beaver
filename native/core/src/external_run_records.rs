//! Durable at-most-once admissions. Unknown effects are never automatically replayed.
use crate::{external_run_contract::RunRequest, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

pub(crate) const RUN: &str = "external-run";
pub(crate) const RECEIPT: &str = "external-request";
pub(crate) fn key(run: &str, request: &str) -> String {
    format!("{}:{run}{request}", run.len())
}

pub(crate) fn create(store: &mut Store, task_id: &str, run_id: &str) -> Result<()> {
    let mut task: Value = store.get("task", task_id)?.context("Task not found")?;
    ensure!(
        task["status"] == "running" && crate::external_run_contract::enabled(&task),
        "External task is not running"
    );
    crate::external_run_recovery::require_clear(store, &task)?;
    for previous in store.list::<Value>(RUN)? {
        ensure!(
            previous["taskId"] != task_id || previous["cleanupConfirmed"] == true,
            "Previous external process lifetime is unconfirmed; task/workspace reuse is blocked"
        );
    }
    let old = task["externalRunId"].as_str().map(str::to_owned);
    let mut previous = old
        .as_deref()
        .map(|id| store.get::<Value>(RUN, id))
        .transpose()?
        .flatten();
    if let Some(previous) = &mut previous {
        ensure!(
            !matches!(
                previous["status"].as_str(),
                Some("active" | "finishing" | "recoveryRequired")
            ),
            "External task already claimed"
        );
        previous["status"] = json!("revoked");
    }
    task["externalRunId"] = json!(run_id);
    let run = json!({"runId":run_id,"taskId":task_id,"revision":0,"status":"active","cleanupConfirmed":false});
    store.transaction(|db| {
        if let (Some(old), Some(previous)) = (old, previous) {
            db.execute(
                "UPDATE entities SET value=? WHERE kind=? AND id=?",
                rusqlite::params![previous.to_string(), RUN, old],
            )?;
        }
        db.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
            rusqlite::params![RUN, run_id, run.to_string()],
        )?;
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), task_id],
        )?;
        Ok(())
    })
}

pub(crate) enum Admission {
    New(Value),
    Replay(Value),
}
pub(crate) fn begin(store: &mut Store, request: &RunRequest, method: &str) -> Result<Admission> {
    let mut run: Value = store
        .get(RUN, &request.run_id)?
        .context("External run missing")?;
    ensure!(
        run["status"] == "active" && run["taskId"] == request.task_id,
        "External run revoked"
    );
    let task: Value = store
        .get("task", &request.task_id)?
        .context("Task missing")?;
    ensure!(
        task["status"] == "running" && task["externalRunId"] == request.run_id,
        "Stale external run"
    );
    let identity = key(&request.run_id, &request.request_id);
    let fingerprint = request.fingerprint(method);
    if let Some(saved) = store.get::<Value>(RECEIPT, &identity)? {
        ensure!(
            saved["fingerprint"] == fingerprint,
            "External requestId parameter conflict"
        );
        ensure!(
            matches!(saved["status"].as_str(), Some("succeeded" | "failed")),
            "External request outcome unknown; inspect receipt, do not replay"
        );
        return Ok(Admission::Replay(saved));
    }
    ensure!(
        run["revision"] == request.revision,
        "Stale external revision"
    );
    let next = request
        .revision
        .checked_add(1)
        .context("External revision exhausted")?;
    run["revision"] = json!(next);
    let receipt = json!({"taskId":request.task_id,"runId":request.run_id,"requestId":request.request_id,
        "revision":next,"fingerprint":fingerprint,"status":"pending"});
    store.transaction(|db| {
        db.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
            rusqlite::params![RECEIPT, identity, receipt.to_string()],
        )?;
        db.execute(
            "UPDATE entities SET value=? WHERE kind=? AND id=?",
            rusqlite::params![run.to_string(), RUN, request.run_id],
        )?;
        Ok(())
    })?;
    Ok(Admission::New(receipt))
}

pub(crate) fn save(store: &Store, receipt: &Value) -> Result<()> {
    let run = receipt["runId"].as_str().context("Receipt run missing")?;
    let request = receipt["requestId"]
        .as_str()
        .context("Receipt request missing")?;
    store.put(RECEIPT, &key(run, request), receipt)
}

pub(crate) fn complete(store: &mut Store, receipt: &Value, finishing: bool) -> Result<()> {
    let run_id = receipt["runId"].as_str().context("Receipt run missing")?;
    let identity = key(
        run_id,
        receipt["requestId"]
            .as_str()
            .context("Receipt request missing")?,
    );
    let mut run: Value = store.get(RUN, run_id)?.context("External run missing")?;
    if finishing {
        run["status"] = json!("finishing");
    }
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind=? AND id=?",
            rusqlite::params![receipt.to_string(), RECEIPT, identity],
        )?;
        if finishing {
            db.execute(
                "UPDATE entities SET value=? WHERE kind=? AND id=?",
                rusqlite::params![run.to_string(), RUN, run_id],
            )?;
        }
        Ok(())
    })
}

pub(crate) fn recover(store: &mut Store) -> Result<()> {
    crate::external_run_recovery::recover(store)?;
    for mut receipt in store.list::<Value>(RECEIPT)? {
        if matches!(receipt["status"].as_str(), Some("pending" | "started")) {
            receipt["status"] = json!("unknown");
            receipt["error"] =
                json!("Host stopped before a definitive result; automatic replay prohibited");
            save(store, &receipt)?;
        }
    }
    Ok(())
}
