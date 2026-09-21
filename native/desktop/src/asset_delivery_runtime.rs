use crate::{business_routing::task_runtime_handles, Backend};
use anyhow::{bail, ensure, Context, Result};
use base64::Engine;
use beaver_core::{asset_delivery_files as artifacts, asset_delivery_review as review};
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};
use tauri::Emitter;

pub(crate) async fn call(
    app: tauri::AppHandle,
    backend: Arc<Backend>,
    method: &str,
    input: Value,
) -> Result<Value> {
    crate::business_catalog::validate(method, &input)
        .map_err(|(_, message)| anyhow::anyhow!(message))?;
    if method == "assetTask.deliveryDecide" {
        let decision: review::Decision = serde_json::from_value(input)?;
        let handles = task_runtime_handles(
            &backend.project_storage,
            backend.store.clone(),
            &backend.root,
            &decision.id,
        )
        .map_err(anyhow::Error::msg)?;
        {
            let store = handles
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            if let Some(result) = review::duplicate(&store, &decision)? {
                return Ok(result);
            }
            review::check(&store, &decision)?;
        }
        backend
            .scheduler
            .synchronize(decision.id.clone())
            .await
            .map_err(anyhow::Error::msg)?;
        let target = backend.clone();
        let store = handles.store;
        let files = handles.files;
        let result = tauri::async_runtime::spawn_blocking(move || {
            let prepared = {
                let store = store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
                if let Some(result) = review::duplicate(&store, &decision)? {
                    return Ok(result);
                }
                review::prepare(&store, decision)?
            };
            let verified = prepared.verify(&files)?;
            let mut store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            ensure!(
                !target.closing.load(Ordering::SeqCst),
                "Application is closing"
            );
            review::decide(&mut store, verified)
        })
        .await??;
        backend.scheduler.wake().map_err(anyhow::Error::msg)?;
        let _ = app.emit("beaver:changed", ());
        return Ok(result);
    }
    let id = input["id"].as_str().context("Missing task ID")?.to_owned();
    let handles = task_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        &id,
    )
    .map_err(anyhow::Error::msg)?;
    let method = method.to_owned();
    let store = handles.store;
    let files = handles.files;
    tauri::async_runtime::spawn_blocking(move || {
        let candidate = {
            let store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            if method == "assetTask.deliveryState" {
                return review::inspect(&store, &id);
            }
            artifacts::get(
                &store,
                &id,
                input["candidateId"]
                    .as_str()
                    .context("Missing candidate ID")?,
            )?
        };
        match method.as_str() {
            "assetTask.deliveryFile" => {
                let path = input["path"]
                    .as_str()
                    .context("Missing candidate file path")?;
                let bytes = artifacts::read(&files, &candidate, path)?;
                Ok(json!({"path":path,"sha256":candidate.files[path],"base64":base64::engine::general_purpose::STANDARD.encode(bytes)}))
            }
            "assetTask.deliveryExport" => Ok(json!({"path":artifacts::export(&files, &candidate)?})),
            _ => bail!("Unknown delivery operation"),
        }
    }).await?
}
