use beaver_core::{
    documents, files::Files, project_storage_router::ProjectStorageRouter, reveal, store::Store,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub(super) fn project_document_operation(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少操作参数"))?;
    let project_id = input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少项目标识"))?;
    match router.runtime_for_project(project_id) {
        Ok(runtime) => return read_project_document(method, runtime.project_root(), &input),
        Err(project_error) => {
            if router.registered_project_uses_local_storage(project_id)? {
                return Err(anyhow::anyhow!(
                    "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                ));
            }
        }
    }
    let store = host_store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    let root = documents::project_path(&store, project_id)?;
    read_project_document(method, &root, &input)
}

fn read_project_document(method: &str, root: &Path, input: &Value) -> anyhow::Result<Value> {
    if method == "assets" {
        return Ok(json!(documents::assets(root)?));
    }
    let relative = input["path"]
        .as_str()
        .filter(|path| path.len() <= 2000)
        .ok_or_else(|| anyhow::anyhow!("资料路径无效"))?;
    match method {
        "asset.text" => Ok(json!(documents::text(root, relative)?)),
        "document.read" => Ok(documents::read_document(root, relative)?),
        _ => Err(anyhow::anyhow!("未知资料操作：{method}")),
    }
}

pub(super) fn resolve_project_asset(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<reveal::Target> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少显示参数"))?;
    let project_id = input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少项目标识"))?;
    match router.runtime_for_project(project_id) {
        Ok(runtime) => {
            let store = runtime.store();
            let store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
            return Ok(reveal::resolve(
                &store,
                runtime.files().as_ref(),
                method,
                &input,
            )?);
        }
        Err(project_error) => {
            if router.registered_project_uses_local_storage(project_id)? {
                return Err(anyhow::anyhow!(
                    "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                ));
            }
        }
    }
    let store = host_store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(reveal::resolve(
        &store,
        &Files::new(host_root.to_path_buf()),
        method,
        &input,
    )?)
}

pub(super) fn reveal_project_asset(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let target = resolve_project_asset(router, host_store, host_root, method, input)?;
    reveal::open(&target)?;
    Ok(if method == "asset.reveal" {
        Value::Null
    } else {
        json!("")
    })
}
