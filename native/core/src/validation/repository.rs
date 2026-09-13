use super::{
    flow::Definition,
    model::{Baseline, Flow, Run},
};
use crate::{
    files::{safe_path, Files, Snapshot},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
pub fn get<T: DeserializeOwned>(store: &Store, kind: &str, id: &str) -> Result<T> {
    store.get(kind, id)?.context("Validation record not found")
}
pub fn project(store: &Store, id: &str) -> Result<Value> {
    get(store, "project", id)
}
pub fn snapshot(store: &Store, files: &Files, project_id: &str) -> Result<Snapshot> {
    let project = project(store, project_id)?;
    files.capture(Path::new(
        project["path"].as_str().context("Project path missing")?,
    ))
}
pub fn run_dir(data: &Path, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).context("Invalid run ID")?;
    std::fs::create_dir_all(data.join("validation"))?;
    safe_path(&data.join("validation"), id)
}
pub fn prepare_flow(
    store: &Store,
    project_id: &str,
    definition: Definition,
    expected: u32,
    reason: &str,
) -> Result<Flow> {
    project(store, project_id)?;
    definition.validate()?;
    let id = digest(&json!([project_id, definition.key]))?;
    let current = store.get::<Flow>("validationFlow", &id)?;
    if current.as_ref().map_or(0, |f| f.revision) != expected {
        bail!("Flow changed; refresh before saving");
    }
    if expected > 0 && reason.trim().is_empty() {
        bail!("Explain why the flow is being revised or retired");
    }
    let flow = Flow {
        id: id.clone(),
        project_id: project_id.into(),
        revision: expected + 1,
        definition,
        updated_at: now(),
        reason: reason.into(),
    };
    Ok(flow)
}

pub fn flow_records(flow: &Flow) -> Result<Vec<super::requests::Record>> {
    let value = serde_json::to_value(flow)?;
    Ok(vec![
        (
            "validationFlowRevision",
            format!("{}:{}", flow.id, flow.revision),
            value.clone(),
        ),
        ("validationFlow", flow.id.clone(), value),
    ])
}

pub fn save_flow(
    store: &mut Store,
    project_id: &str,
    definition: Definition,
    expected: u32,
) -> Result<Flow> {
    let flow = prepare_flow(
        store,
        project_id,
        definition,
        expected,
        "Updated by validation manifest",
    )?;
    super::requests::commit(store, flow_records(&flow)?)?;
    Ok(flow)
}

pub fn flows(store: &Store, project_id: &str) -> Result<Vec<Flow>> {
    Ok(store
        .list::<Flow>("validationFlow")?
        .into_iter()
        .filter(|f| f.project_id == project_id)
        .collect())
}
pub fn baseline(store: &Store, flow: &Flow) -> Result<Option<Baseline>> {
    let signature = flow.definition.signature()?;
    let mut all = store.list::<Baseline>("validationBaseline")?;
    all.retain(|b| {
        b.project_id == flow.project_id && b.flow_id == flow.id && b.signature == signature
    });
    all.sort_by(|a, b| b.confirmed_at.cmp(&a.confirmed_at));
    Ok(all.into_iter().next())
}

pub fn build_run(
    store: &Store,
    project_id: &str,
    snapshot: Snapshot,
    flow: Option<Flow>,
    task_id: Option<String>,
    release_id: Option<String>,
) -> Result<Run> {
    project(store, project_id)?;
    let baseline_id = flow
        .as_ref()
        .map(|f| baseline(store, f))
        .transpose()?
        .flatten()
        .map(|b| b.id);
    let run = Run {
        id: id(),
        project_id: project_id.into(),
        task_id,
        release_id,
        kind: if flow.is_some() { "visual" } else { "code" }.into(),
        managed: false,
        created_at: now(),
        finished_at: None,
        snapshot_id: digest(&snapshot)?,
        snapshot,
        flow,
        baseline_id,
        runner_version: super::RUNNER_VERSION.into(),
        engine_version: String::new(),
        status: "queued".into(),
        phase: "queued".into(),
        completed_steps: 0,
        verdict: "unjudged".into(),
        error: None,
        log: String::new(),
        evidence: vec![],
        code: None,
        judgments: vec![],
        confirmations: vec![],
    };
    Ok(run)
}

pub fn new_run(
    store: &Store,
    project_id: &str,
    snapshot: Snapshot,
    flow: Option<Flow>,
    task_id: Option<String>,
    release_id: Option<String>,
) -> Result<Run> {
    let run = build_run(store, project_id, snapshot, flow, task_id, release_id)?;
    store.put("validationRun", &run.id, &run)?;
    Ok(run)
}

pub fn recover(store: &Store) -> Result<()> {
    for mut run in store.list::<Run>("validationRun")? {
        if ["queued", "running"].contains(&run.status.as_str()) {
            run.status = "interrupted".into();
            run.verdict = "needsReview".into();
            run.error = Some(
                "Application stopped before this run completed; rerun to collect new evidence"
                    .into(),
            );
            run.finished_at = Some(now());
            store.put("validationRun", &run.id, &run)?;
        }
    }
    Ok(())
}

pub use super::manifest::import_manifest;
