use super::{
    evidence,
    model::{Flow, Release, Run},
    repository,
    requests::{Record, Request},
    settings,
};
use crate::{files::Files, store::Store};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

pub fn scope(release: &Release) -> Result<String> {
    repository::digest(&json!([
        release.project_id,
        release.snapshot_id,
        release.preset,
        release.visual_required,
        release.policy_version,
        release.flows,
        release.excluded,
        release.missing
    ]))
}

pub fn start(store: &mut Store, data: &Path, input: &Value) -> Result<Value> {
    let request = Request::new("validation.release.start", input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let project = super::operations::string(input, "projectId")?;
    let preset = super::operations::string(input, "preset")?;
    let files = Files::new(data.into());
    ensure!(
        !crate::journal::Journal::new(store, &files).blocked(project)?,
        "Project has unfinished file recovery"
    );
    let snapshot = repository::snapshot(store, &files, project)?;
    crate::game_export::snapshot_preset(data, &snapshot, preset)?;
    repository::import_manifest(store, &files, project, &snapshot)?;
    let settings = settings::read(store, project)?;
    let registered = repository::flows(store, project)?;
    let flows: Vec<Flow> = registered
        .iter()
        .filter(|f| f.definition.retired_reason.is_empty())
        .cloned()
        .collect();
    let excluded: Vec<Value> = registered.iter().filter(|f| !f.definition.retired_reason.is_empty())
        .map(|f| json!({"flowId":f.id,"revision":f.revision,"name":f.definition.name,"reason":f.definition.retired_reason})).collect();
    let mut missing = vec![];
    if flows.is_empty() {
        missing.push("No applicable visual flows have been registered".into());
    }
    for coverage in store
        .list::<Value>("validationCoverage")?
        .into_iter()
        .filter(|v| v["projectId"] == project)
    {
        let task = coverage["taskId"]
            .as_str()
            .context("Coverage task missing")?;
        if !registered
            .iter()
            .any(|f| f.definition.task_ids.iter().any(|id| id == task))
        {
            missing.push(format!(
                "Task {} has no registered visual flow ({task})",
                coverage["title"].as_str().unwrap_or(task)
            ));
        }
    }
    let mut release = Release {
        id: repository::id(),
        project_id: project.into(),
        snapshot_id: repository::digest(&snapshot)?,
        snapshot,
        preset: preset.into(),
        visual_required: settings.visual_required,
        policy_version: format!("{}:{}", super::RUNNER_VERSION, settings.revision),
        flow_ids: flows.iter().map(|f| f.id.clone()).collect(),
        flows,
        scope_id: String::new(),
        excluded,
        missing,
        run_ids: vec![],
        created_at: repository::now(),
        exports: vec![],
    };
    release.scope_id = scope(&release)?;
    let mut records: Vec<Record> = vec![];
    let selected = std::iter::once(None).chain(
        release
            .flows
            .iter()
            .filter(|_| release.visual_required)
            .cloned()
            .map(Some),
    );
    for flow in selected {
        let mut run = repository::build_run(
            store,
            project,
            release.snapshot.clone(),
            flow,
            None,
            Some(release.id.clone()),
        )?;
        run.managed = true;
        release.run_ids.push(run.id.clone());
        records.push(("validationRun", run.id.clone(), serde_json::to_value(run)?));
    }
    records.push((
        "validationRelease",
        release.id.clone(),
        serde_json::to_value(&release)?,
    ));
    request.finish(store, json!({"id":release.id,"snapshotId":release.snapshot_id,"scopeId":release.scope_id,"runIds":release.run_ids}), records)
}

pub fn inspect(store: &Store, release: &Release) -> Result<Value> {
    ensure!(
        release.scope_id == scope(release)?
            && release.snapshot_id == repository::digest(&release.snapshot)?,
        "Release candidate or scope has changed"
    );
    let mut items = vec![];
    let mut unique = BTreeSet::new();
    let mut code_count = 0;
    let mut seen_flows = BTreeSet::new();
    let mut green = !release.visual_required || release.missing.is_empty();
    for id in &release.run_ids {
        ensure!(unique.insert(id), "Duplicate release run");
        let run: Run = repository::get(store, "validationRun", id)?;
        ensure!(
            run.project_id == release.project_id
                && run.release_id.as_deref() == Some(&release.id)
                && run.snapshot_id == release.snapshot_id
                && run.snapshot == release.snapshot
                && run.runner_version == super::RUNNER_VERSION,
            "Run does not belong to the frozen release candidate"
        );
        let passed = if run.kind == "code" {
            code_count += 1;
            super::code::green(&run)
        } else {
            let flow = run.flow.as_ref().context("Visual run has no flow")?;
            ensure!(
                release.visual_required
                    && seen_flows.insert(flow.id.clone())
                    && release.flows.iter().any(|f| f.id == flow.id
                        && f.revision == flow.revision
                        && f.definition == flow.definition),
                "Unexpected visual scope"
            );
            super::judgment::green(store, &run)?
        };
        green &= passed;
        items.push(json!({"runId":id,"kind":run.kind,"flowId":run.flow.as_ref().map(|f| &f.id),"status":run.status,"phase":run.phase,"verdict":run.verdict,"passed":passed,"error":run.error}));
    }
    ensure!(
        code_count == 1 && (!release.visual_required || seen_flows.len() == release.flows.len()),
        "Release check is missing required runs"
    );
    let mut result = serde_json::to_value(release)?;
    result.as_object_mut().unwrap().remove("snapshot");
    result["items"] = json!(items);
    result["ready"] = json!(green);
    if green {
        result["ready"] = json!(engine_version(store, release).is_ok());
    }
    Ok(result)
}

pub fn display(store: &Store, release: &Release) -> Result<Value> {
    match inspect(store, release) {
        Ok(value) => Ok(value),
        Err(error) => {
            let mut value = serde_json::to_value(release)?;
            value.as_object_mut().unwrap().remove("snapshot");
            value["ready"] = json!(false);
            value["integrityError"] = json!(error.to_string());
            Ok(value)
        }
    }
}

pub fn engine_version(store: &Store, release: &Release) -> Result<String> {
    let versions: Result<BTreeSet<String>> = release
        .run_ids
        .iter()
        .map(|id| Ok(repository::get::<Run>(store, "validationRun", id)?.engine_version))
        .collect();
    let versions = versions?;
    ensure!(
        versions.len() == 1 && !versions.contains(""),
        "All release checks must use the same Godot version"
    );
    Ok(versions.into_iter().next().unwrap())
}

/// All export entry points must use this check and the returned immutable snapshot.
pub fn authorize(store: &Store, data: &Path, input: &Value) -> Result<Release> {
    let id = super::operations::string(input, "releaseCheckId")?;
    let release: Release = repository::get(store, "validationRelease", id)?;
    ensure!(
        input["id"] == release.project_id
            && input["snapshotId"] == release.snapshot_id
            && input["scopeId"] == release.scope_id
            && input["preset"] == release.preset,
        "Export does not match the reviewed candidate, preset and scope"
    );
    ensure!(
        inspect(store, &release)?["ready"] == true,
        "Required code or visual checks are not green for this release"
    );
    for id in &release.run_ids {
        let run: Run = repository::get(store, "validationRun", id)?;
        if run.kind == "visual" {
            evidence::validate(data, &run)?;
            if run.verdict == "autoPassed" {
                let (_, baseline) = super::comparison::baseline_run(store, &run)?
                    .context("Comparison baseline missing")?;
                evidence::validate(data, &baseline)?;
            }
        } else {
            super::code::validate(data, &run)?;
        }
    }
    Ok(release)
}
