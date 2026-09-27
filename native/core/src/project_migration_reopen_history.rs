use super::{task, Fixture};
use crate::{
    asset_delivery, asset_task, asset_work_actions,
    asset_work_contract::Action,
    asset_work_inputs,
    files::{file_hash, Files},
    store::TaskEvent,
    task_actions,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub(super) const VERSIONS: [&str; 2] = ["first frozen model", "second frozen model"];
pub(super) const CURRENT: &str = "current workspace after both attempts";
pub(super) const EVIDENCE: &str = "Saved validation output from the legacy host\n";

pub(super) struct History {
    pub approved: Value,
    pub approval_events: Vec<TaskEvent>,
    pub asset: asset_task::State,
    pub evidence: Value,
}

fn work(
    state: &mut asset_task::State,
    files: &Files,
    workspace: &Path,
    change: Value,
) -> Result<Value> {
    let action: Action = serde_json::from_value(change)?;
    let captured = match &action {
        Action::Begin { input_files, .. } => {
            Some(asset_work_inputs::capture(files, workspace, input_files)?)
        }
        _ => None,
    };
    asset_work_actions::apply(
        state,
        state.revision,
        "design",
        "legacy-thread",
        "legacy-turn",
        &action,
        captured.as_deref(),
    )
}

fn create(
    state: &mut asset_task::State,
    files: &Files,
    workspace: &Path,
    title: &str,
) -> Result<Value> {
    Ok(work(
        state,
        files,
        workspace,
        json!({"action":"create","definition":{
            "title":title,"goal":"Record inspectable work","acceptance":"Saved evidence"}
        }),
    )?["subtaskId"]
        .clone())
}

pub(super) fn seed(fixture: &mut Fixture) -> Result<History> {
    let [first, second] = fixture.projects.clone();
    let mut approved = task(fixture, "task-0", &first)?;
    approved["status"] = json!("completed");
    approved["unknown"] = json!({"legacyField":"preserved"});
    fixture.store.put("task", "task-0", &approved)?;
    task_actions::accept(&mut fixture.store, "task-0")?;
    let approved = fixture.store.get("task", "task-0")?.unwrap();
    let approval_events = fixture.store.events("task-0")?;

    let mut running = task(fixture, "task-1", &second)?;
    running["status"] = json!("running");
    running["threadId"] = json!("legacy-thread");
    running["turnId"] = json!("legacy-turn");
    fixture.store.put("task", "task-1", &running)?;
    let mut asset = asset_task::enable(&fixture.store, &running)?;
    asset_delivery::configure(
        &mut asset,
        0,
        &serde_json::from_value(json!({
            "id":"legacy-delivery","version":1,
            "stages":[{"id":"design","name":"Design"},{"id":"model","name":"Model"}]
        }))?,
    )?;
    let files = Files::new(fixture.data.clone());
    let workspace = fixture.data.join("workspaces/task-1");
    let subtask = create(&mut asset, &files, &workspace, "Revise the same source")?;
    for (index, bytes) in VERSIONS.into_iter().enumerate() {
        if index > 0 {
            work(
                &mut asset,
                &files,
                &workspace,
                json!({"action":"reopen",
                "subtaskId":subtask,"reason":"Use the revised source"}),
            )?;
        }
        fs::write(workspace.join("model.blend"), bytes)?;
        let attempt = work(
            &mut asset,
            &files,
            &workspace,
            json!({"action":"begin",
            "subtaskId":subtask,"inputs":{"revision":index},
            "recoveryNote":"Inspected the source before this attempt",
            "inputFiles":[{"path":"model.blend","role":"source"}]}),
        )?;
        work(
            &mut asset,
            &files,
            &workspace,
            json!({"action":"finish",
            "attemptId":attempt["attemptId"],"outcome":"completed",
            "summary":"Saved historical source","outputs":{"path":"model.blend"},
            "tools":[{"name":"legacy-file-writer"}]}),
        )?;
    }
    let cancelled = create(&mut asset, &files, &workspace, "Optional discarded work")?;
    work(
        &mut asset,
        &files,
        &workspace,
        json!({"action":"cancel",
        "subtaskId":cancelled,"reason":"Owner cancelled this optional change"}),
    )?;
    fs::write(workspace.join("model.blend"), CURRENT)?;
    let unfinished = create(&mut asset, &files, &workspace, "Work active at backup")?;
    work(
        &mut asset,
        &files,
        &workspace,
        json!({"action":"begin",
        "subtaskId":unfinished,"inputs":{},"inputFiles":[]}),
    )?;
    asset_task::save(&fixture.store, &asset)?;

    let mut queued = task(fixture, "task-queued", &second)?;
    queued["status"] = json!("queued");
    fixture.store.put("task", "task-queued", &queued)?;
    fixture.store.put(
        "task",
        "task-retained",
        &json!({
            "id":"task-retained","projectId":second,"status":"queued",
            "workspace":fixture.temp.path().join("external/task-retained")
        }),
    )?;
    fixture.store.event(
        "task-retained",
        "legacy-time",
        "system",
        "Unconverted history",
    )?;

    let evidence_path = fixture.data.join("validation/run-1/run.log");
    fs::create_dir_all(evidence_path.parent().unwrap())?;
    fs::write(&evidence_path, EVIDENCE)?;
    let hash = file_hash(&evidence_path)?.context("Evidence fixture missing")?;
    let evidence = json!({"id":"run-1","projectId":second,"status":"completed",
        "evidence":[{"file":"run.log","sha256":hash}],"unknown":{"legacy":true}});
    fixture.store.put("validationRun", "run-1", &evidence)?;
    Ok(History {
        approved,
        approval_events,
        asset,
        evidence,
    })
}
