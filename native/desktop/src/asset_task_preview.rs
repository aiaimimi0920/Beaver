use crate::{business_routing::task_runtime_handles, Backend};
use anyhow::{bail, Context, Result};
use beaver_core::{
    asset_preview::Client,
    asset_reference,
    asset_task::{self, Frame, Reference},
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) fn client(backend: &Backend, id: &str, session: Option<&str>) -> Result<Client> {
    asset_task::validate_id(id)?;
    let client = backend
        .scheduler
        .asset_client(id)
        .map_err(anyhow::Error::msg)?;
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
    let state = asset_task::get(&store, id)?;
    if state.session_id.as_deref() != Some(&client.session_id)
        || session.is_some_and(|s| s != client.session_id)
    {
        bail!("Blender 会话已改变，请重新获取画面");
    }
    Ok(client)
}

pub(crate) async fn status(backend: &Backend, id: &str, input: &Value) -> Result<Value> {
    // A disconnected task still has durable stages and feedback to display.
    {
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
        asset_task::get(&store, id)?;
    }
    let result = async {
        let client = client(backend, id, None)?;
        let mut status = client
            .call(
                "status",
                json!({"subscriber":input["subscriber"],"release":input["release"] == true}),
            )
            .await?;
        status["connected"] = json!(true);
        Ok::<_, anyhow::Error>(status)
    }
    .await;
    Ok(result
        .unwrap_or_else(|error| json!({"connected":false,"taskId":id,"error":error.to_string()})))
}

pub(crate) async fn freeze(backend: Arc<Backend>, id: &str, input: &Value) -> Result<Value> {
    let client = client(
        &backend,
        id,
        Some(input["sessionId"].as_str().context("缺少会话标识")?),
    )?;
    let frame_id = input["frameId"].as_str().context("缺少画面标识")?;
    let (frame, bytes) = client.capture(frame_id).await?;
    let handles = task_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        id,
    )
    .map_err(anyhow::Error::msg)?;
    let id = id.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        Ok(json!(asset_reference::capture(
            &store,
            handles.files.as_ref(),
            &id,
            frame,
            &bytes
        )?))
    })
    .await?
}

pub(crate) async fn pick(backend: &Backend, id: &str, input: &Value) -> Result<Value> {
    let reference_id = input["referenceId"].as_str().context("缺少参考画面")?;
    let handles = task_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        id,
    )
    .map_err(anyhow::Error::msg)?;
    let reference = {
        let store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        asset_reference::get(&store, id, reference_id)?
    };
    let client = client(backend, id, Some(&reference.frame.session_id))?;
    let result = client
        .call(
            "pick",
            json!({"frame":reference.frame,"point":input["point"]}),
        )
        .await?;
    if result.is_null() {
        bail!("未命中可见网格，请在模型表面重新点选");
    }
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
    let state = asset_task::get(&store, id)?;
    if state.session_id.as_deref() != Some(&client.session_id) {
        bail!("Blender 会话已改变");
    }
    Ok(json!(asset_reference::set_pick(
        &store,
        id,
        reference_id,
        serde_json::from_value(result)?
    )?))
}

pub(crate) fn reference(backend: &Backend, id: &str, reference_id: &str) -> Result<Reference> {
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
    let state = asset_task::get(&store, id)?;
    if let Some(reference) = state
        .last_frame
        .iter()
        .chain(state.feedback.iter().map(|f| &f.reference))
        .find(|r| r.id == reference_id)
    {
        return Ok(reference.clone());
    }
    asset_reference::get(&store, id, reference_id)
}

/// Shared by the native binary scheme and the authenticated acceptance API.
pub(crate) async fn image(backend: Arc<Backend>, path: &str) -> Result<Vec<u8>> {
    let parts: Vec<_> = path.trim_start_matches('/').split('/').collect();
    match parts.as_slice() {
        ["asset-task", id, "references", reference_id] => {
            let reference = reference(&backend, id, reference_id)?;
            let handles = task_runtime_handles(
                &backend.project_storage,
                backend.store.clone(),
                &backend.root,
                id,
            )
            .map_err(anyhow::Error::msg)?;
            tauri::async_runtime::spawn_blocking(move || {
                asset_reference::read(handles.files.as_ref(), &reference)
            })
            .await?
        }
        ["asset-task", id, session, "frames", frame] => {
            client(&backend, id, Some(session))?.frame(frame).await
        }
        _ => bail!("未知资产画面路径"),
    }
}

pub(crate) async fn validate_pick(
    backend: &Backend,
    id: &str,
    reference: &Reference,
) -> Result<Frame> {
    let client = client(backend, id, Some(&reference.frame.session_id))?;
    // This reaches Blender's main thread, including depsgraph updates; status alone is not proof.
    Ok(serde_json::from_value(
        client
            .call("validate", json!({"frame":reference.frame}))
            .await?,
    )?)
}
