//! Serialized request execution with a durable admission before every side effect.
use crate::{
    executor::Outcome,
    external_run_contract::{FinishRequest, RunRequest, ToolRequest},
    external_run_records::{self as records, Admission},
    external_runs::{ExternalRegistry, Live},
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};

impl ExternalRegistry {
    pub async fn submit_plan(&self, request: RunRequest) -> Result<Value> {
        self.request("submitPlan", request).await
    }
    pub async fn tool(&self, request: RunRequest) -> Result<Value> {
        self.request("tool", request).await
    }
    pub async fn finish(&self, request: RunRequest) -> Result<Value> {
        self.request("finish", request).await
    }

    async fn request(&self, method: &'static str, request: RunRequest) -> Result<Value> {
        request.validate()?;
        let live = self.live(&request.task_id, Some(&request.run_id))?;
        // Keep the admitted operation alive if the transport drops its caller future.
        let serial = live.serial.clone().lock_owned().await;
        tokio::spawn(async move {
            let _serial = serial;
            live.require_active()?;
            let admission = records::begin(
                &mut *live
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Store unavailable"))?,
                &request,
                method,
            )?;
            let mut receipt = match admission {
                Admission::Replay(saved) => return Ok(saved),
                Admission::New(receipt) => receipt,
            };
            receipt["status"] = json!("started");
            records::save(
                &*live
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Store unavailable"))?,
                &receipt,
            )?;
            let result = perform(&live, method, &request, &receipt).await;
            if let Some(saved) = live
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Store unavailable"))?
                .get::<Value>(
                    records::RECEIPT,
                    &records::key(&request.run_id, &request.request_id),
                )?
            {
                if let Some(progress) = saved.get("progress") {
                    receipt["progress"] = progress.clone();
                }
            }
            match &result {
                Ok((value, _)) => {
                    receipt["status"] = json!("succeeded");
                    receipt["result"] = value.clone();
                }
                Err(error) => {
                    receipt["status"] = json!("failed");
                    receipt["error"] = json!(error.to_string());
                }
            }
            // Never return a retryable operation error after side effects. The receipt
            // is the definitive failed/successful result; persistence failure is unknown.
            let outcome = result.ok().and_then(|(_, outcome)| outcome);
            {
                let mut store = live
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
                records::complete(&mut store, &receipt, outcome.is_some())?;
            }
            if let Some(outcome) = outcome {
                live.closed.store(true, Ordering::SeqCst);
                live.finish
                    .lock()
                    .map_err(|_| anyhow::anyhow!("External completion unavailable"))?
                    .take()
                    .context("External completion already requested")?
                    .send(outcome)
                    .map_err(|_| anyhow::anyhow!("External execution already stopped"))?;
            }
            Ok(receipt)
        })
        .await
        .context("External operation worker failed")?
    }
}

async fn perform(
    live: &Arc<Live>,
    method: &str,
    request: &RunRequest,
    receipt: &Value,
) -> Result<(Value, Option<Outcome>)> {
    live.require_active()?;
    match method {
        "submitPlan" => {
            crate::external_tools::ensure_idle(&live.run_id)?;
            let store = live
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
            crate::task_plan::submit_external(
                &store,
                &live.files,
                &live.task_id,
                &request.arguments,
            )?;
            Ok((json!({"accepted":true}), Some(Outcome::Completed)))
        }
        "finish" => {
            let arguments: FinishRequest = serde_json::from_value(request.arguments.clone())?;
            crate::external_tools::ensure_idle(&live.run_id)?;
            let outcome = match arguments.outcome.as_str() {
                "completed" => {
                    ensure!(
                        arguments.error.is_none(),
                        "Completed outcome cannot have an error"
                    );
                    Outcome::Completed
                }
                "interrupted" => Outcome::Interrupted,
                "failed" => Outcome::Failed(
                    arguments
                        .error
                        .filter(|error| !error.is_empty() && error.len() <= 32000)
                        .context("Failed outcome requires an error")?,
                ),
                _ => anyhow::bail!("Unknown external outcome"),
            };
            Ok((
                json!({"accepted":true,"validation":"required"}),
                Some(outcome),
            ))
        }
        "tool" => {
            let arguments: ToolRequest = serde_json::from_value(request.arguments.clone())?;
            {
                let store = live
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
                let task: Value = store.get("task", &live.task_id)?.context("Task missing")?;
                ensure!(
                    crate::external_run_permissions::allowed(
                        &task,
                        &arguments.tool,
                        &arguments.arguments
                    ),
                    "This task is read-only; production tools require a non-planning code task"
                );
            }
            let context = crate::external_tools::Context {
                workspace: live.workspace.clone(),
                run_id: live.run_id.clone(),
                request_id: request.request_id.clone(),
                blender_path: live.blender_path.clone(),
            };
            let live = live.clone();
            let mut progress_receipt = receipt.clone();
            let result = tokio::task::spawn_blocking(move || {
                let mut progress = |value: &Value| {
                    progress_receipt["progress"] = value.clone();
                    records::save(
                        &*live
                            .store
                            .lock()
                            .map_err(|_| anyhow::anyhow!("Store unavailable"))?,
                        &progress_receipt,
                    )
                };
                crate::external_tools::execute(
                    &context,
                    &arguments.tool,
                    &arguments.arguments,
                    &live.cancelled,
                    &mut progress,
                )
            })
            .await
            .context("External tool worker failed")??;
            Ok((result, None))
        }
        _ => anyhow::bail!("Unknown external operation"),
    }
}
