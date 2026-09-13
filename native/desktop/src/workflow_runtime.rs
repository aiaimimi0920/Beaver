use crate::Backend;
use anyhow::{bail, Context, Result};
use beaver_core::{files::Files, journal::Journal, workflows};
use serde_json::{json, Value};
use std::{path::Path, sync::atomic::Ordering};

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
        let mut project = {
            let mut store = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            if method == "project.create" {
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
                )?
            } else {
                let id = input["id"].as_str().context("Missing project id")?;
                let project: Value = store.get("project", id)?.context("Project not found")?;
                if Journal::new(&mut store, &Files::new(backend.root.clone())).blocked(id)? {
                    bail!("Project recovery pending");
                }
                if method != "workflow.list"
                    && store.list::<Value>("task")?.iter().any(|t| {
                        t["projectId"] == id
                            && matches!(t["status"].as_str(), Some("running" | "queued"))
                    })
                {
                    bail!("Use the task-scoped workflow MCP while creation is active");
                }
                project
            }
        };
        if let Some(options) = options {
            project["npr"] = json!({"status":"installing","godot":options.godot});
            {
                let store = backend
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
                store.put("project", project["id"].as_str().unwrap(), &project)?;
            }
            let root = Path::new(project["path"].as_str().context("Invalid project path")?);
            let result = workflows::install(root, &options.godot, &backend.closing);
            project["npr"] = match &result {
                Ok(runtime) => json!({"status":"ready","runtime":runtime}),
                Err(error) => {
                    json!({"status":"failed","godot":options.godot,"error":error.to_string()})
                }
            };
            {
                let store = backend
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
                store.put("project", project["id"].as_str().unwrap(), &project)?;
            }
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
