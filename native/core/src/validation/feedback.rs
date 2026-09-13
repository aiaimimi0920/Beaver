use super::{
    operations::{owned_run, string},
    repository,
    requests::Request,
};
use crate::{store::Store, task_create};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

fn selection(run: &super::model::Run, input: &Value) -> Result<Value> {
    ensure!(
        input["snapshotId"] == run.snapshot_id,
        "Feedback must name the displayed snapshot"
    );
    let evidence = input["evidenceId"]
        .as_str()
        .map(|id| {
            run.evidence
                .iter()
                .find(|e| e.id == id)
                .context("Evidence not found in this run")
        })
        .transpose()?;
    if let Some(range) = input.get("range").filter(|v| !v.is_null()) {
        let media = evidence.context("Select video evidence before selecting a time range")?;
        let range: [f64; 2] = serde_json::from_value(range.clone())?;
        ensure!(
            media.kind == "video"
                && range.iter().all(|n| n.is_finite())
                && range[0] >= media.start
                && range[1] <= media.end
                && range[1] >= range[0],
            "Video selection is out of bounds"
        );
    }
    if let Some(region) = input.get("region").filter(|v| !v.is_null()) {
        ensure!(
            evidence.is_some(),
            "Select evidence before selecting a region"
        );
        let [x, y, w, h]: [f64; 4] = serde_json::from_value(region.clone())?;
        ensure!(
            [x, y, w, h].iter().all(|v| v.is_finite() && *v >= 0.0)
                && w > 0.0
                && h > 0.0
                && x + w <= 1.0
                && y + h <= 1.0,
            "Region must fit the image in normalized coordinates"
        );
    }
    Ok(
        json!({"evidenceId":input["evidenceId"],"range":input["range"],"region":input["region"],
        "references":evidence.map(|e| &e.references)}),
    )
}

pub fn create(
    store: &mut Store,
    data: &Path,
    input: &Value,
    designs: &Value,
    blueprints: &Value,
    source: &str,
) -> Result<Value> {
    let request = Request::new("validation.feedback.create", input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let run = owned_run(store, input)?;
    let text = string(input, "text")?.trim();
    ensure!(
        !text.is_empty() && text.encode_utf16().count() <= 10000,
        "Feedback text is empty or too long"
    );
    let selected = selection(&run, input)?;
    let mode = input["mode"].as_str().unwrap_or("followup");
    ensure!(
        ["new", "child", "followup", "review"].contains(&mode),
        "Unknown feedback task mode"
    );
    let parent_id = input["taskId"].as_str().or(run.task_id.as_deref());
    let parent: Option<Value> = parent_id
        .map(|id| repository::get(store, "task", id))
        .transpose()?;
    if let Some(parent) = &parent {
        ensure!(
            parent["projectId"] == run.project_id,
            "Feedback task belongs to another project"
        );
        if mode == "followup" {
            ensure!(
                ["completed", "rolledBack"].contains(&parent["status"].as_str().unwrap_or("")),
                "Use the active task's conversation, or create an independent child task"
            );
        }
    }
    ensure!(
        mode != "child" || parent.is_some(),
        "Child feedback requires a parent task"
    );
    let id = repository::id();
    let mut record = json!({"id":id,"projectId":run.project_id,"runId":run.id,"snapshotId":run.snapshot_id,
        "flowId":run.flow.as_ref().map(|f| &f.id),"selection":selected,"text":text,"source":source,
        "mode":mode,"status":if mode == "review" { "reviewQueued" } else { "taskQueued" },"createdAt":repository::now()});
    let instruction = if mode == "review" {
        "审查真实截图、视频关键帧、时间轴和相关历史源码，给出具体画面/玩法问题与定位。只读审查，不修改项目，不代替用户确认正确，不推进认可基准。"
    } else {
        "检查项目当前版本，并结合反馈的历史画面和代码定位修复。保留已交付任务历史，不以删测试、跳过动作或移除采集点掩盖问题。修复后 Beaver 会在当前版本重跑原流程。"
    };
    let prompt = format!("{instruction}\n读取 .beaver-context/validation/feedback.json 和 run.json；media 中是原始证据，source 中是该证据对应的历史源码。视频请结合每个动作的关键帧和反馈时间段查看。历史源码仅作对照，不覆盖当前项目。\n反馈原文：{text}");
    let task_input = json!({"projectId":run.project_id,"prompt":prompt,
        "title":format!("{} · {}", if mode == "review" { "画面审查" } else { "验收反馈" }, text.chars().take(45).collect::<String>()),
        "decompose":false,"direction":if run.kind == "code" { "engineering" } else { "visual" },
        "capability":if mode == "review" { "review" } else { "code" }});
    let task = task_create::create_recorded(
        store,
        data,
        task_input,
        designs,
        blueprints,
        |files, workspace, task| {
            task["origin"] = json!(if source == "ui" { "user" } else { "system" });
            if mode != "new" && mode != "review" {
                if let Some(parent) = &parent {
                    task["parentTaskId"] = parent["id"].clone();
                    task["relation"] = json!(if mode == "child" { "child" } else { "followup" });
                }
            }
            record["taskId"] = task["id"].clone();
            task["validationFeedbackId"] = json!(id);
            if run.kind == "code" && mode != "review" {
                task["validationRepair"] = json!({"runId":run.id,"snapshotId":run.snapshot_id,
                    "engineVersion":run.engine_version,"error":run.error,"code":run.code});
            }
            super::feedback_context::freeze_feedback(files, workspace, &run, &record)?;
            let result = json!({"feedbackId":id,"taskId":task["id"]});
            Ok(vec![
                ("validationFeedback", id.clone(), record.clone()),
                request.record(&result),
            ])
        },
    )?;
    Ok(json!({"feedbackId":id,"taskId":task["id"]}))
}
