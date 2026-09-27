//! Project catalog commands. Mutations require an open project-local runtime.
use crate::business_routing::project_runtime_handles;
use beaver_core::{
    object_catalog, object_registration, object_version_acceptance, object_version_capture,
    project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::Value;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub(super) fn dispatch(
    router: &ProjectStorageRouter,
    host: &Arc<Mutex<Store>>,
    root: &Path,
    method: &str,
    input: Option<&Value>,
) -> Option<Result<Value, String>> {
    if !matches!(
        method,
        "object.list"
            | "object.get"
            | "object.register"
            | "object.updateRegistration"
            | "object.captureVersion"
            | "object.acceptVersion"
            | "object.versionFile"
            | "object.scenePreview.run"
            | "object.scenePreview.get"
            | "object.attemptScenePreview.run"
            | "object.attemptScenePreview.get"
    ) {
        return None;
    }
    Some(
        input
            .ok_or_else(|| anyhow::anyhow!("缺少对象参数"))
            .and_then(|input| call(router, host, root, method, input))
            .map_err(|error| error.to_string()),
    )
}

fn call(
    router: &ProjectStorageRouter,
    host: &Arc<Mutex<Store>>,
    root: &Path,
    method: &str,
    input: &Value,
) -> anyhow::Result<Value> {
    let target = if matches!(
        method,
        "object.scenePreview.run" | "object.attemptScenePreview.run"
    ) {
        &input["target"]
    } else {
        input
    };
    let project_id = target["projectId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("项目 ID 无效"))?;
    if matches!(method, "object.list" | "object.get") {
        let handles = project_runtime_handles(router, host.clone(), root, project_id)
            .map_err(anyhow::Error::msg)?;
        let store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        return if method == "object.get" {
            let id = input["objectId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| anyhow::anyhow!("对象 ID 无效"))?;
            Ok(serde_json::to_value(
                object_catalog::get(&store, id)?.filter(|item| item.project_id == project_id),
            )?)
        } else {
            Ok(serde_json::to_value(object_catalog::search(
                &store,
                project_id,
                input.get("query").and_then(Value::as_str),
            )?)?)
        };
    }
    let runtime = router.runtime_for_project(project_id)?;
    if method == "object.scenePreview.run" {
        return beaver_core::object_scene_preview::enqueue(&runtime, input);
    }
    if method == "object.scenePreview.get" {
        return beaver_core::object_scene_preview::latest(
            &runtime,
            &serde_json::from_value(input.clone())?,
        );
    }
    if method == "object.attemptScenePreview.run" {
        return beaver_core::object_attempt_scene_preview::enqueue(&runtime, input);
    }
    if method == "object.attemptScenePreview.get" {
        return beaver_core::object_attempt_scene_preview::latest(
            &runtime,
            &serde_json::from_value(input.clone())?,
        );
    }
    if method == "object.versionFile" {
        return Ok(serde_json::to_value(
            beaver_core::object_version_file::read(
                &runtime,
                &serde_json::from_value(input.clone())?,
            )?,
        )?);
    }
    let receipt = match method {
        "object.register" => {
            object_registration::register(&runtime, &serde_json::from_value(input.clone())?)?
        }
        "object.updateRegistration" => {
            object_registration::update(&runtime, &serde_json::from_value(input.clone())?)?
        }
        "object.captureVersion" => {
            object_version_capture::capture(&runtime, &serde_json::from_value(input.clone())?)?
        }
        "object.acceptVersion" => {
            object_version_acceptance::accept(&runtime, &serde_json::from_value(input.clone())?)?
        }
        _ => unreachable!("catalog dispatch method"),
    };
    Ok(serde_json::to_value(receipt)?)
}

#[cfg(test)]
#[path = "object_catalog_dispatch_tests.rs"]
mod tests;
