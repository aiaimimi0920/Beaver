use anyhow::{ensure, Context, Result};
use beaver_core::{
    asset_delivery, asset_task, files::Files, framework_contract, store::Store, task_callback,
    task_callback_contract, task_callback_runtime,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub fn validate_tools(request: &Value) -> Result<()> {
    let tools = request["params"]["dynamicTools"]
        .as_array()
        .context("Missing dynamic tools")?;
    ensure!(
        tools
            .iter()
            .any(|tool| *tool == task_callback_contract::definition()),
        "task callback tool missing from new thread"
    );
    ensure!(
        tools
            .iter()
            .any(|tool| *tool == framework_contract::definition()),
        "workflow tool missing from new thread"
    );
    Ok(())
}

pub async fn seed_unfinished_work(
    shared: Arc<Mutex<Store>>,
    files: Arc<Files>,
    task: &Value,
) -> Result<()> {
    let created = {
        let mut store = shared.lock().unwrap();
        let mut state = asset_task::enable(&store, task)?;
        asset_delivery::configure(
            &mut state,
            0,
            &serde_json::from_value(json!({
                "id":"fixture", "version":1, "stages":[
                    {"id":"design","name":"Design"},{"id":"model","name":"Model"}]
            }))?,
        )?;
        asset_task::save(&store, &state)?;
        let mut old = task.clone();
        old["threadId"] = json!("old-thread");
        old["turnId"] = json!("old-turn");
        store.put("task", "t", &old)?;
        task_callback::call(
            &mut store,
            "t",
            "old-thread",
            "old-turn",
            &json!({"operation":"work","requestId":"seed-create","expectedRevision":0,
            "assetRevision":1,"stageId":"design","change":{"action":"create",
                "definition":{"title":"Brief","goal":"Write a brief","acceptance":"Owner can review it"}}}),
        )?
    };
    task_callback_runtime::call(
        shared.clone(),
        files,
        "t".into(),
        "old-thread".into(),
        "old-turn".into(),
        json!({"operation":"work","requestId":"seed-begin","expectedRevision":1,
            "assetRevision":2,"stageId":"design","change":{"action":"begin",
                "subtaskId":created["subtaskId"],"inputs":{"request":"Brief"},
                "inputFiles":[{"path":"project.godot","role":"dependency"}]}}),
    )
    .await?;
    shared.lock().unwrap().put("task", "t", task)
}

pub fn report_request(prompt: &str, thread: &str) -> Result<Value> {
    ensure!(
        (thread == "existing-thread") == (prompt == "callback-resume"),
        "callback tool migration selected the wrong thread"
    );
    Ok(json!({"id":"callback-report","method":"item/tool/call",
        "params":{"threadId":thread,"turnId":"turn","tool":"beaver_task",
        "arguments":{"operation":"report","requestId":"draft","expectedRevision":2,
            "report":{"kind":"result","summary":"Fixture draft reported",
                "inputs":{"design":"request"},"outputs":{"file":"draft.blend"}}}}}))
}

pub fn after_reply(request: &Value, thread: &str) -> Result<Value> {
    ensure!(
        request["result"]["success"] == true,
        "callback failed: {request}"
    );
    let state: Value = serde_json::from_str(
        request["result"]["contentItems"][0]["text"]
            .as_str()
            .context("callback content missing")?,
    )?;
    if request["id"] == "callback-report" {
        ensure!(state["revision"] == 3);
        ensure!(state["recorded"] == true && state["acceptance"] == "notEvaluated");
        Ok(json!({"id":"callback-state","method":"item/tool/call",
            "params":{"threadId":thread,"turnId":"turn","tool":"beaver_task",
                "arguments":{"operation":"state"}}}))
    } else if request["id"] == "callback-state" {
        ensure!(state["revision"] == 3);
        ensure!(
            state["task"]["status"] == "running" && state["receipts"][0]["requestId"] == "draft"
        );
        let asset = &state["asset"];
        ensure!(
            asset["work"]["attempts"][0]["status"] == "interrupted",
            "new turn retained the previous running attempt"
        );
        Ok(json!({"id":"callback-work-begin","method":"item/tool/call",
            "params":{"threadId":thread,"turnId":"turn","tool":"beaver_task",
                "arguments":{"operation":"work","requestId":"retry-work",
                    "expectedRevision":3,"assetRevision":asset["revision"],"stageId":"design",
                    "change":{"action":"begin","subtaskId":asset["work"]["subtasks"][0]["id"],
                        "inputs":{"inspected":"retained workspace"},
                        "inputFiles":[{"path":"project.godot","role":"dependency"}],
                        "recoveryNote":"Inspected retained files; resume the brief"}}}}))
    } else {
        ensure!(state["revision"] == 4 && state["attemptId"].is_string());
        Ok(
            json!({"method":"turn/completed","params":{"threadId":thread,
            "turn":{"id":"turn","status":"completed"}}}),
        )
    }
}

pub fn verify_receipt(store: &Store, task: &Value, mode: &str, workspace: &Path) -> Result<()> {
    ensure!(task["callbackToolVersion"] == task_callback_contract::VERSION);
    if mode == "callback-upgrade" {
        ensure!(task["priorCallbackThreadId"] == "existing-thread");
    }
    let receipt = task_callback::inspect(store, "t", Some("draft"))?;
    ensure!(receipt["receipt"]["request"]["report"]["outputs"]["file"] == "draft.blend");
    ensure!(task["status"] == "running" && task["unknown"] == 42 && task["direction"] == "story");
    ensure!(
        !workspace.join("draft.blend").exists(),
        "report created a file"
    );
    let work = asset_task::get(store, "t")?.work;
    ensure!(work.attempts.len() == 2 && work.subtasks[0].status == "interrupted");
    ensure!(work
        .attempts
        .iter()
        .all(|attempt| attempt.status == "interrupted"
            && attempt.ended_at.is_some()
            && attempt.session_id.is_none()));
    ensure!(work.attempts[0].thread_id == "old-thread" && work.attempts[0].turn_id == "old-turn");
    ensure!(work.attempts[1].thread_id == task["threadId"] && work.attempts[1].turn_id == "turn");
    ensure!(work.attempts[0].id != work.attempts[1].id);
    for attempt in &work.attempts {
        let inputs = attempt
            .input_files
            .as_ref()
            .context("Missing captured inputs")?;
        ensure!(inputs.len() == 1 && inputs[0].path == "project.godot");
        ensure!(inputs[0].role == beaver_core::asset_work_inputs::Role::Dependency);
        beaver_core::asset_work_inputs::verify(
            &beaver_core::files::Files::new(workspace.parent().unwrap().join("data")),
            workspace,
            inputs,
        )?;
    }
    let receipt = task_callback::inspect(store, "t", Some("retry-work"))?;
    ensure!(receipt["receipt"]["response"]["attemptId"] == work.attempts[1].id);
    Ok(())
}
