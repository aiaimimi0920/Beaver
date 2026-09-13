use super::{model::Run, repository, task_gate};
use crate::{files::Files, store::Store};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub fn freeze_repair(store: &Store, data: &Path, workspace: &Path, task: &Value) -> Result<()> {
    if let Some(id) = task["validationRepair"]["runId"].as_str() {
        let run: Run = repository::get(store, "validationRun", id)?;
        anyhow::ensure!(
            task["projectId"] == run.project_id,
            "Repair belongs to another project"
        );
        super::feedback_context::freeze_repair(
            &Files::new(data.into()),
            workspace,
            &run,
            &task["validationRepair"],
        )?;
    }
    Ok(())
}

fn child(
    store: &mut Store,
    files: &Files,
    task: &mut Value,
    title: &str,
    prompt: &str,
    origin: &str,
    repair: Option<Value>,
) -> Result<()> {
    let id = task["id"].as_str().context("Task ID missing")?.to_owned();
    let child_id = repository::id();
    let now = repository::now();
    let mut child = json!({
        "id":child_id,"projectId":task["projectId"],"parentTaskId":id,"relation":"child",
        "title":title,"prompt":prompt,"origin":origin,
        "capability":"code","direction":"engineering","decompose":false,"validationVersion":1,
        "autoAccept":task["autoAccept"],"workspace":files.root().join("workspaces").join(&child_id),
        "workspacePrepared":false,"baseline":{},"changes":[],"conflicts":[],"dependsOn":[],
        "status":"queued","createdAt":now,"updatedAt":now
    });
    if let Some(repair) = repair {
        child["validationRepair"] = repair;
        child["validationRepairAttempts"] = task["validationRepairAttempts"].clone();
    }
    for key in [
        "askRatio",
        "maxMinutes",
        "projectContext",
        "design",
        "stopConditions",
    ] {
        if let Some(value) = task.get(key) {
            child[key] = value.clone();
        }
    }
    task["subtaskIds"]
        .as_array_mut()
        .context("Missing child IDs")?
        .push(json!(child_id));
    task["status"] = json!("waitingChildren");
    store.transaction(|db| {
        db.execute(
            "INSERT INTO entities(kind,id,value) VALUES('task',?,?)",
            rusqlite::params![child_id, child.to_string()],
        )?;
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)",
            rusqlite::params![child_id, now, origin, prompt],
        )?;
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), id],
        )?;
        Ok(())
    })
}

fn resume(task: &mut Value) -> Result<()> {
    task["status"] = json!("queued");
    task["validationOnly"] = json!(false);
    task["validationPrepared"] = json!(false);
    task.as_object_mut()
        .context("Invalid task")?
        .remove("turnId");
    Ok(())
}

pub fn steered(store: &mut Store, files: &Files, task: &mut Value, text: &str) -> Result<()> {
    if task["integrationValidation"] == true {
        child(
            store,
            files,
            task,
            "落实集成验收期间的补充要求",
            text,
            "user",
            None,
        )?;
    } else {
        resume(task)?;
    }
    store.event(
        task["id"].as_str().context("Task ID missing")?,
        &repository::now(),
        "validationSteering",
        "已保存补充要求；应用修改后重新运行代码验收，当前候选尚未合入",
    )?;
    Ok(())
}

/// Automatic retries are system decisions, never synthetic user messages.
pub fn repair(store: &mut Store, files: &Files, task: &mut Value) -> Result<()> {
    if task["status"] != "failed" || !task["validationRepair"].is_object() {
        return Ok(());
    }
    let attempts = task["validationRepairAttempts"].as_u64().unwrap_or(0);
    let repair = task["validationRepair"].clone();
    let fingerprint = repository::digest(&json!([
        repair["snapshotId"],
        repair["code"],
        repair["error"]
    ]))?;
    if attempts >= 2
        || task["validationRepairFingerprint"] == fingerprint
        || repair["engineVersion"].as_str().is_none_or(str::is_empty)
    {
        return Ok(());
    }
    task["validationRepairAttempts"] = json!(attempts + 1);
    task["validationRepairFingerprint"] = json!(fingerprint);
    let id = task["id"].as_str().context("Task ID missing")?.to_owned();
    if task["integrationValidation"] == true {
        child(
            store,
            files,
            task,
            "修复集成代码验收",
            "修复父任务集成结果中的 GUT 失败，保持已确认功能与必需测试范围。",
            "system",
            Some(repair),
        )?;
    } else {
        resume(task)?;
    }
    task.as_object_mut()
        .context("Invalid task")?
        .remove("turnId");
    store.event(
        &id,
        &repository::now(),
        "validationRepair",
        &format!("GUT 自动修复 {}/2，保留失败运行记录", attempts + 1),
    )?;
    Ok(())
}

pub fn integrated(store: &Store, task: &mut Value) -> Result<()> {
    let children = task["subtaskIds"].as_array().context("Missing child IDs")?;
    let count = children.len();
    for id in children {
        let child: Value =
            repository::get(store, "task", id.as_str().context("Invalid child ID")?)?;
        anyhow::ensure!(
            child["status"] == "completed" && child["accepted"] == true,
            "A child changed before integration validation completed"
        );
    }
    task["status"] = json!("completed");
    task["report"] = json!(format!(
        "{}\n\n{} 个子任务已合入，集成候选 GUT 通过。画面在测试与画面页独立生成。",
        task["plan"]["summary"].as_str().unwrap_or(""),
        count
    ));
    Ok(())
}

/// Queue only durable metadata here. Rendering never runs inside a task merge.
pub fn delivered(store: &Store, task: &Value) -> Result<()> {
    if task["validationVersion"] != 1 || task["capability"] == "review" {
        return Ok(());
    }
    let id = task["id"].as_str().context("Task ID missing")?;
    let changes: Vec<crate::files::Change> = serde_json::from_value(task["changes"].clone())?;
    if task["codeValidation"]["status"] != "completed" && !task_gate::required(task, &changes) {
        return Ok(());
    }
    if store.get::<Value>("validationCoverage", id)?.is_none() {
        store.put("validationCoverage", id, &json!({
            "id":id,"taskId":id,"projectId":task["projectId"],"title":task["title"],
            "snapshotId":task["codeValidation"]["snapshotId"],"codeRunId":task["codeValidation"]["runId"],
            "status":"pending","flowIds":[],"runIds":[],"createdAt":repository::now()
        }))?;
    }
    Ok(())
}
