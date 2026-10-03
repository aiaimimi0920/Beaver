use crate::{resources, tasks, workflow, Host};
use anyhow::Result;
use beaver_core::{call_log, preferences};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn call(host: Arc<Host>, method: String, input: Value) -> Result<Value> {
    if method.starts_with("external.") {
        return crate::external::call(&host, &method, input).await;
    }
    if matches!(
        method.as_str(),
        "task.answer" | "task.continue" | "task.interrupt"
    ) {
        return tasks::control(host, &method, input).await;
    }
    tokio::task::spawn_blocking(move || dispatch(&host, &method, input)).await?
}

fn dispatch(host: &Host, method: &str, input: Value) -> Result<Value> {
    match method {
        "project.create" | "project.npr.install" | "workflow.list" | "workflow.run" => {
            workflow::call(host, method, input)
        }
        "task.create" => {
            let task = tasks::create(host, input)?;
            host.scheduler.wake().map_err(anyhow::Error::msg)?;
            Ok(task)
        }
        "task.events" => {
            let id = input["id"].as_str().unwrap();
            let runtime = crate::runtime::task(&host.router, id)?;
            let handle = runtime.store();
            let store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            Ok(serde_json::to_value(store.events(id)?)?)
        }
        "state" => state(host),
        "settings.get" => {
            let store = host
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Host store lock unavailable"))?;
            preferences::read(&store, resources::defaults()?)
        }
        "settings.save" => {
            let mut store = host
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Host store lock unavailable"))?;
            preferences::save(
                &mut store,
                &preferences::SystemVault,
                input["settings"].clone(),
                json!({}),
            )
        }
        "logs.query" => {
            let handle = if let Some(id) = input["taskId"].as_str() {
                crate::runtime::task(&host.router, id)?.store()
            } else if let Some(id) = input["projectId"].as_str() {
                host.router.runtime_for_project(id)?.store()
            } else {
                host.store.clone()
            };
            let store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Log store lock unavailable"))?;
            call_log::query(&store, &input)
        }
        _ => anyhow::bail!("Unknown headless method"),
    }
}

fn state(host: &Host) -> Result<Value> {
    let (projects, tasks) = host.router.open_state_records()?;
    let store = host
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Host store lock unavailable"))?;
    Ok(json!({
        "projects": projects, "tasks": tasks,
        "settings": preferences::read(&store, resources::defaults()?)?,
        "features": serde_json::from_str::<Value>(include_str!("../../../dist-native/features.json"))?,
        "host": {"kind":"headless", "storage":"project-local", "objectScheduling":false}
    }))
}
