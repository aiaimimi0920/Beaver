use super::{model::Run, repository, requests};
use crate::{
    files::{safe_path, Files},
    store::Store,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::ErrorKind, path::Path};

fn watch(store: &Store, project: &str) -> Result<String> {
    let record = repository::project(store, project)?;
    let root = Path::new(record["path"].as_str().context("Missing project path")?);
    let manifest = match std::fs::read(safe_path(root, "beaver.validation.json")?) {
        Ok(bytes) => {
            anyhow::ensure!(
                bytes.len() <= 2 * 1024 * 1024,
                "Validation manifest exceeds 2 MiB"
            );
            Some(format!("{:x}", Sha256::digest(bytes)))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    repository::digest(&json!([manifest, repository::flows(store, project)?]))
}

pub fn refresh(store: &mut Store, data: &Path, coverage: &mut Value) -> Result<()> {
    let id = coverage["taskId"]
        .as_str()
        .context("Missing coverage task")?
        .to_owned();
    if coverage["status"] == "queued" {
        let ids: Vec<String> = serde_json::from_value(coverage["runIds"].clone())?;
        let runs: Vec<Run> = ids
            .iter()
            .map(|id| repository::get(store, "validationRun", id))
            .collect::<Result<_>>()?;
        if runs
            .iter()
            .any(|run| ["queued", "running"].contains(&run.status.as_str()))
        {
            return Ok(());
        }
        let complete = !runs.is_empty() && runs.iter().all(|run| run.status == "completed");
        coverage["status"] = json!(if complete { "completed" } else { "failed" });
        coverage["reason"] = if complete {
            Value::Null
        } else {
            json!("画面流程未全部完成，请查看运行记录并重跑")
        };
        return Ok(());
    }
    if !["pending", "missing", "retired"].contains(&coverage["status"].as_str().unwrap_or("")) {
        return Ok(());
    }
    let project = coverage["projectId"]
        .as_str()
        .context("Missing coverage project")?
        .to_owned();
    let task: Value = repository::get(store, "task", &id)?;
    anyhow::ensure!(
        task["projectId"] == project,
        "Coverage belongs to another project"
    );
    if task["status"] != "completed" || task["accepted"] != true {
        return Ok(());
    }
    let signature = watch(store, &project)?;
    if coverage["status"] != "pending" && coverage["watchSignature"] == signature {
        return Ok(());
    }
    let files = Files::new(data.into());
    let snapshot = repository::snapshot(store, &files, &project)?;
    repository::import_manifest(store, &files, &project, &snapshot)?;
    coverage["watchSignature"] = json!(watch(store, &project)?);
    let linked: Vec<_> = repository::flows(store, &project)?
        .into_iter()
        .filter(|flow| flow.definition.task_ids.contains(&id))
        .collect();
    coverage["flowIds"] = json!(linked.iter().map(|flow| &flow.id).collect::<Vec<_>>());
    if linked.is_empty() {
        coverage["status"] = json!("missing");
        coverage["reason"] = json!("缺少可运行流程：请登记与此任务关联的画面流程");
        return Ok(());
    }
    let active: Vec<_> = linked
        .iter()
        .filter(|flow| flow.definition.retired_reason.is_empty())
        .collect();
    if active.is_empty() {
        coverage["status"] = json!("retired");
        coverage["reason"] = json!(linked
            .iter()
            .map(|flow| flow.definition.retired_reason.as_str())
            .collect::<Vec<_>>()
            .join("; "));
        return Ok(());
    }
    let mut records = vec![];
    let mut ids = vec![];
    for flow in active {
        let mut run = repository::build_run(
            store,
            &project,
            snapshot.clone(),
            Some(flow.clone()),
            Some(id.clone()),
            None,
        )?;
        run.managed = true;
        ids.push(run.id.clone());
        records.push(("validationRun", run.id.clone(), serde_json::to_value(run)?));
    }
    coverage["status"] = json!("queued");
    coverage["reason"] = Value::Null;
    coverage["runIds"] = json!(ids);
    records.push(("validationCoverage", id, coverage.clone()));
    requests::commit(store, records)
}
