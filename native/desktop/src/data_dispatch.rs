use crate::{
    anyhow_result,
    business_routing::{project_runtime_handles, query_runtime_handles},
    Backend,
};
use serde_json::Value;
use std::sync::atomic::Ordering;

#[path = "data_dispatch_object_catalog.rs"]
mod data_dispatch_object_catalog;
#[path = "data_dispatch_object_import.rs"]
mod data_dispatch_object_import;
#[path = "data_dispatch_object_tasks.rs"]
mod data_dispatch_object_tasks;
#[path = "data_dispatch_projects.rs"]
mod data_dispatch_projects;
#[path = "data_dispatch_state.rs"]
mod data_dispatch_state;
#[path = "data_dispatch_tasks.rs"]
mod data_dispatch_tasks;
use data_dispatch_projects::{
    project_document_operation, resolve_project_asset, reveal_project_asset,
};
use data_dispatch_state::state_operation;
use data_dispatch_tasks::{
    complete_task_action, create_project_task, create_related_task, document_save_operation,
    feature_add_operation, resolve_task_reveal, retry_merge_task, reveal_task, task_callback_state,
    task_events, task_resource_operation, task_setting_operation,
};

pub(crate) fn call(
    backend: &Backend,
    method: String,
    input: Option<Value>,
) -> Result<Value, String> {
    if backend.closing.load(Ordering::SeqCst) {
        return Err("应用正在退出".into());
    }
    if method == "state" {
        return state_operation(backend.store.clone(), backend.project_storage.as_ref())
            .map_err(|error| error.to_string());
    }
    if method == "project.storage.status" {
        let input = input.as_ref().ok_or("缺少项目参数")?;
        let id = input["id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("项目 ID 无效")?;
        let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
        return beaver_core::object_framework_status::registration_status(&store, id)
            .map_err(|error| error.to_string());
    }
    if method == "project.unregister" {
        let input = input.as_ref().ok_or("缺少注销参数")?;
        let id = input["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("项目 ID 无效")?;
        let expected = input["expectedPath"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 2000)
            .ok_or("原项目路径无效")?;
        return backend
            .project_storage
            .unregister(id, expected)
            .map_err(|error| error.to_string());
    }
    if method == "project.reassociate" {
        let input = input.as_ref().ok_or("缺少重新关联参数")?;
        let id = input["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("项目 ID 无效")?;
        let expected = input["expectedPath"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 2000)
            .ok_or("原项目路径无效")?;
        let directory = input["path"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 2000)
            .ok_or("项目路径无效")?;
        return backend
            .project_storage
            .reassociate(id, expected, std::path::Path::new(directory), &backend.root)
            .map_err(|error| error.to_string());
    }
    if let Some(result) = data_dispatch_object_import::dispatch(
        &backend.project_storage,
        &backend.store,
        &backend.root,
        &method,
        input.as_ref(),
    ) {
        return result;
    }
    if let Some(result) = data_dispatch_object_catalog::dispatch(
        &backend.project_storage,
        &backend.store,
        &backend.root,
        &method,
        input.as_ref(),
    ) {
        return result;
    }
    if let Some(result) =
        data_dispatch_object_tasks::dispatch(&backend.project_storage, &method, input.as_ref())
    {
        return result;
    }
    if method == "task.create" {
        return create_project_task(&backend.project_storage, input)
            .map_err(|error| error.to_string());
    }
    if method == "feature.add" {
        return feature_add_operation(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(
        method.as_str(),
        "task.followup" | "task.delegate" | "task.dialogueRollback"
    ) {
        return create_related_task(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(
        method.as_str(),
        "task.direction" | "task.autonomy" | "task.approval"
    ) {
        return task_setting_operation(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "task.retryMerge" {
        return retry_merge_task(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &backend.closing,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(method.as_str(), "task.rollback" | "task.accept") {
        return complete_task_action(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(
        method.as_str(),
        "task.resources" | "task.resourceText" | "task.resourceBytes"
    ) {
        return task_resource_operation(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "task.events" {
        return task_events(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "task.callbackState" {
        return task_callback_state(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "task.reveal" {
        return reveal_task(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(method.as_str(), "project.reveal" | "asset.reveal") {
        return reveal_project_asset(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "document.save" {
        return document_save_operation(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if matches!(method.as_str(), "assets" | "asset.text" | "document.read") {
        return project_document_operation(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &method,
            input,
        )
        .map_err(|error| error.to_string());
    }
    if method == "objectFramework.status" {
        let input = input.ok_or_else(|| "缺少项目参数".to_owned())?;
        let project_id = input["projectId"]
            .as_str()
            .ok_or_else(|| "缺少项目标识".to_owned())?;
        let handles = query_runtime_handles(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &input,
        )?;
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        return beaver_core::object_framework_status::status_with_routing(
            &store,
            project_id,
            handles.project_routed,
        )
        .map_err(|error| error.to_string());
    }
    if method == "logs.query" {
        let input = input.unwrap_or_else(|| serde_json::json!({}));
        let handles = query_runtime_handles(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            &input,
        )?;
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        return beaver_core::call_log::query(&store, &input).map_err(|error| error.to_string());
    }

    if matches!(
        method.as_str(),
        "project.blueprint.save" | "project.overview.save"
    ) {
        let input = input.ok_or_else(|| "缺少项目规划参数".to_owned())?;
        let id = input["id"]
            .as_str()
            .ok_or_else(|| "缺少项目标识".to_owned())?;
        let revision = input["expectedRevision"]
            .as_u64()
            .ok_or_else(|| "规划版本无效".to_owned())?;
        let overview = method == "project.overview.save";
        if overview && input["allowRiskyChanges"] != true {
            return Err("修改基础设定需要明确确认风险".into());
        }
        let handles = project_runtime_handles(
            backend.project_storage.as_ref(),
            backend.store.clone(),
            &backend.root,
            id,
        )?;
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        return beaver_core::blueprint::save(
            &store,
            id,
            input[if overview { "overview" } else { "blueprint" }].clone(),
            revision,
            overview,
            &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string());
    }

    let mut store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
    let result = (|| -> anyhow_result::Result<Value> {
        match method.as_str() {
            "settings.save" => {
                let _setup = backend
                    .setup_gate
                    .try_lock()
                    .map_err(|_| "工具安装期间不能修改设置")?;
                let input = input.as_ref().ok_or("缺少设置参数")?;
                Ok(beaver_core::preferences::save(
                    &mut store,
                    &beaver_core::preferences::SystemVault,
                    input["settings"].clone(),
                    input["keys"].clone(),
                )?)
            }
            "settings.importLocalCodex" => {
                let _setup = backend
                    .setup_gate
                    .try_lock()
                    .map_err(|_| "工具安装期间不能导入设置")?;
                Ok(beaver_core::local_codex::import_current(
                    &mut store,
                    input.as_ref().ok_or("缺少设置参数")?,
                )?)
            }
            "tools.setupStatus" => Ok(store
                .get::<Value>("toolSetup", "main")?
                .unwrap_or_else(beaver_core::tool_setup::idle)),
            "settings.clearKey" => {
                let slot = input
                    .as_ref()
                    .and_then(|v| v["slot"].as_str())
                    .ok_or("缺少凭据类型")?;
                beaver_core::preferences::clear_key(&store, slot)?;
                Ok(Value::Null)
            }
            "project.import" => {
                let directory = input
                    .as_ref()
                    .and_then(|v| v["path"].as_str())
                    .filter(|s| !s.is_empty() && s.len() <= 2000)
                    .ok_or("项目路径无效")?;
                Ok(beaver_core::projects::import_project(
                    &store,
                    &backend.root,
                    std::path::Path::new(directory),
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/design-catalog.json"
                    ))?,
                )?)
            }
            _ => Err(format!("原生后端尚未迁移此操作：{method}").into()),
        }
    })();
    result.map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "data_dispatch_tests.rs"]
mod tests;
