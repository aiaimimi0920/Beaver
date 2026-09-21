use crate::{
    asset_task_preview as preview, asset_task_windows, business_routing::task_runtime_handles,
    Backend,
};
use anyhow::{bail, Context, Result};
use base64::Engine;
use beaver_core::{
    asset_feedback::{self, Submission},
    asset_reference, asset_submission, asset_task, asset_withdrawal,
    store::Store,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::Emitter;

pub(crate) fn transient(method: &str) -> bool {
    matches!(
        method,
        "assetTask.status"
            | "assetTask.view"
            | "assetTask.state"
            | "assetTask.image"
            | "assetTask.deliveryState"
            | "assetTask.deliveryFile"
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
    if method.starts_with("assetTask.delivery") {
        return crate::asset_delivery_runtime::call(app, backend, method, input).await;
    }
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
            let handles = task_runtime_handles(
                &backend.project_storage,
                backend.store.clone(),
                &backend.root,
                id,
            )
            .map_err(anyhow::Error::msg)?;
            let store = handles
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
    let handles = task_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        id,
    )
    .map_err(anyhow::Error::msg)?;
    let store = handles
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

fn feedback_task_to_index(store: &Store, task_id: &str) -> Result<Option<Value>> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Ok(None);
    }
    store
        .get::<Value>("task", task_id)?
        .with_context(|| format!("反馈任务不存在：{task_id}"))
        .map(Some)
}

async fn submit(backend: Arc<Backend>, input: Submission) -> Result<Value> {
    input.validate()?;
    let handles = task_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        &input.id,
    )
    .map_err(anyhow::Error::msg)?;
    let duplicate = {
        let store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let state = asset_task::get(&store, &input.id)?;
        if let Some(feedback) = asset_feedback::duplicate(&state, &input)? {
            Some((
                feedback.clone(),
                feedback_task_to_index(&store, &feedback.task_id)?,
            ))
        } else {
            None
        }
    };
    if let Some((feedback, task_to_index)) = duplicate {
        if let Some(task) = task_to_index {
            backend.project_storage.index_task(&task)?;
        }
        return Ok(json!(feedback));
    }
    let (reference, delivered) = {
        let store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
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
    let store = handles.store;
    let files = handles.files;
    let (result, task_to_index) =
        tauri::async_runtime::spawn_blocking(move || -> Result<(Value, Option<Value>)> {
            let mut store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            let feedback = asset_submission::submit(
                &mut store,
                files.as_ref(),
                input,
                live.as_ref(),
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
            )?;
            let task_to_index = feedback_task_to_index(&store, &feedback.task_id)?;
            Ok((json!(feedback), task_to_index))
        })
        .await??;
    if let Some(task) = task_to_index {
        backend.project_storage.index_task(&task)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::feedback_task_to_index;
    use beaver_core::store::Store;
    use serde_json::json;

    #[test]
    fn feedback_task_to_index_reads_existing_task() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(&temp.path().join("store"))?;
        store.put(
            "task",
            "task-a",
            &json!({"id":"task-a","projectId":"project-a","status":"queued"}),
        )?;

        let task = feedback_task_to_index(&store, "task-a")?.expect("task should be indexed");
        assert_eq!(task["projectId"], "project-a");
        Ok(())
    }

    #[test]
    fn feedback_task_to_index_ignores_empty_task_id() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(&temp.path().join("store"))?;

        assert!(feedback_task_to_index(&store, "   ")?.is_none());
        Ok(())
    }
}
