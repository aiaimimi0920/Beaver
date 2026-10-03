//! Caller JSON carries run/revision fencing, never the host's volatile identity.
use crate::Host;
use anyhow::{bail, Result};
use beaver_core::{external_run_contract::RunRequest, external_runs};
use serde_json::Value;

pub async fn call(host: &Host, method: &str, input: Value) -> Result<Value> {
    match method {
        "external.pending" => host.external.pending(),
        "external.context" => host.external.context(input["taskId"].as_str().unwrap()),
        "external.receipt" | "external.history" | "external.jobEvidence" => {
            let task_id = input["taskId"].as_str().unwrap();
            let runtime = crate::runtime::task(&host.router, task_id)?;
            let handle = runtime.store();
            let store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store unavailable"))?;
            if method == "external.history" {
                external_runs::run_history(&store, task_id)
            } else if method == "external.jobEvidence" {
                external_runs::read_job_evidence(
                    &store,
                    &runtime.files(),
                    task_id,
                    input["runId"].as_str().unwrap(),
                    input["requestId"].as_str().unwrap(),
                )
            } else {
                external_runs::read_receipt(
                    &store,
                    task_id,
                    input["runId"].as_str().unwrap(),
                    input["requestId"].as_str().unwrap(),
                )
            }
        }
        "external.submitPlan" | "external.tool" | "external.finish" => {
            let request: RunRequest = serde_json::from_value(input)?;
            match method {
                "external.submitPlan" => host.external.submit_plan(request).await,
                "external.tool" => host.external.tool(request).await,
                _ => host.external.finish(request).await,
            }
        }
        _ => bail!("Unknown external method"),
    }
}
