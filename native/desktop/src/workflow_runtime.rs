use crate::{project_runtime_lifecycle, Backend};
use anyhow::{bail, Context, Result};
use beaver_core::{
    files::Files, journal::Journal, project_runtime::ProjectRuntime,
    project_storage_router::ProjectStorageRouter, workflows,
};
use serde_json::{json, Value};
use std::{path::Path, sync::atomic::Ordering};

fn project_id(project: &Value) -> Result<&str> {
    project["id"].as_str().context("Invalid project id")
}

fn project_runtime(backend: &Backend, id: &str) -> Result<Option<ProjectRuntime>> {
    project_runtime_for(&backend.project_storage, id)
}

fn project_runtime_for(router: &ProjectStorageRouter, id: &str) -> Result<Option<ProjectRuntime>> {
    if let Ok(runtime) = router.runtime_for_project(id) {
        return Ok(Some(runtime));
    }
    if router.registered_project_uses_local_storage(id)? {
        Ok(Some(project_runtime_lifecycle::open_registered(
            router, id,
        )?))
    } else {
        Ok(None)
    }
}

fn load_project(
    backend: &Backend,
    id: &str,
    method: &str,
) -> Result<(Value, Option<ProjectRuntime>)> {
    let runtime = project_runtime(backend, id)?;
    let (store_handle, files) = if let Some(runtime) = &runtime {
        (runtime.store(), runtime.files())
    } else {
        (
            backend.store.clone(),
            std::sync::Arc::new(Files::new(backend.root.clone())),
        )
    };
    let mut store = store_handle
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
    let project: Value = store.get("project", id)?.context("Project not found")?;
    if Journal::new(&mut store, &files).blocked(id)? {
        bail!("Project recovery pending");
    }
    if method != "workflow.list"
        && store.list::<Value>("task")?.iter().any(|task| {
            task["projectId"] == id && matches!(task["status"].as_str(), Some("running" | "queued"))
        })
    {
        bail!("Use the task-scoped workflow MCP while creation is active");
    }
    Ok((project, runtime))
}

#[cfg(test)]
mod tests {
    use super::project_runtime_for;
    use beaver_core::{
        project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
    };
    use serde_json::json;
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    fn project_root(parent: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
        let root = parent.join("project");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        Ok(root)
    }

    #[test]
    fn opened_runtime_does_not_reparse_host_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = project_root(temp.path())?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let project = json!({
            "id": "project-a",
            "name": "Runtime project",
            "path": root,
        });
        let project_store = ProjectStore::initialize(&root, "project-a")?;
        project_store
            .store()
            .put("project", "project-a", &project)?;
        drop(project_store);
        host.lock()
            .map_err(|_| anyhow::anyhow!("host store lock unavailable"))?
            .put(
                "project",
                "project-a",
                &json!({
                    "id": "project-a",
                    "path": root,
                }),
            )?;
        let router = ProjectStorageRouter::new(host.clone());
        router.open_registered("project-a")?;
        host.lock()
            .map_err(|_| anyhow::anyhow!("host store lock unavailable"))?
            .put(
                "project",
                "project-a",
                &json!({
                    "id": "project-a",
                    "path": temp.path().join("missing-project"),
                }),
            )?;
        let runtime = project_runtime_for(&router, "project-a")?.expect("open runtime");
        assert_eq!(runtime.project_root(), fs::canonicalize(root)?);
        Ok(())
    }
}

fn persist_project(
    backend: &Backend,
    runtime: Option<&ProjectRuntime>,
    project: &Value,
) -> Result<()> {
    let id = project_id(project)?;
    let store_handle = if let Some(runtime) = runtime {
        if runtime.project_id() != id {
            bail!("项目运行时与项目实体不一致");
        }
        runtime.store()
    } else {
        backend.store.clone()
    };
    store_handle
        .lock()
        .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
        .put("project", id, project)?;
    Ok(())
}

pub fn call(backend: &Backend, method: &str, mut input: Value) -> Result<Value, String> {
    (|| -> Result<Value> {
        let _operation = backend
            .operations
            .lock()
            .map_err(|_| anyhow::anyhow!("工具操作锁不可用"))?;
        if backend.closing.load(Ordering::SeqCst) {
            bail!("应用正在退出");
        }
        let options = if method == "project.create" {
            input
                .as_object_mut()
                .and_then(|v| v.remove("npr"))
                .map(serde_json::from_value::<workflows::NprOptions>)
                .transpose()?
        } else if method == "project.npr.install" {
            Some(workflows::NprOptions {
                godot: input["godot"]
                    .as_str()
                    .context("Missing custom engine")?
                    .to_owned(),
            })
        } else {
            None
        };
        if let Some(options) = &options {
            workflows::engine_path(&options.godot)?;
        }
        let (mut project, loaded_runtime) = if method == "project.create" {
            let store = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            (
                beaver_core::projects::create_project(
                    &store,
                    &backend.root,
                    input.clone(),
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/design-catalog.json"
                    ))?,
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/blueprint-catalog.json"
                    ))?,
                    &serde_json::from_str(include_str!("../../../dist-native/templates.json"))?,
                )?,
                None,
            )
        } else {
            let id = input["id"].as_str().context("Missing project id")?;
            load_project(backend, id, method)?
        };
        let runtime = match loaded_runtime {
            Some(runtime) => Some(runtime),
            None => project_runtime(backend, project_id(&project)?)?,
        };
        if let Some(options) = options {
            project["npr"] = json!({"status":"installing","godot":options.godot});
            persist_project(backend, runtime.as_ref(), &project)?;
            let root = Path::new(project["path"].as_str().context("Invalid project path")?);
            let result = workflows::install(root, &options.godot, &backend.closing);
            project["npr"] = match &result {
                Ok(runtime) => json!({"status":"ready","runtime":runtime}),
                Err(error) => {
                    json!({"status":"failed","godot":options.godot,"error":error.to_string()})
                }
            };
            persist_project(backend, runtime.as_ref(), &project)?;
            if let Err(error) = result {
                bail!("项目 {} 已保留；NPR 接入失败：{error}", project["id"]);
            }
        }
        if method.starts_with("project.") {
            return Ok(project);
        }
        let root = Path::new(project["path"].as_str().context("Invalid project path")?);
        if method == "workflow.list" {
            return workflows::list(root);
        }
        input
            .as_object_mut()
            .context("Invalid workflow input")?
            .remove("id");
        workflows::run(root, input, &backend.closing)
    })()
    .map_err(|e| e.to_string())
}
