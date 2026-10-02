//! Read-only recovery evidence, scoped to a real task in a host-resolved store.
use crate::{external_run_records as records, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

fn require_task(store: &Store, id: &str) -> Result<()> {
    let task: Value = store.get("task", id)?.context("Task not found")?;
    ensure!(
        task["id"] == id && crate::external_run_contract::enabled(&task),
        "External task scope required"
    );
    Ok(())
}

pub fn read_receipt(store: &Store, task_id: &str, run_id: &str, request_id: &str) -> Result<Value> {
    require_task(store, task_id)?;
    let run: Value = store
        .get(records::RUN, run_id)?
        .context("External run not found")?;
    ensure!(
        run["runId"] == run_id && run["taskId"] == task_id,
        "External run scope mismatch"
    );
    let receipt: Value = store
        .get(records::RECEIPT, &records::key(run_id, request_id))?
        .context("External receipt not found")?;
    ensure!(
        receipt["taskId"] == task_id
            && receipt["runId"] == run_id
            && receipt["requestId"] == request_id,
        "External receipt scope mismatch"
    );
    Ok(receipt)
}

pub fn run_history(store: &Store, task_id: &str) -> Result<Value> {
    require_task(store, task_id)?;
    let runs: Vec<Value> = store
        .list::<Value>(records::RUN)?
        .into_iter()
        .filter(|run| run["taskId"] == task_id)
        .collect();
    Ok(json!({"taskId":task_id,"runs":runs}))
}

/// Supplementary crash evidence. This never authorizes a new run or clears recovery.
pub fn read_job_evidence(
    store: &Store,
    files: &crate::files::Files,
    task_id: &str,
    run_id: &str,
    request_id: &str,
) -> Result<Value> {
    let receipt = read_receipt(store, task_id, run_id, request_id)?;
    ensure!(
        receipt["fingerprint"]["method"] == "tool"
            && matches!(
                receipt["fingerprint"]["arguments"]["tool"].as_str(),
                Some("blender.start" | "blender.python")
            ),
        "Only a recorded managed Blender start has historical job evidence"
    );
    let task: Value = store.get("task", task_id)?.context("Task missing")?;
    let workspace = files.resolve_workspace(
        task_id,
        std::path::Path::new(
            task["workspace"]
                .as_str()
                .context("Task workspace missing")?,
        ),
    )?;
    let start = receipt
        .get("progress")
        .or_else(|| receipt.get("result"))
        .context("No durable job identity was recorded")?;
    ensure!(
        start["runId"] == run_id && start["requestId"] == request_id,
        "Recorded job identity mismatch"
    );
    let evidence = crate::external_tools::historical_job_evidence(&workspace, start)?;
    let run: Value = store.get(records::RUN, run_id)?.context("Run missing")?;
    Ok(
        json!({"taskId":task_id,"runId":run_id,"requestId":request_id,
        "runStatus":run["status"],"cleanupConfirmed":run["cleanupConfirmed"] == true,
        "authority":"supplementary_only_does_not_clear_recovery","job":evidence}),
    )
}
