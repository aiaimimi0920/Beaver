use crate::{asset_task_preview as preview, asset_task_windows, Backend};
use anyhow::{bail, Context, Result};
use base64::Engine;
use beaver_core::{
    asset_feedback::{self, Submission},
    asset_reference, asset_submission, asset_task, asset_withdrawal,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::Emitter;

pub(crate) fn transient(method: &str) -> bool {
    matches!(
        method,
        "assetTask.status" | "assetTask.view" | "assetTask.state" | "assetTask.image"
    )
}

pub(crate) async fn call(
    app: tauri::AppHandle,
    backend: Arc<Backend>,
    method: &str,
    input: Value,
) -> Result<Value> {
    let id = input["id"].as_str().context("缺少任务标识")?;
    asset_task::validate_id(id)?;
    match method {
        "assetTask.open" => {
            return asset_task_windows::open(&app, &backend, id)
                .await
                .map_err(anyhow::Error::msg)
        }
        "assetTask.status" => return preview::status(&backend, id, &input).await,
        "assetTask.view" => {
            let session = input["sessionId"].as_str().context("缺少会话标识")?;
            return preview::client(&backend, id, Some(session))?
                .call("view", input["view"].clone())
                .await;
        }
        "assetTask.freeze" => return preview::freeze(backend, id, &input).await,
        "assetTask.pick" => return preview::pick(&backend, id, &input).await,
        "assetTask.state" => return state(&backend, id),
        "assetTask.image" => {
            let reference_id = input["referenceId"].as_str().context("缺少参考画面")?;
            asset_task::validate_id(reference_id)?;
            let bytes = preview::image(
                backend,
                &format!("/asset-task/{id}/references/{reference_id}"),
            )
            .await?;
            return Ok(
                json!({"contentType":"image/png","base64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
            );
        }
        _ => {}
    }
    let result = match method {
        "assetTask.submit" => submit(backend.clone(), serde_json::from_value(input)?).await?,
        "assetTask.cancel" => {
            let store = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            asset_withdrawal::cancel(
                &store,
                id,
                input["feedbackId"].as_str().context("缺少反馈标识")?,
            )?;
            Value::Null
        }
        _ => bail!("未知资产任务操作"),
    };
    backend.scheduler.wake().map_err(anyhow::Error::msg)?;
    let _ = app.emit("beaver:changed", ());
    Ok(result)
}

fn state(backend: &Backend, id: &str) -> Result<Value> {
    let store = backend
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    let asset = asset_task::get(&store, id)?;
    let project: Value = store
        .get("project", &asset.project_id)?
        .context("项目不存在")?;
    let mut tasks: Vec<Value> = store
        .list::<Value>("task")?
        .into_iter()
        .filter(|t| t["projectId"] == asset.project_id)
        .collect();
    for task in &mut tasks {
        task["effectiveAskRatio"] = json!(beaver_core::autonomy::effective(&store, task)?);
        task["delivery"] = project["delivery"].clone();
    }
    let task = tasks.iter().find(|t| t["id"] == id).context("任务不存在")?;
    Ok(json!({"task":task,"tasks":tasks,"project":project,"asset":asset}))
}

async fn submit(backend: Arc<Backend>, input: Submission) -> Result<Value> {
    input.validate()?;
    let (reference, delivered) = {
        let store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let state = asset_task::get(&store, &input.id)?;
        if let Some(feedback) = asset_feedback::duplicate(&state, &input)? {
            return Ok(json!(feedback));
        }
        let task: Value = store.get("task", &input.id)?.context("任务不存在")?;
        let delivered = matches!(task["status"].as_str(), Some("completed" | "rolledBack"));
        (
            asset_reference::get(&store, &input.id, &input.reference_id)?,
            delivered,
        )
    };
    let live = if reference.pick.is_some() && !delivered {
        Some(preview::validate_pick(&backend, &input.id, &reference).await?)
    } else {
        None
    };
    // Store serialization also protects final merge and dependent-task release.
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let feedback = asset_submission::submit(
            &mut store,
            &backend.root,
            input,
            live.as_ref(),
            &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
            &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
        )?;
        Ok(json!(feedback))
    })
    .await?
}
