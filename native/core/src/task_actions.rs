use crate::{
    files::{Change, Files},
    journal::{Journal, OperationKind},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Returns true when the live executor must receive a steer request instead.
pub fn continue_task(store: &mut Store, id: &str, text: &str, fresh_context: bool) -> Result<bool> {
    if text.encode_utf16().count() > 50000 {
        bail!("补充要求过长");
    }
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    if task["id"] != id {
        bail!("任务标识不匹配");
    }
    let project = task["projectId"].as_str().context("项目标识无效")?;
    if store.get::<Value>("project", project)?.is_none() {
        bail!("项目不存在");
    }
    let status = task["status"].as_str().context("任务状态无效")?;
    if fresh_context {
        anyhow::ensure!(
            ["interrupted", "failed"].contains(&status),
            "请先中止任务，再用新会话继续；已交付、冲突或待回答任务不能重开会话"
        );
        anyhow::ensure!(
            task["threadId"].as_str().is_some_and(|id| !id.is_empty()),
            "任务尚无可重新开始的 AI 会话"
        );
        anyhow::ensure!(
            !task["clarifications"].as_array().is_some_and(|items| items
                .iter()
                .any(|q| q.get("answers").is_none_or(Value::is_null))),
            "请先回答待补充的问题"
        );
    }
    if status == "waitingChildren" {
        anyhow::ensure!(text.is_empty(), "请打开具体子任务补充要求");
        task["planPaused"] = json!(false);
        store.put("task", id, &task)?;
        for mut child in store.list::<Value>("task")? {
            if child["parentTaskId"] == id
                && child["status"] == "interrupted"
                && child["workspacePrepared"].is_boolean()
            {
                child["status"] = json!("queued");
                store.put(
                    "task",
                    child["id"].as_str().context("任务标识无效")?,
                    &child,
                )?;
            }
        }
        return Ok(false);
    }
    if status == "running" {
        return Ok(true);
    }
    if status == "awaitingInput" {
        bail!("请先回答待补充的问题");
    }
    if !["queued", "interrupted", "failed"].contains(&status) {
        bail!("已交付或冲突任务请创建后续任务，保留独立回退边界");
    }
    let queued = status == "queued";
    let addition = if !queued && text.is_empty() {
        "继续未完成的目标，先检查已有进度，避免重复已完成的工作。"
    } else {
        text
    };
    task["prompt"] = json!(format!(
        "{}\n\n{}：{addition}",
        task["prompt"].as_str().context("任务目标无效")?,
        if queued {
            "补充要求"
        } else {
            "继续要求"
        }
    ));
    task["status"] = json!("queued");
    task["validationRepairAttempts"] = json!(0);
    task["validationRechecks"] = json!(0);
    if task["integrationValidation"] != true {
        task["validationOnly"] = json!(false);
        task["validationPrepared"] = json!(false);
    }
    task.as_object_mut()
        .context("任务格式无效")?
        .remove("turnId");
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    if fresh_context {
        let mut history = match task.get("sessionHistory") {
            Some(value) => value.as_array().context("会话历史无效")?.clone(),
            None => Vec::new(),
        };
        history.push(json!({"threadId":task["threadId"],"endedAt":now,"reason":"freshContext"}));
        task["sessionHistory"] = json!(history);
        let excerpt = |key: &str, limit: usize| -> String {
            task[key]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(limit)
                .collect()
        };
        let handoff = json!({
            "previousReport":excerpt("report", 2500),
            "previousError":excerpt("error", 1500),
            "changedPaths":task["changes"].as_array().map(|changes| changes.iter().filter_map(|c| c["path"].as_str()).take(80).collect::<Vec<_>>()).unwrap_or_default(),
        });
        task["restartContext"] = handoff;
        task.as_object_mut()
            .context("任务格式无效")?
            .remove("threadId");
        task.as_object_mut()
            .context("任务格式无效")?
            .remove("automaticRetries");
    }
    task["updatedAt"] = json!(now);
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), id],
        )?;
        let mut units = 0;
        let event: String = addition
            .chars()
            .take_while(|c| {
                units += c.len_utf16();
                units <= 32000
            })
            .collect();
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,'user',?)",
            rusqlite::params![id, now, event],
        )?;
        if fresh_context {
            db.execute("INSERT INTO events(task,time,kind,text) VALUES(?,?,'contextRestart',?)", rusqlite::params![id, now, "使用新 AI 会话继续；原会话历史、工作副本、任务基线及回退边界均保留。先核实已有文件，不重做已完成操作。"])?;
        }
        Ok(())
    })?;
    Ok(false)
}

fn completed(store: &Store, id: &str) -> Result<Value> {
    let task: Value = store.get("task", id)?.context("任务不存在")?;
    if task["id"] != id || task["status"] != "completed" {
        bail!("仅已合入的任务可执行此操作");
    }
    Ok(task)
}

/// Caller holds the project write lock and exclusive store access.
pub fn rollback(store: &mut Store, files: &Files, id: &str, keep: Vec<String>) -> Result<()> {
    let mut task = completed(store, id)?;
    let changes: Vec<Change> =
        serde_json::from_value(task["changes"].clone()).context("任务变更记录无效")?;
    let mut unique = HashSet::new();
    for path in &keep {
        if !changes.iter().any(|change| change.path == *path) || !unique.insert(path) {
            bail!("保留文件不属于任务变更或存在重复");
        }
    }
    let reverse = changes
        .into_iter()
        .filter(|change| !keep.contains(&change.path))
        .map(|change| Change {
            path: change.path,
            before: change.after,
            after: change.before,
        })
        .collect();
    let project = task["projectId"]
        .as_str()
        .context("项目标识无效")?
        .to_string();
    task["status"] = json!("rolledBack");
    task["accepted"] = json!(false);
    task["retainedFiles"] = json!(keep);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    task["updatedAt"] = json!(now);
    Journal::new(store, files).apply(&project, reverse, task, OperationKind::Rollback)?;
    store.event(
        id,
        &now,
        "rollback",
        &format!(
            "标准回退完成；保留：{}",
            if keep.is_empty() {
                "无".into()
            } else {
                keep.join(", ")
            }
        ),
    )?;
    Ok(())
}

pub fn accept(store: &mut Store, id: &str) -> Result<()> {
    accept_with_source(store, id, "user")
}
pub(crate) fn accept_with_source(store: &mut Store, id: &str, source: &str) -> Result<()> {
    let mut task = completed(store, id)?;
    if task["accepted"] == true {
        return Ok(());
    }
    let adoption = if task["feature"].is_object() {
        let feature = &task["feature"];
        let feature_id = feature["id"].as_str().context("功能块标识无效")?;
        let project_id = task["projectId"].as_str().context("项目标识无效")?;
        let key = format!("{project_id}:{feature_id}");
        let old = store.get::<Value>("feature", &key)?.unwrap_or(Value::Null);
        if old["taskId"] != feature["previous"]["taskId"] {
            bail!("该功能块已由另一任务更新，请重新整合后认可");
        }
        Some((
            key,
            json!({"id":feature_id,"version":feature["version"],"snapshot":feature["snapshot"],"taskId":id}),
        ))
    } else {
        None
    };
    task["accepted"] = json!(true);
    task["approvalSource"] = json!(source);
    task["updatedAt"] =
        json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    store.transaction(|db| {
        db.execute("UPDATE entities SET value=? WHERE kind='task' AND id=?", rusqlite::params![task.to_string(),id])?;
        db.execute("INSERT INTO events(task,time,kind,text) VALUES(?,?,'approval',?)", rusqlite::params![id,task["updatedAt"].as_str(),source])?;
        if let Some((key, value)) = adoption {
            db.execute("INSERT INTO entities(kind,id,value) VALUES('feature',?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value", rusqlite::params![key,value.to_string()])?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuation_preserves_context_and_delivery_boundaries() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("project", "p", &json!({"id":"p"}))?;
        for status in [
            "queued",
            "interrupted",
            "failed",
            "running",
            "awaitingInput",
            "completed",
            "rolledBack",
            "conflict",
        ] {
            let task = json!({"id":"t","projectId":"p","status":status,"prompt":"原目标","threadId":"old-thread","baseline":{"a":"hash"},"projectContext":{"name":"冻结名称"}});
            store.put("task", "t", &task)?;
            let result = continue_task(&mut store, "t", "补充角色设定", false);
            let after: Value = store.get("task", "t")?.unwrap();
            if ["queued", "interrupted", "failed"].contains(&status) {
                assert!(!result?);
                assert_eq!(after["status"], "queued");
                assert!(after["prompt"].as_str().unwrap().contains("补充角色设定"));
                assert_eq!(after["threadId"], task["threadId"]);
                assert_eq!(after["baseline"], task["baseline"]);
                assert_eq!(after["projectContext"], task["projectContext"]);
            } else if status == "running" {
                assert!(result?);
                assert_eq!(after, task);
            } else {
                assert!(result.is_err());
                assert_eq!(after, task);
            }
        }
        Ok(())
    }
    #[test]
    fn fresh_context_keeps_workspace_and_rollback_ownership_and_rejects_live_or_delivered_tasks(
    ) -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("project", "p", &json!({"id":"p"}))?;
        for status in [
            "interrupted",
            "failed",
            "queued",
            "running",
            "awaitingInput",
            "completed",
            "conflict",
            "waitingChildren",
            "rolledBack",
        ] {
            let task = json!({"id":"t","projectId":"p","status":status,"prompt":"original goal","threadId":"old-thread","turnId":"old-turn","workspace":"existing-copy","baseline":{"a":"original-hash"},"changes":[{"path":"a","before":"original-hash","after":"new-hash"}],"projectContext":{"revision":3},"report":"prior output","error":"upstream unavailable","automaticRetries":1,"unknown":42});
            store.put("task", "t", &task)?;
            let result = continue_task(&mut store, "t", "check current files", true);
            let after: Value = store.get("task", "t")?.unwrap();
            if ["interrupted", "failed"].contains(&status) {
                assert!(!result?);
                assert_eq!(after["status"], "queued");
                for key in [
                    "workspace",
                    "baseline",
                    "changes",
                    "projectContext",
                    "unknown",
                ] {
                    assert_eq!(after[key], task[key]);
                }
                assert!(after.get("threadId").is_none() && after.get("turnId").is_none());
                assert_eq!(after["sessionHistory"][0]["threadId"], "old-thread");
                assert_eq!(after["restartContext"]["changedPaths"], json!(["a"]));
                assert_eq!(after["restartContext"]["previousReport"], "prior output");
                assert!(
                    continue_task(&mut store, "t", "", true).is_err(),
                    "double restart must not queue twice"
                );
            } else {
                assert!(result.is_err());
                assert_eq!(after, task);
            }
        }
        Ok(())
    }
    #[test]
    fn retained_rollback_preserves_other_work_and_rejects_conflicts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        std::fs::create_dir(&root)?;
        std::fs::write(root.join("a"), "old")?;
        std::fs::write(root.join("b"), "old")?;
        let data = temp.path().join("data");
        let mut store = Store::open(&data)?;
        let files = Files::new(data);
        let before = files.capture(&root)?;
        std::fs::write(root.join("a"), "new")?;
        std::fs::write(root.join("b"), "new")?;
        let changes = Files::changes(&before, &files.capture(&root)?);
        store.put("project", "p", &json!({"id":"p","path":root}))?;
        store.put(
            "task",
            "t",
            &json!({"id":"t","projectId":"p","status":"completed","changes":changes}),
        )?;
        assert!(rollback(&mut store, &files, "t", vec!["unknown".into()]).is_err());
        std::fs::write(root.join("a"), "human")?;
        assert!(rollback(&mut store, &files, "t", vec![]).is_err());
        assert_eq!(std::fs::read_to_string(root.join("b"))?, "new");
        std::fs::write(root.join("unrelated"), "keep")?;
        rollback(&mut store, &files, "t", vec!["a".into()])?;
        assert_eq!(std::fs::read_to_string(root.join("a"))?, "human");
        assert_eq!(std::fs::read_to_string(root.join("b"))?, "old");
        assert_eq!(std::fs::read_to_string(root.join("unrelated"))?, "keep");
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["status"],
            "rolledBack"
        );
        Ok(())
    }
    #[test]
    fn accepting_feature_cannot_overwrite_newer_adoption() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("task", "t", &json!({"id":"t","projectId":"p","status":"completed","feature":{"id":"save","version":"2","snapshot":{},"previous":{"taskId":"old"}}}))?;
        store.put("feature", "p:save", &json!({"taskId":"newer"}))?;
        assert!(accept(&mut store, "t").is_err());
        assert_ne!(store.get::<Value>("task", "t")?.unwrap()["accepted"], true);
        store.put("feature", "p:save", &json!({"taskId":"old"}))?;
        accept(&mut store, "t")?;
        assert_eq!(
            store.get::<Value>("feature", "p:save")?.unwrap()["taskId"],
            "t"
        );
        accept(&mut store, "t")?;
        Ok(())
    }
}
