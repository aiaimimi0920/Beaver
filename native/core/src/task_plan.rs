use crate::{
    documents::project_path,
    files::{safe_path, Files},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, path::Path};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    title: String,
    prompt: String,
    direction: String,
    acceptance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow: Option<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    summary: String,
    steps: Vec<Step>,
}
pub fn validate(value: Value) -> Result<Plan> {
    let mut plan: Plan = serde_json::from_value(value).context("子任务计划格式无效")?;
    fn text(value: &mut String, max: usize) -> Result<()> {
        *value = value.trim().to_owned();
        anyhow::ensure!(
            !value.is_empty() && value.encode_utf16().count() <= max,
            "计划文字为空或过长"
        );
        Ok(())
    }
    text(&mut plan.summary, 2000)?;
    anyhow::ensure!((2..=12).contains(&plan.steps.len()), "请提交 2-12 个子任务");
    let mut titles = HashSet::new();
    let selected = plan
        .steps
        .iter()
        .filter(|step| step.workflow.is_some())
        .count();
    anyhow::ensure!(
        selected == 0 || selected == plan.steps.len(),
        "Choose a workflow for every step or retain the legacy plan format"
    );
    for step in &mut plan.steps {
        anyhow::ensure!(
            step.workflow
                .as_deref()
                .is_none_or(|id| ["general", "npr-character"].contains(&id)),
            "Unknown production workflow"
        );
        text(&mut step.title, 120)?;
        text(&mut step.prompt, 6000)?;
        text(&mut step.acceptance, 2000)?;
        anyhow::ensure!(titles.insert(step.title.clone()), "子任务标题不能重复");
        anyhow::ensure!(
            [
                "general",
                "story",
                "gameplay",
                "visual",
                "audio",
                "engineering",
                "review",
                "release"
            ]
            .contains(&step.direction.as_str()),
            "任务方向无效"
        );
    }
    Ok(plan)
}
pub fn tool() -> Value {
    serde_json::from_str(include_str!("../../../dist-native/plan-tool.json"))
        .expect("embedded plan tool")
}
pub fn submit(store: &Store, files: &Files, id: &str, params: &Value) -> Result<()> {
    let task: Value = store.get("task", id)?.context("任务不存在")?;
    crate::object_framework::require_legacy(&task)?;
    anyhow::ensure!(
        !crate::external_run_contract::enabled(&task),
        "External plans require the host run contract"
    );
    anyhow::ensure!(
        task["threadId"].is_string()
            && params["threadId"] == task["threadId"]
            && (!task["turnId"].is_string() || params["turnId"] == task["turnId"]),
        "过期的计划请求"
    );
    submit_validated(store, files, id, task, &params["arguments"])
}

pub(crate) fn submit_external(
    store: &Store,
    files: &Files,
    id: &str,
    arguments: &Value,
) -> Result<()> {
    let task: Value = store.get("task", id)?.context("Task missing")?;
    crate::object_framework::require_legacy(&task)?;
    anyhow::ensure!(
        crate::external_run_contract::enabled(&task),
        "External execution mode required"
    );
    submit_validated(store, files, id, task, arguments)
}

fn submit_validated(
    store: &Store,
    files: &Files,
    id: &str,
    mut task: Value,
    arguments: &Value,
) -> Result<()> {
    crate::object_framework::require_legacy(&task)?;
    anyhow::ensure!(
        task["status"] == "running" && task["decompose"] == true && task["parentTaskId"].is_null(),
        "当前任务不能提交计划"
    );
    anyhow::ensure!(
        !task["clarifications"]
            .as_array()
            .is_some_and(|qs| qs.iter().any(|q| q["answers"].is_null())),
        "请先回答未完成的问题"
    );
    anyhow::ensure!(!task["plan"].is_object(), "计划已经提交");
    let plan = validate(arguments.clone())?;
    let mut bindings = serde_json::Map::new();
    for step in &plan.steps {
        if let Some(id) = &step.workflow {
            let root = files.resolve_workspace(
                task["id"].as_str().context("Task ID missing")?,
                Path::new(
                    task["workspace"]
                        .as_str()
                        .context("Task workspace missing")?,
                ),
            )?;
            bindings.insert(id.clone(), crate::task_workflows::selection(&root, id)?);
        }
    }
    if !bindings.is_empty() {
        task["workflowBindings"] = Value::Object(bindings);
    }
    task["plan"] = serde_json::to_value(plan)?;
    task["report"] = task["plan"]["summary"].clone();
    store.put("task", id, &task)?;
    Ok(())
}

/// After a successful planning merge. Parent + all child identities commit together.
pub fn expand(store: &mut Store, files: &Files, parent: &mut Value) -> Result<()> {
    crate::object_framework::require_legacy(parent)?;
    if parent["status"] != "waitingChildren"
        || parent["subtaskIds"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    {
        return Ok(());
    }
    let plan = validate(parent["plan"].clone())?;
    let id = parent["id"].as_str().context("任务标识无效")?.to_owned();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let ids: Vec<String> = plan
        .steps
        .iter()
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect();
    let children = plan.steps.iter().enumerate().map(|(i, step)| -> Result<Value> {
        let mut child = json!({"id":ids[i],"projectId":parent["projectId"],"parentTaskId":id,"relation":"child","title":step.title,
          "prompt":format!("子目标：{}\n验收标准：{}\n阅读 .beaver-context/parent/task.json 中的原始目标和已确认回答，以及当前工作副本内的前置成果。仅完成本子目标并验证，不代替其他子任务宣布完成。",step.prompt,step.acceptance),
          "direction":step.direction,"capability":"code","decompose":false,"autoAccept":parent["autoAccept"].as_bool().unwrap_or(true),
          "workspace":files.workspace_location(&ids[i])?,"workspacePrepared":false,"baseline":{},"changes":[],"conflicts":[],
          "status":"queued","createdAt":now,"updatedAt":now,"dependsOn":if i == 0 { vec![] } else { vec![ids[i-1].clone()] }});
        for key in ["askRatio","references","stopConditions","maxMinutes","projectContext","design","validationVersion","executionMode"] { if let Some(v) = parent.get(key) { child[key] = v.clone(); } }
        if let Some(workflow) = &step.workflow {
            let binding = parent["workflowBindings"].get(workflow).context("Plan workflow binding missing")?;
            child["productionWorkflow"] = binding.clone();
        }
        Ok(child)
    }).collect::<Result<Vec<_>>>()?;
    parent["subtaskIds"] = json!(ids);
    store.transaction(|db| {
        for child in &children {
            db.execute(
                "INSERT INTO entities(kind,id,value) VALUES('task',?,?)",
                rusqlite::params![child["id"].as_str(), child.to_string()],
            )?;
        }
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![parent.to_string(), id],
        )?;
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,'plan',?)",
            rusqlite::params![
                id,
                now,
                format!(
                    "已创建 {} 个独立子任务，按依赖顺序执行并审批。",
                    children.len()
                )
            ],
        )?;
        Ok(())
    })
}

pub fn eligible(task: &Value, tasks: &[Value]) -> bool {
    if crate::external_run_recovery::blocked(task) {
        return false;
    }
    // Legacy history retained by a partition activation is never scheduled until converted.
    if crate::object_framework::marked(task) || task.get("migrationRetained").is_some() {
        return false;
    }
    if let Some(parent) = task["parentTaskId"]
        .as_str()
        .filter(|_| task["workspacePrepared"].is_boolean())
    {
        if !tasks.iter().any(|t| {
            t["id"] == parent
                && t["status"] == "waitingChildren"
                && t["planPaused"] != true
                && !crate::object_framework::marked(t)
        }) {
            return false;
        }
    }
    task["dependsOn"].as_array().is_none_or(|ids| {
        ids.iter().all(|id| {
            tasks.iter().any(|t| {
                t["id"] == *id
                    && t["status"] == "completed"
                    && t["accepted"] == true
                    && !crate::object_framework::marked(t)
            })
        })
    })
}

/// Freeze the latest integrated files only when dependencies have passed approval.
pub fn prepare(store: &mut Store, files: &Files, task: &mut Value) -> Result<()> {
    crate::object_framework::require_legacy(task)?;
    crate::external_run_recovery::require_clear(store, task)?;
    if task["workspacePrepared"] != false {
        return Ok(());
    }
    let parent: Value = store
        .get("task", task["parentTaskId"].as_str().context("父任务无效")?)?
        .context("父任务不存在")?;
    crate::object_framework::require_legacy(&parent)?;
    let id = task["id"].as_str().context("任务标识无效")?;
    anyhow::ensure!(
        !crate::journal::Journal::new(store, files)
            .blocked(task["projectId"].as_str().context("项目标识无效")?)?,
        "项目存在未完成的文件恢复"
    );
    let workspace = files.workspace(id)?;
    let baseline = files.capture(&project_path(
        store,
        task["projectId"].as_str().context("项目标识无效")?,
    )?)?;
    // Never overwrite a manually changed partial preparation after a failed start.
    if workspace.exists() {
        anyhow::ensure!(
            files.capture(&workspace)? == baseline,
            "未启动的子任务副本已有不同内容，请保留并检查后重试"
        );
    }
    files.restore_copy(&baseline, &workspace)?;
    let dir = safe_path(&workspace, ".beaver-context/parent")?;
    fs::create_dir_all(&dir)?;
    let record = json!({"title":parent["title"],"prompt":parent["prompt"],"plan":parent["plan"],"clarifications":parent["clarifications"]});
    fs::write(dir.join("task.json"), serde_json::to_vec_pretty(&record)?)?;
    let location = files.workspace_location(id)?;
    task["baseline"] = json!(baseline);
    task["workspace"] = json!(location);
    task["workspacePrepared"] = json!(true);
    Ok(())
}

pub fn reconcile(store: &mut Store, files: &Files) -> Result<()> {
    reconcile_matching(store, files, |_| true)
}

pub(crate) fn reconcile_matching(
    store: &mut Store,
    files: &Files,
    owns: impl Fn(&Value) -> bool,
) -> Result<()> {
    let tasks: Vec<Value> = store
        .list::<Value>("task")?
        .into_iter()
        .filter(owns)
        .collect();
    for mut parent in tasks
        .iter()
        .filter(|t| {
            !crate::object_framework::marked(t)
                && t["status"] == "waitingChildren"
                && t["planPaused"] != true
        })
        .cloned()
    {
        expand(store, files, &mut parent)?;
        let ids = parent["subtaskIds"]
            .as_array()
            .context("子任务标识无效")?
            .clone();
        if !ids.is_empty()
            && ids.iter().all(|id| {
                tasks.iter().any(|t| {
                    t["id"] == *id
                        && t["status"] == "completed"
                        && t["accepted"] == true
                        && !crate::object_framework::marked(t)
                })
            })
        {
            if parent["validationVersion"] == 1 {
                parent["status"] = json!("queued");
                parent["integrationValidation"] = json!(true);
                parent["validationOnly"] = json!(true);
            } else {
                parent["status"] = json!("completed");
            }
            parent["report"] = json!(format!("{}\n\n{} 个子任务已完成合入并通过审批。请检查各子任务的验证报告；这不替代实际试玩。",parent["plan"]["summary"].as_str().unwrap_or(""),ids.len()));
            let id = parent["id"].as_str().context("任务标识无效")?.to_owned();
            parent["updatedAt"] =
                json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            store.put("task", &id, &parent)?;
        }
    }
    Ok(())
}

pub fn approval(store: &Store, id: &str, automatic: bool) -> Result<Value> {
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    crate::object_framework::require_legacy(&task)?;
    if task["accepted"] == true {
        bail!("已审批任务不能更改历史审批来源");
    }
    task["autoAccept"] = json!(automatic);
    store.put("task", id, &task)?;
    for mut child in store.list::<Value>("task")? {
        if child["parentTaskId"] == id
            && child["workspacePrepared"].is_boolean()
            && child["accepted"] != true
            && !crate::object_framework::marked(&child)
        {
            child["autoAccept"] = json!(automatic);
            store.put(
                "task",
                child["id"].as_str().context("任务标识无效")?,
                &child,
            )?;
        }
    }
    Ok(task)
}
