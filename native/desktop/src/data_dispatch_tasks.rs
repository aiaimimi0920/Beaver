use crate::business_routing::task_runtime_handles;
use beaver_core::{
    documents, files::Files, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub(super) fn create_project_task(
    router: &ProjectStorageRouter,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let project_id = input["projectId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少项目标识"))?;
    let runtime = router.runtime_for_project(project_id)?;
    let store_handle = runtime.store();
    let files = runtime.files();
    let mut store = store_handle
        .lock()
        .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
    let task = beaver_core::task_create::create(
        &mut store,
        files.as_ref(),
        input,
        &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
        &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
    )?;
    router.index_task(&task)?;
    Ok(task)
}

pub(super) fn document_save_operation(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少操作参数"))?;
    let project_id = input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少项目标识"))?;
    let relative = input["path"]
        .as_str()
        .filter(|path| path.len() <= 2000)
        .ok_or_else(|| anyhow::anyhow!("资料路径无效"))?;
    let text = input["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少资料内容"))?;
    let revision = match input.get("revision") {
        Some(Value::Null) => None,
        Some(Value::String(value))
            if value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            Some(value.as_str())
        }
        _ => return Err(anyhow::anyhow!("资料版本无效")),
    };

    match router.runtime_for_project(project_id) {
        Ok(runtime) => {
            let store_handle = runtime.store();
            let task = {
                let mut store = store_handle
                    .lock()
                    .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
                documents::save_document(
                    &mut store,
                    runtime.files().as_ref(),
                    project_id,
                    relative,
                    text,
                    revision,
                )?
            };
            router.index_task(&task)?;
            return Ok(task);
        }
        Err(project_error) => {
            if router.registered_project_uses_local_storage(project_id)? {
                return Err(anyhow::anyhow!(
                    "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                ));
            }
        }
    }

    let mut store = host_store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(documents::save_document(
        &mut store,
        &Files::new(host_root.to_path_buf()),
        project_id,
        relative,
        text,
        revision,
    )?)
}

pub(super) fn create_related_task(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let task_id = input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少任务标识"))?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let task = {
        let mut store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        beaver_core::task_relations::create(
            &mut store,
            handles.files.as_ref(),
            method,
            input,
            &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
            &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
        )?
    };
    if handles.project_routed {
        router.index_task(&task)?;
    }
    Ok(task)
}

fn input_task_id(input: &Value) -> anyhow::Result<&str> {
    input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("缺少任务标识"))
}

pub(super) fn feature_add_operation(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少功能块参数"))?;
    let project_id = input["projectId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("项目标识无效"))?;
    let designs: Value =
        serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
    let blueprints: Value =
        serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
    let sources: Value =
        serde_json::from_str(include_str!("../../../dist-native/feature-sources.json"))?;
    match router.runtime_for_project(project_id) {
        Ok(runtime) => {
            let store_handle = runtime.store();
            let mut store = store_handle
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
            let task = beaver_core::feature_tasks::create(
                &mut store,
                runtime.files().as_ref(),
                input,
                &sources,
                &designs,
                &blueprints,
            )?;
            router.index_task(&task)?;
            return Ok(task);
        }
        Err(project_error) => {
            if router.registered_project_uses_local_storage(project_id)? {
                return Err(anyhow::anyhow!(
                    "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                ));
            }
        }
    }
    let mut store = host_store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(beaver_core::feature_tasks::create(
        &mut store,
        &Files::new(host_root.to_path_buf()),
        input,
        &sources,
        &designs,
        &blueprints,
    )?)
}

pub(super) fn task_setting_operation(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let missing = match method {
        "task.approval" => "缺少审批参数",
        "task.autonomy" => "缺少任务设置",
        _ => "缺少任务参数",
    };
    let input = input.ok_or_else(|| anyhow::anyhow!(missing))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    match method {
        "task.approval" => Ok(beaver_core::task_plan::approval(
            &store,
            task_id,
            input["autoAccept"]
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("审批策略无效"))?,
        )?),
        "task.autonomy" => Ok(beaver_core::autonomy::set(
            &store,
            task_id,
            input
                .get("askRatio")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("缺少询问档位"))?,
        )?),
        "task.direction" => Ok(beaver_core::task_relations::set_direction(&store, &input)?),
        _ => anyhow::bail!("未知任务设置操作：{method}"),
    }
}

pub(super) fn retry_merge_task(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    closing: &AtomicBool,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let mut store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(beaver_core::task_finish::retry_merge(
        &mut store,
        handles.files.as_ref(),
        task_id,
        closing,
    )?)
}

pub(super) fn complete_task_action(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let mut store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    if method == "task.accept" {
        beaver_core::task_actions::accept(&mut store, task_id)?;
    } else {
        let keep: Vec<String> = serde_json::from_value(input["keep"].clone())
            .map_err(|_| anyhow::anyhow!("保留文件列表无效"))?;
        beaver_core::task_actions::rollback(&mut store, handles.files.as_ref(), task_id, keep)?;
    }
    Ok(Value::Null)
}

pub(super) fn task_resource_operation(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    let task: Value = store
        .get("task", task_id)?
        .ok_or_else(|| anyhow::anyhow!("任务不存在"))?;
    if method == "task.resources" {
        return Ok(beaver_core::task_resources::resources(
            handles.files.as_ref(),
            &task,
        )?);
    }
    let path = input["path"]
        .as_str()
        .filter(|path| !path.is_empty() && path.encode_utf16().count() <= 2000)
        .ok_or_else(|| anyhow::anyhow!("资源路径无效"))?;
    if method == "task.resourceBytes" {
        Ok(beaver_core::task_resources::raw(
            handles.files.as_ref(),
            &task,
            path,
        )?)
    } else {
        Ok(json!(beaver_core::task_resources::text(
            handles.files.as_ref(),
            &task,
            path,
        )?))
    }
}

pub(super) fn task_events(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少任务参数"))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(serde_json::to_value(store.events(task_id)?)?)
}

pub(super) fn task_callback_state(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let input = input.unwrap_or_else(|| json!({}));
    crate::business_catalog::validate("task.callbackState", &input)
        .map_err(|(_, message)| anyhow::Error::msg(message))?;
    let task_id = input["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing task ID"))?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let mut store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    Ok(beaver_core::task_callback::business(
        &mut store,
        "task.callbackState",
        &input,
    )?)
}

pub(super) fn resolve_task_reveal(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<beaver_core::reveal::Target> {
    let input = input.ok_or_else(|| anyhow::anyhow!("缺少显示参数"))?;
    let task_id = input_task_id(&input)?;
    let handles =
        task_runtime_handles(router, host_store, host_root, task_id).map_err(anyhow::Error::msg)?;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    beaver_core::reveal::resolve(&store, handles.files.as_ref(), "task.reveal", &input)
}

pub(super) fn reveal_task(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: Option<Value>,
) -> anyhow::Result<Value> {
    let target = resolve_task_reveal(router, host_store, host_root, input)?;
    beaver_core::reveal::open(&target)?;
    Ok(json!(""))
}
