use crate::{
    documents::project_path,
    executor::Outcome,
    files::{file_hash, safe_path, Change, Files, Snapshot},
    journal::{Journal, OperationKind},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

fn awaiting(task: &Value) -> bool {
    task["clarifications"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|q| q.get("answers").is_none_or(Value::is_null))
    })
}

/// The executor must already be stopped. Caller owns project merge and database locks.
pub fn finish(
    store: &mut Store,
    files: &Files,
    id: &str,
    outcome: Outcome,
    closing: &AtomicBool,
) -> Result<Value> {
    finish_task(store, files, id, outcome, closing, false)
}

/// Retry recorded output without starting an executor or recapturing the workspace.
/// Caller owns the same project merge and database locks as `finish`.
pub fn retry_merge(
    store: &mut Store,
    files: &Files,
    id: &str,
    closing: &AtomicBool,
) -> Result<Value> {
    finish_task(store, files, id, Outcome::Completed, closing, true)
}

fn finish_task(
    store: &mut Store,
    files: &Files,
    id: &str,
    outcome: Outcome,
    closing: &AtomicBool,
    retry: bool,
) -> Result<Value> {
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    if task["id"] != id {
        bail!("任务标识不匹配");
    }
    if retry {
        anyhow::ensure!(
            task["status"] == "conflict"
                && task["capability"] != "review"
                && task["conflicts"]
                    .as_array()
                    .is_some_and(|paths| !paths.is_empty())
                && task.get("error").is_none_or(Value::is_null)
                && !awaiting(&task),
            "仅文件合入冲突可重试；其他任务请使用原有恢复操作"
        );
        anyhow::ensure!(!closing.load(Ordering::SeqCst), "应用正在退出");
        let project = task["projectId"].as_str().context("项目标识无效")?;
        anyhow::ensure!(
            !Journal::new(store, files).blocked(project)?,
            "项目文件恢复尚未完成，不能重试合入"
        );
    } else if !["running", "awaitingInput"].contains(&task["status"].as_str().unwrap_or("")) {
        bail!("任务已结束或尚未执行，不能重复提交结果");
    }
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    task["updatedAt"] = json!(now);
    task.as_object_mut()
        .context("任务格式无效")?
        .remove("error");
    if let Outcome::Failed(error) = &outcome {
        task["error"] = json!(error);
    }
    let result = (|| -> Result<()> {
        let changes: Vec<Change> = if retry {
            store.event(
                id,
                &now,
                "mergeRetry",
                "重试已记录的任务合入，未重新运行 AI",
            )?;
            serde_json::from_value(task["changes"].clone()).context("任务变更记录无效")?
        } else {
            let baseline: Snapshot =
                serde_json::from_value(task["baseline"].clone()).context("任务基线无效")?;
            let workspace = task["workspace"].as_str().context("任务工作副本无效")?;
            if outcome == Outcome::Completed && task["capability"] != "review" {
                let normalized =
                    crate::source_encoding::normalize(Path::new(workspace), &baseline)?;
                if !normalized.is_empty() {
                    store.event(
                        id,
                        &now,
                        "encoding",
                        &format!(
                            "已将修改的源文件规范化为 UTF-8 无 BOM：{}",
                            normalized.join(", ")
                        ),
                    )?;
                }
            }
            let after = files.capture(Path::new(workspace))?;
            Files::changes(&baseline, &after)
        };
        task["changes"] = json!(changes);
        if awaiting(&task) {
            task["status"] = json!("awaitingInput");
        } else if outcome == Outcome::Completed && !closing.load(Ordering::SeqCst) {
            if task["decompose"] == true {
                crate::task_plan::validate(task["plan"].clone())
                    .context("规划任务未提交可执行子任务，请继续并调用 beaver_submit_plan")?;
            }
            if task["capability"] == "review" && !changes.is_empty() {
                task["status"] = json!("conflict");
                task["error"] = json!("审查任务修改了文件，未合入原项目。");
            } else {
                if task["capability"] != "review" {
                    let baseline: Snapshot =
                        serde_json::from_value(task["baseline"].clone()).context("任务基线无效")?;
                    let report = crate::code_structure::check_changes(files, &baseline, &changes)?;
                    store.event(
                        id,
                        &now,
                        "codeStructure",
                        &format!(
                            "Code structure: {} checked sources, {} violations",
                            report.files.len(),
                            report.violations.len()
                        ),
                    )?;
                    task["codeStructure"] = serde_json::to_value(&report)?;
                    report.ensure_ok()?;
                }
                let project_id = task["projectId"]
                    .as_str()
                    .context("项目标识无效")?
                    .to_string();
                let project = project_path(store, &project_id)?;
                let mut pending = Vec::new();
                let mut already_applied = Vec::new();
                for change in changes {
                    if file_hash(&safe_path(&project, &change.path)?)? == change.after {
                        already_applied.push(change.path);
                    } else {
                        pending.push(change);
                    }
                }
                // Matching external edits must not become owned by this task's rollback.
                let changes = pending;
                task["changes"] = json!(changes);
                if !already_applied.is_empty() {
                    store.event(
                        id,
                        &now,
                        "merge",
                        &format!("已保留项目中内容相同的文件：{}", already_applied.join(", ")),
                    )?;
                }
                let conflicts = files.conflicts(&project, &changes)?;
                task["conflicts"] = json!(conflicts);
                if !conflicts.is_empty() {
                    task["status"] = json!("conflict");
                } else if closing.load(Ordering::SeqCst) {
                    task["status"] = json!("interrupted");
                } else {
                    task["status"] = json!(if task["decompose"] == true {
                        "waitingChildren"
                    } else {
                        "completed"
                    });
                    Journal::new(store, files).apply(
                        &project_id,
                        changes,
                        task.clone(),
                        OperationKind::Merge,
                    )?;
                }
            }
        } else {
            task["status"] = json!(if closing.load(Ordering::SeqCst)
                || outcome == Outcome::Interrupted
            {
                "interrupted"
            } else {
                "failed"
            });
        }
        Ok(())
    })();
    if let Err(error) = result {
        task["status"] = json!(if awaiting(&task) {
            "awaitingInput"
        } else {
            "failed"
        });
        if let Some(project) = task["projectId"].as_str() {
            if Journal::new(store, files).blocked(project)? {
                task["status"] = json!("conflict");
            }
        }
        task["error"] = json!(error.to_string());
    }
    let message = format!(
        "{}{}",
        task["status"].as_str().unwrap_or("failed"),
        task["error"]
            .as_str()
            .map(|e| format!(": {e}"))
            .unwrap_or_default()
    );
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), id],
        )?;
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,'status',?)",
            rusqlite::params![id, now, message],
        )?;
        Ok(())
    })?;
    if task["status"] == "completed" && task["autoAccept"] == true && task["relation"] == "child" {
        crate::task_actions::accept_with_source(store, id, "automatic")?;
    }
    crate::task_plan::reconcile(store, files)?;
    task = store.get("task", id)?.context("任务不存在")?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    fn setup() -> Result<(tempfile::TempDir, Store, Files, PathBuf, PathBuf)> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let store = Store::open(&data)?;
        let files = Files::new(data);
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("a.txt"), "before")?;
        let baseline = files.capture(&project)?;
        let workspace = temp.path().join("workspace");
        files.restore_copy(&baseline, &workspace)?;
        fs::write(workspace.join("a.txt"), "after")?;
        store.put("project", "p", &json!({"id":"p","path":project}))?;
        store.put("task", "t", &json!({"id":"t","projectId":"p","workspace":workspace,"baseline":baseline,"status":"running","capability":"code","direction":"story","future":42}))?;
        Ok((temp, store, files, project, workspace))
    }
    #[test]
    fn completed_output_merges_durably_and_cannot_be_replayed() -> Result<()> {
        let (_temp, mut store, files, project, _) = setup()?;
        let task = finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        assert_eq!(task["status"], "completed");
        assert_eq!(task["future"], 42);
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "after");
        assert_eq!(store.list::<Value>("operation")?[0]["state"], "complete");
        assert!(finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false)
        )
        .is_err());
        Ok(())
    }
    #[test]
    fn matching_external_changes_are_not_owned_by_task_rollback() -> Result<()> {
        let (_temp, mut store, files, project, workspace) = setup()?;
        fs::write(project.join("deleted.txt"), "old")?;
        let mut task: Value = store.get("task", "t")?.unwrap();
        task["baseline"] = json!(files.capture(&project)?);
        store.put("task", "t", &task)?;
        fs::write(project.join("a.txt"), "after")?;
        fs::write(project.join("shared.txt"), "shared")?;
        fs::write(workspace.join("shared.txt"), "shared")?;
        fs::remove_file(project.join("deleted.txt"))?;
        fs::write(workspace.join("owned.txt"), "owned")?;

        let task = finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        assert_eq!(task["status"], "completed");
        assert_eq!(task["changes"].as_array().unwrap().len(), 1);
        assert_eq!(task["changes"][0]["path"], "owned.txt");
        assert_eq!(fs::read_to_string(project.join("owned.txt"))?, "owned");

        crate::task_actions::rollback(&mut store, &files, "t", vec![])?;
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "after");
        assert_eq!(fs::read_to_string(project.join("shared.txt"))?, "shared");
        assert!(!project.join("deleted.txt").exists());
        assert!(!project.join("owned.txt").exists());
        Ok(())
    }
    #[test]
    fn human_conflict_blocks_entire_batch() -> Result<()> {
        let (_temp, mut store, files, project, workspace) = setup()?;
        fs::write(workspace.join("new.txt"), "new")?;
        fs::write(project.join("a.txt"), "human")?;
        let task = finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        assert_eq!(task["status"], "conflict");
        assert_eq!(task["conflicts"], json!(["a.txt"]));
        assert!(!project.join("new.txt").exists());
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "human");
        Ok(())
    }
    #[test]
    fn retry_uses_recorded_output_and_preserves_real_conflicts() -> Result<()> {
        let (_temp, mut store, files, project, workspace) = setup()?;
        let mut original: Value = store.get("task", "t")?.unwrap();
        original["report"] = json!("validated output");
        original["threadId"] = json!("retained-thread");
        store.put("task", "t", &original)?;
        fs::write(workspace.join("owned.txt"), "validated")?;
        fs::write(project.join("a.txt"), "human")?;
        let task = finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        assert_eq!(task["status"], "conflict");
        fs::write(workspace.join("owned.txt"), "unvalidated edit")?;
        fs::write(workspace.join("unrecorded.txt"), "unrecorded")?;

        let task = retry_merge(&mut store, &files, "t", &AtomicBool::new(false))?;
        assert_eq!(task["status"], "conflict");
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "human");
        assert!(!project.join("owned.txt").exists());

        fs::write(project.join("a.txt"), "after")?;
        let task = retry_merge(&mut store, &files, "t", &AtomicBool::new(false))?;
        assert_eq!(task["status"], "completed");
        assert_eq!(task["conflicts"], json!([]));
        assert_eq!(task["report"], original["report"]);
        assert_eq!(task["threadId"], original["threadId"]);
        assert_eq!(task["baseline"], original["baseline"]);
        assert_eq!(task["changes"].as_array().unwrap().len(), 1);
        assert_eq!(fs::read_to_string(project.join("owned.txt"))?, "validated");
        assert!(!project.join("unrecorded.txt").exists());
        assert!(retry_merge(&mut store, &files, "t", &AtomicBool::new(false)).is_err());

        crate::task_actions::rollback(&mut store, &files, "t", vec![])?;
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "after");
        assert!(!project.join("owned.txt").exists());
        Ok(())
    }
    #[test]
    fn retry_rejects_unfinished_review_and_shutdown_states() -> Result<()> {
        let (_temp, mut store, files, project, _) = setup()?;
        assert!(retry_merge(&mut store, &files, "t", &AtomicBool::new(false)).is_err());
        let mut task: Value = store.get("task", "t")?.unwrap();
        task["capability"] = json!("review");
        store.put("task", "t", &task)?;
        finish(
            &mut store,
            &files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        assert!(retry_merge(&mut store, &files, "t", &AtomicBool::new(false)).is_err());
        assert_eq!(fs::read_to_string(project.join("a.txt"))?, "before");

        let mut task: Value = store.get("task", "t")?.unwrap();
        task["capability"] = json!("code");
        task["conflicts"] = json!(["a.txt"]);
        task.as_object_mut().unwrap().remove("error");
        store.put("task", "t", &task)?;
        assert!(retry_merge(&mut store, &files, "t", &AtomicBool::new(true)).is_err());
        assert_eq!(store.get::<Value>("task", "t")?.unwrap(), task);
        Ok(())
    }
    #[test]
    fn review_questions_shutdown_and_failures_do_not_merge() -> Result<()> {
        for mode in ["review", "question", "shutdown", "failed"] {
            let (_temp, mut store, files, project, _) = setup()?;
            let mut task: Value = store.get("task", "t")?.unwrap();
            if mode == "review" {
                task["capability"] = json!("review");
            }
            if mode == "question" {
                task["status"] = json!("awaitingInput");
                task["clarifications"] = json!([{"id":"q"}]);
            }
            store.put("task", "t", &task)?;
            let outcome = if mode == "failed" {
                Outcome::Failed("fixture failure".into())
            } else {
                Outcome::Completed
            };
            let task = finish(
                &mut store,
                &files,
                "t",
                outcome,
                &AtomicBool::new(mode == "shutdown"),
            )?;
            let expected = match mode {
                "review" => "conflict",
                "question" => "awaitingInput",
                "shutdown" => "interrupted",
                _ => "failed",
            };
            assert_eq!(task["status"], expected);
            assert_eq!(fs::read_to_string(project.join("a.txt"))?, "before");
            assert_eq!(task["changes"].as_array().unwrap().len(), 1);
        }
        Ok(())
    }
}
