use beaver_core::{files::Files, store::Store};
use serde_json::Value;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub(crate) struct TaskRuntimeHandles {
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) files: Arc<Files>,
    pub(crate) project_routed: bool,
}

pub(crate) struct CallLogContext {
    pub(crate) handles: TaskRuntimeHandles,
    pub(crate) task_id: Option<String>,
    pub(crate) project_id: Option<String>,
}

fn legacy_task_handles(host_store: Arc<Mutex<Store>>, host_root: &Path) -> TaskRuntimeHandles {
    TaskRuntimeHandles {
        store: host_store,
        files: Arc::new(Files::new(host_root.to_path_buf())),
        project_routed: false,
    }
}

pub(crate) fn task_runtime_handles(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    task_id: &str,
) -> Result<TaskRuntimeHandles, String> {
    match router.runtime_for_task(task_id) {
        Ok(runtime) => Ok(TaskRuntimeHandles {
            store: runtime.store(),
            files: runtime.files(),
            project_routed: true,
        }),
        Err(project_error) => {
            let host_task = {
                let store = host_store.lock().map_err(|_| "数据库锁不可用".to_owned())?;
                store
                    .get::<Value>("task", task_id)
                    .map_err(|error| error.to_string())?
                    .and_then(|task| {
                        let project_id = task["projectId"].as_str()?.to_owned();
                        Some((task, project_id))
                    })
            };
            if let Some((_task, project_id)) = host_task {
                let host_has_project = host_store
                    .lock()
                    .map_err(|_| "数据库锁不可用".to_owned())?
                    .get::<Value>("project", &project_id)
                    .map_err(|error| error.to_string())?
                    .is_some();
                if router.runtime_for_project(&project_id).is_ok() {
                    let runtime = router
                        .refresh_open_task_route(&project_id, task_id)
                        .map_err(|error| {
                            format!("项目 Runtime 已打开，但任务未能从项目存储解析：{error}")
                        })?;
                    return Ok(TaskRuntimeHandles {
                        store: runtime.store(),
                        files: runtime.files(),
                        project_routed: true,
                    });
                }
                if !host_has_project {
                    return Err(format!("项目未登记，禁止回退到宿主任务：{project_id}"));
                }
                if router
                    .registered_project_uses_local_storage(&project_id)
                    .map_err(|error| error.to_string())?
                {
                    return Err(format!(
                        "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                    ));
                }
                return Ok(legacy_task_handles(host_store, host_root));
            }
            Err(project_error.to_string())
        }
    }
}

pub(crate) fn project_runtime_handles(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    project_id: &str,
) -> Result<TaskRuntimeHandles, String> {
    match router.runtime_for_project(project_id) {
        Ok(runtime) => Ok(TaskRuntimeHandles {
            store: runtime.store(),
            files: runtime.files(),
            project_routed: true,
        }),
        Err(project_error) => {
            if router
                .registered_project_uses_local_storage(project_id)
                .map_err(|error| error.to_string())?
            {
                return Err(format!(
                    "项目本地存储未打开，禁止回退到宿主存储：{project_error}"
                ));
            }
            let host_has_project = host_store
                .lock()
                .map_err(|_| "数据库锁不可用".to_owned())?
                .get::<Value>("project", project_id)
                .map_err(|error| error.to_string())?
                .is_some();
            if host_has_project {
                Ok(legacy_task_handles(host_store, host_root))
            } else {
                Err(project_error.to_string())
            }
        }
    }
}

pub(crate) fn query_runtime_handles(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    input: &Value,
) -> Result<TaskRuntimeHandles, String> {
    if input.get("taskId").is_some() {
        let task_id = input["taskId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "Invalid log filter".to_owned())?;
        let handles = task_runtime_handles(router, host_store, host_root, task_id)?;
        if input.get("projectId").is_some() {
            let project_id = input["projectId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| "Invalid log filter".to_owned())?;
            ensure_task_project(&handles, task_id, project_id)?;
        }
        return Ok(handles);
    }
    if input.get("projectId").is_some() {
        let project_id = input["projectId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "Invalid log filter".to_owned())?;
        return project_runtime_handles(router, host_store, host_root, project_id);
    }
    Ok(legacy_task_handles(host_store, host_root))
}

fn task_project_id(handles: &TaskRuntimeHandles, task_id: &str) -> Result<Option<String>, String> {
    let task = handles
        .store
        .lock()
        .map_err(|_| "数据库锁不可用".to_owned())?
        .get::<Value>("task", task_id)
        .map_err(|error| error.to_string())?;
    Ok(task.and_then(|task| task["projectId"].as_str().map(str::to_owned)))
}

fn ensure_task_project(
    handles: &TaskRuntimeHandles,
    task_id: &str,
    project_id: &str,
) -> Result<(), String> {
    if let Some(task_project_id) = task_project_id(handles, task_id)? {
        if task_project_id != project_id {
            return Err("任务与项目标识不一致".into());
        }
    }
    Ok(())
}

fn project_id_for_method(method: &str, input: &Value) -> Option<String> {
    let project_scoped = method.starts_with("project.")
        && !matches!(method, "project.create" | "project.import")
        || method.starts_with("workflow.")
        || matches!(
            method,
            "assets"
                | "asset.text"
                | "asset.reveal"
                | "document.read"
                | "document.save"
                | "game.play"
                | "game.presets"
                | "game.export"
                | "game.verifyExport"
        );
    project_scoped
        .then(|| input["id"].as_str())
        .flatten()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

fn export_path_project_id(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    method: &str,
    input: &Value,
) -> Result<Option<String>, String> {
    if method != "game.verifyExport" {
        return Ok(None);
    }
    let Some(path) = input["path"].as_str().filter(|path| !path.is_empty()) else {
        return Ok(None);
    };
    crate::game_storage::project_id_for_export_path(router, host_store, path)
        .map_err(|error| error.to_string())
}

pub(crate) fn call_log_context(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    host_root: &Path,
    method: &str,
    input: &Value,
) -> Result<CallLogContext, String> {
    let task_id = if method.starts_with("task.") || method.starts_with("assetTask.") {
        input["id"].as_str()
    } else {
        input["taskId"].as_str()
    }
    .filter(|id| !id.is_empty())
    .map(str::to_owned);
    let explicit_project_id = input["projectId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned);
    let method_project_id = project_id_for_method(method, input);
    if let (Some(explicit), Some(method_project)) =
        (explicit_project_id.as_deref(), method_project_id.as_deref())
    {
        if explicit != method_project {
            return Err("项目与请求标识不一致".into());
        }
    }
    let explicit_project_id = explicit_project_id.or(method_project_id);
    // Registration inspection and mutations must work offline without retaining a runtime.
    if matches!(
        method,
        "project.unregister" | "project.reassociate" | "project.storage.status"
    ) {
        return Ok(CallLogContext {
            handles: legacy_task_handles(host_store, host_root),
            task_id: None,
            project_id: explicit_project_id,
        });
    }
    let export_project_id = export_path_project_id(router, host_store.clone(), method, input)?;
    if let (Some(explicit), Some(export_project)) =
        (explicit_project_id.as_deref(), export_project_id.as_deref())
    {
        if explicit != export_project {
            return Err("项目与导出路径归属不一致".into());
        }
    }
    let explicit_project_id = explicit_project_id.or(export_project_id);

    if let Some(task_id) = task_id.clone() {
        match task_runtime_handles(router, host_store.clone(), host_root, &task_id) {
            Ok(handles) => {
                let task_project_id = task_project_id(&handles, &task_id)?;
                if let Some(project_id) = explicit_project_id.as_deref() {
                    ensure_task_project(&handles, &task_id, project_id)?;
                }
                return Ok(CallLogContext {
                    handles,
                    task_id: Some(task_id),
                    project_id: explicit_project_id.or(task_project_id),
                });
            }
            Err(task_error) => {
                if let Some(project_id) = explicit_project_id.clone() {
                    let handles =
                        project_runtime_handles(router, host_store, host_root, &project_id)?;
                    return Ok(CallLogContext {
                        handles,
                        task_id: Some(task_id),
                        project_id: Some(project_id),
                    });
                }
                return Err(task_error);
            }
        }
    }

    if let Some(project_id) = explicit_project_id {
        let handles = project_runtime_handles(router, host_store, host_root, &project_id)?;
        return Ok(CallLogContext {
            handles,
            task_id: None,
            project_id: Some(project_id),
        });
    }

    Ok(CallLogContext {
        handles: legacy_task_handles(host_store, host_root),
        task_id: None,
        project_id: None,
    })
}

#[path = "business_routing_dispatch.rs"]
mod dispatch;

pub(crate) use dispatch::call;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "project_registration_routing_tests.rs"]
mod registration_tests;
