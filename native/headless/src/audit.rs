use crate::Host;
use anyhow::Result;
use beaver_core::call_log;
use serde_json::{json, Value};
use std::time::Instant;

pub struct Record {
    id: String,
    started: Instant,
    task: Option<String>,
    project: Option<String>,
}

pub fn begin(host: &Host, method: &str, input: &Value) -> Result<Record> {
    let task = if method.starts_with("external.") {
        input["taskId"].as_str()
    } else if method.starts_with("task.") {
        input["id"].as_str()
    } else {
        None
    }
    .map(str::to_owned);
    let project = input["projectId"]
        .as_str()
        .or_else(|| {
            (method.starts_with("project.") || method.starts_with("workflow."))
                .then(|| input["id"].as_str())
                .flatten()
        })
        .map(str::to_owned);
    let project = project.or_else(|| {
        task.as_deref().and_then(|id| {
            crate::runtime::task(&host.router, id)
                .ok()
                .map(|runtime| runtime.project_id().to_owned())
        })
    });
    let store = host
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Host log lock unavailable"))?;
    let id = call_log::begin(
        &store,
        "api",
        method,
        task.as_deref(),
        project.as_deref(),
        input,
    )?;
    Ok(Record {
        id,
        started: Instant::now(),
        task,
        project,
    })
}

pub fn finish(host: &Host, mut record: Record, method: &str, result: &Result<Value>) -> Result<()> {
    if let Ok(value) = result {
        if method == "task.create" {
            record.task = value["id"].as_str().map(str::to_owned);
            record.project = value["projectId"].as_str().map(str::to_owned);
        } else if method == "project.create" {
            record.project = value["id"].as_str().map(str::to_owned);
        }
    }
    // Resolve the router before acquiring any Store: registry -> Store lock order.
    let local = record
        .project
        .as_deref()
        .and_then(|id| host.router.runtime_for_project(id).ok());
    let host_store = host
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Host log lock unavailable"))?;
    call_log::link(
        &host_store,
        &record.id,
        record.task.as_deref(),
        record.project.as_deref(),
    )?;
    let (status, output) = match result {
        Ok(value) => ("succeeded", value.clone()),
        Err(error) => ("failed", json!({"error":error.to_string()})),
    };
    call_log::finish(
        &host_store,
        &record.id,
        status,
        record
            .started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
        &output,
    )?;
    if let Some(runtime) = local {
        let handle = runtime.store();
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("Project log lock unavailable"))?;
        call_log::copy_record(&host_store, &store, &record.id)?;
    }
    Ok(())
}
