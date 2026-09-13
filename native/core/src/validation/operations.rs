use super::{
    model::{Flow, Run},
    repository,
    requests::Request,
};
use crate::{
    files::{safe_path, Files},
    store::Store,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{io::Read, path::Path};

pub fn string<'a>(input: &'a Value, key: &str) -> Result<&'a str> {
    input[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("Missing {key}"))
}

pub fn public_run(run: &Run) -> Result<Value> {
    let mut value = serde_json::to_value(run)?;
    value.as_object_mut().unwrap().remove("snapshot");
    value["snapshotFiles"] = json!(run.snapshot.len());
    Ok(value)
}

pub fn owned_run(store: &Store, input: &Value) -> Result<Run> {
    let run: Run = repository::get(store, "validationRun", string(input, "runId")?)?;
    ensure!(
        run.project_id == string(input, "projectId")?,
        "Run belongs to another project"
    );
    Ok(run)
}

pub fn list(store: &Store, project: &str) -> Result<Value> {
    repository::project(store, project)?;
    let mut runs = store.list::<Run>("validationRun")?;
    runs.retain(|r| r.project_id == project);
    runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let runs: Result<Vec<_>> = runs
        .into_iter()
        .take(200)
        .map(|r| {
            let mut value = public_run(&r)?;
            value["log"] = json!("");
            value["evidenceCount"] = json!(r.evidence.len());
            value["evidence"] = json!([]);
            Ok(value)
        })
        .collect();
    let selected = |kind| -> Result<Vec<Value>> {
        Ok(store
            .list::<Value>(kind)?
            .into_iter()
            .filter(|v| v["projectId"] == project)
            .collect())
    };
    Ok(
        json!({"flows":repository::flows(store, project)?,"runs":runs?,"coverage":selected("validationCoverage")?,
        "feedback":selected("validationFeedback")?,"repairDecisions":selected("validationRepairDecision")?,"settings":super::settings::read(store, project)?}),
    )
}

pub fn save_flow(store: &mut Store, input: &Value) -> Result<Value> {
    let request = Request::new("validation.flow.save", input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let mut definition: super::flow::Definition =
        serde_json::from_value(input["definition"].clone())?;
    if definition.roaming.is_some() && definition.steps.is_empty() {
        let seed = definition.config.seed;
        super::roaming::generate(&mut definition, seed)?;
    }
    let expected = input["expectedRevision"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .context("Invalid expectedRevision")?;
    let project = string(input, "projectId")?;
    for task_id in &definition.task_ids {
        let task: Value = repository::get(store, "task", task_id)?;
        ensure!(
            task["projectId"] == project,
            "Flow task belongs to another project"
        );
    }
    let flow = repository::prepare_flow(
        store,
        project,
        definition,
        expected,
        input["reason"].as_str().unwrap_or(""),
    )?;
    request.finish(
        store,
        serde_json::to_value(&flow)?,
        repository::flow_records(&flow)?,
    )
}

pub fn explore(store: &mut Store, input: &Value) -> Result<Value> {
    let request = Request::new("validation.flow.explore", input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let flow: Flow = repository::get(store, "validationFlow", string(input, "flowId")?)?;
    let project = string(input, "projectId")?;
    ensure!(
        flow.project_id == project
            && input["expectedRevision"].as_u64() == Some(u64::from(flow.revision)),
        "Roaming flow changed; refresh first"
    );
    let seed = input["seed"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .context("Invalid random seed")?;
    let mut definition = flow.definition;
    definition.key = format!(
        "{}-{}",
        definition.key.chars().take(60).collect::<String>(),
        &repository::id()[..8]
    );
    definition.name = format!("{} / route {seed}", definition.name);
    definition.retired_reason.clear();
    super::roaming::generate(&mut definition, seed)?;
    let flow = repository::prepare_flow(
        store,
        project,
        definition,
        0,
        "Explicit random exploration; original route retained",
    )?;
    request.finish(
        store,
        serde_json::to_value(&flow)?,
        repository::flow_records(&flow)?,
    )
}

pub fn enqueue(store: &mut Store, data: &Path, method: &str, input: &Value) -> Result<Value> {
    let request = Request::new(method, input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let project = string(input, "projectId")?;
    let files = Files::new(data.into());
    ensure!(
        !crate::journal::Journal::new(store, &files).blocked(project)?,
        "Project has unfinished file recovery"
    );
    let snapshot = repository::snapshot(store, &files, project)?;
    repository::import_manifest(store, &files, project, &snapshot)?;
    let flows = match method {
        "validation.code.run" => vec![None],
        "validation.run.rerun" => vec![owned_run(store, input)?.flow],
        "validation.flow.run" => {
            let flow: Flow = repository::get(store, "validationFlow", string(input, "flowId")?)?;
            ensure!(
                flow.project_id == project && flow.definition.retired_reason.is_empty(),
                "Flow is retired or belongs to another project"
            );
            ensure!(
                input["expectedRevision"].as_u64() == Some(u64::from(flow.revision)),
                "Flow changed; refresh before running"
            );
            vec![Some(flow)]
        }
        "validation.run.all" => std::iter::once(None)
            .chain(
                repository::flows(store, project)?
                    .into_iter()
                    .filter(|f| f.definition.retired_reason.is_empty())
                    .map(Some),
            )
            .collect(),
        _ => bail!("Unknown run operation"),
    };
    let task_id = input["taskId"].as_str().map(str::to_owned);
    if let Some(id) = &task_id {
        let task: Value = repository::get(store, "task", id)?;
        ensure!(
            task["projectId"] == project,
            "Task belongs to another project"
        );
    }
    let mut records = vec![];
    let mut ids = vec![];
    for flow in flows {
        let mut run = repository::build_run(
            store,
            project,
            snapshot.clone(),
            flow,
            task_id.clone(),
            None,
        )?;
        run.managed = true;
        ids.push(run.id.clone());
        records.push(("validationRun", run.id.clone(), serde_json::to_value(run)?));
    }
    request.finish(
        store,
        json!({"runIds":ids,"snapshotId":repository::digest(&snapshot)?}),
        records,
    )
}

fn text_file(path: &Path) -> Result<String> {
    let mut bytes = vec![];
    std::fs::File::open(path)?
        .take(512001)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 512000,
        "Source is too large; use its resource reference"
    );
    String::from_utf8(bytes).context("Source is binary or not UTF-8")
}

pub fn source(store: &Store, data: &Path, input: &Value) -> Result<Value> {
    let run = owned_run(store, input)?;
    let relative = super::flow::relative(string(input, "path")?)?;
    let hash = run
        .snapshot
        .get(relative)
        .context("Path is absent from the recorded game snapshot")?;
    let blob = Files::new(data.into()).blob(hash)?;
    ensure!(
        crate::files::file_hash(&blob)?.as_ref() == Some(hash),
        "Historical source changed or is missing"
    );
    let historical = text_file(&blob)?;
    let project = repository::project(store, &run.project_id)?;
    let path = safe_path(
        Path::new(project["path"].as_str().context("Missing project path")?),
        relative,
    )?;
    let current = text_file(&path).ok();
    Ok(
        json!({"path":relative,"snapshotId":run.snapshot_id,"sha256":hash,"historical":historical,"current":current}),
    )
}
