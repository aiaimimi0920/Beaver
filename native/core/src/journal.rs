use crate::{
    files::{file_hash, safe_path, Change, Files},
    store::Store,
};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Merge,
    Rollback,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationState {
    Applying,
    Aborting,
    Complete,
    Aborted,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperation {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub kind: OperationKind,
    pub state: OperationState,
    pub changes: Vec<Change>,
    pub task_after: Value,
}

pub struct Journal<'a> {
    store: &'a mut Store,
    files: &'a Files,
}
impl<'a> Journal<'a> {
    pub fn new(store: &'a mut Store, files: &'a Files) -> Self {
        Self { store, files }
    }
    pub fn recover(&mut self) -> Result<()> {
        for mut operation in self
            .store
            .list::<FileOperation>("operation")?
            .into_iter()
            .rev()
        {
            if !matches!(
                operation.state,
                OperationState::Applying | OperationState::Aborting
            ) {
                continue;
            }
            if let Err(error) = self.recover_one(&mut operation) {
                if let Some(mut task) = self.store.get::<Value>("task", &operation.task_id)? {
                    if !task.is_object() {
                        bail!("恢复任务记录无效");
                    }
                    let message = format!("文件恢复尚未完成，项目禁止继续写入。恢复项目路径/权限后重启 Beaver：{error}");
                    task["status"] = json!("conflict");
                    task["error"] = json!(message);
                    self.store.put("task", &operation.task_id, &task)?;
                    self.store
                        .event(&operation.task_id, &now(), "recovery", &message)?;
                }
            }
        }
        Ok(())
    }
    pub fn blocked(&self, project: &str) -> Result<bool> {
        Ok(crate::object_run_recovery::candidate::publication::blocked(
            &self.store.connection,
            project,
        )? || self
            .store
            .list::<FileOperation>("operation")?
            .iter()
            .any(|operation| {
                operation.project_id == project
                    && matches!(
                        operation.state,
                        OperationState::Applying | OperationState::Aborting
                    )
            }))
    }
    fn project(&self, operation: &FileOperation) -> Result<PathBuf> {
        let project: Value = self
            .store
            .get("project", &operation.project_id)?
            .context("文件恢复日志指向不存在的项目")?;
        Ok(PathBuf::from(
            project["path"].as_str().context("项目路径无效")?,
        ))
    }
    fn commit(&mut self, operation: &mut FileOperation) -> Result<()> {
        if !operation.task_after.is_object() {
            bail!("任务结束记录无效");
        }
        self.store.transaction(|connection| {
            if let Some(current) = get(connection, "task", &operation.task_id)? {
                if let Some(direction) = current.get("direction") {
                    operation.task_after["direction"] = direction.clone();
                }
            }
            put(
                connection,
                "task",
                &operation.task_id,
                &operation.task_after,
            )?;
            if operation.kind == OperationKind::Rollback {
                if let Some(feature) = operation.task_after.get("feature") {
                    let id = feature["id"].as_str().context("功能块标识无效")?;
                    let key = format!("{}:{id}", operation.project_id);
                    if get(connection, "feature", &key)?.and_then(|v| v.get("taskId").cloned())
                        == Some(json!(operation.task_id))
                    {
                        let retained = operation
                            .task_after
                            .get("retainedFiles")
                            .and_then(Value::as_array)
                            .is_some_and(|v| !v.is_empty());
                        if let Some(previous) = feature
                            .get("previous")
                            .filter(|v| !v.is_null() && !retained)
                        {
                            put(connection, "feature", &key, previous)?;
                        } else {
                            connection.execute(
                                "DELETE FROM entities WHERE kind='feature' AND id=?",
                                [&key],
                            )?;
                        }
                    }
                }
            }
            let mut completed = operation.clone();
            completed.state = OperationState::Complete;
            put(
                connection,
                "operation",
                &operation.id,
                &serde_json::to_value(&completed)?,
            )?;
            Ok(())
        })?;
        operation.state = OperationState::Complete;
        Ok(())
    }
    fn abort(&mut self, operation: &mut FileOperation) -> Result<()> {
        operation.state = OperationState::Aborting;
        self.store.put("operation", &operation.id, operation)?;
        let project = self.project(operation)?;
        let mut reverse = Vec::new();
        let mut conflicts = Vec::new();
        for change in &operation.changes {
            let current = file_hash(&safe_path(&project, &change.path)?)?;
            if current == change.after {
                reverse.push(Change {
                    path: change.path.clone(),
                    before: change.after.clone(),
                    after: change.before.clone(),
                });
            } else if current != change.before {
                conflicts.push(change.path.clone());
            }
        }
        reverse.reverse();
        self.files.apply(&project, &reverse)?;
        let mut task = self
            .store
            .get::<Value>("task", &operation.task_id)?
            .context("恢复任务不存在")?;
        if !task.is_object() {
            bail!("恢复任务记录无效");
        }
        task["status"] = json!(if operation.kind == OperationKind::Rollback {
            "completed"
        } else {
            "conflict"
        });
        task["conflicts"] = json!(conflicts);
        let message = "上次文件操作未完成，已撤销可确认的写入；外部修改保持不变。请检查后重试或发起对话整合。";
        task["error"] = json!(message);
        let mut aborted = operation.clone();
        aborted.state = OperationState::Aborted;
        self.store.transaction(|connection| {
            put(connection, "task", &operation.task_id, &task)?;
            put(
                connection,
                "operation",
                &operation.id,
                &serde_json::to_value(&aborted)?,
            )?;
            connection.execute(
                "INSERT INTO events(task,time,kind,text) VALUES(?,?,'recovery',?)",
                params![operation.task_id, now(), message],
            )?;
            Ok(())
        })?;
        operation.state = OperationState::Aborted;
        Ok(())
    }
    pub fn recover_one(&mut self, operation: &mut FileOperation) -> Result<()> {
        if operation.state == OperationState::Aborting {
            return self.abort(operation);
        }
        if operation.state != OperationState::Applying {
            return Ok(());
        }
        let project = self.project(operation)?;
        let mut remaining = Vec::new();
        for change in &operation.changes {
            let current = file_hash(&safe_path(&project, &change.path)?)?;
            if current == change.after {
                continue;
            }
            if current != change.before {
                return self.abort(operation);
            }
            remaining.push(change.clone());
        }
        self.files.apply(&project, &remaining)?;
        self.commit(operation)?;
        self.store.event(
            &operation.task_id,
            &now(),
            "recovery",
            "已恢复上次中断的文件操作。",
        )?;
        Ok(())
    }
    pub fn apply(
        &mut self,
        project_id: &str,
        changes: Vec<Change>,
        task_after: Value,
        kind: OperationKind,
    ) -> Result<()> {
        if self.blocked(project_id)? {
            bail!("项目有尚未恢复的文件操作，禁止继续写入");
        }
        let task_id = task_after["id"]
            .as_str()
            .context("任务标识无效")?
            .to_string();
        let mut operation = FileOperation {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            task_id,
            kind,
            state: OperationState::Applying,
            changes,
            task_after,
        };
        let project = self.project(&operation)?;
        let conflicts = self.files.conflicts(&project, &operation.changes)?;
        if !conflicts.is_empty() {
            bail!("文件冲突，未覆盖：{}", conflicts.join(", "));
        }
        self.store.put("operation", &operation.id, &operation)?;
        let result = self
            .files
            .apply(&project, &operation.changes)
            .and_then(|_| self.commit(&mut operation));
        if let Err(error) = result {
            self.abort(&mut operation)?;
            return Err(error);
        }
        Ok(())
    }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn get(connection: &Connection, kind: &str, id: &str) -> Result<Option<Value>> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![kind, id],
            |r| r.get(0),
        )
        .optional()?;
    value.map(|v| Ok(serde_json::from_str(&v)?)).transpose()
}
fn put(connection: &Connection, kind: &str, id: &str, value: &Value) -> Result<()> {
    connection.execute("INSERT INTO entities(kind,id,value) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value", params![kind,id,serde_json::to_string(value)?])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Result<(tempfile::TempDir, Store, Files, PathBuf, FileOperation)> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        std::fs::create_dir(&project)?;
        std::fs::write(project.join("a"), "before")?;
        std::fs::write(project.join("b"), "before")?;
        let files = Files::new(temp.path().join("data"));
        let before = files.capture(&project)?;
        std::fs::write(project.join("a"), "after")?;
        std::fs::write(project.join("b"), "after")?;
        let after = files.capture(&project)?;
        files.restore_copy(&before, &project)?;
        let store = Store::open(&temp.path().join("data"))?;
        store.put("project", "p", &json!({"id":"p","path":project}))?;
        store.put(
            "task",
            "t",
            &json!({"id":"t","status":"running","direction":"art"}),
        )?;
        let op = FileOperation {
            id: "op".into(),
            project_id: "p".into(),
            task_id: "t".into(),
            kind: OperationKind::Merge,
            state: OperationState::Applying,
            changes: Files::changes(&before, &after),
            task_after: json!({"id":"t","status":"completed","direction":"code"}),
        };
        store.put("operation", "op", &op)?;
        Ok((temp, store, files, project, op))
    }
    #[test]
    fn resumes_partial_operation_and_preserves_latest_direction() -> Result<()> {
        let (_temp, mut store, files, project, op) = fixture()?;
        files.apply(&project, &op.changes[..1])?;
        Journal::new(&mut store, &files).recover()?;
        assert_eq!(std::fs::read_to_string(project.join("b"))?, "after");
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["direction"],
            "art"
        );
        assert_eq!(
            store
                .get::<FileOperation>("operation", "op")?
                .unwrap()
                .state,
            OperationState::Complete
        );
        Journal::new(&mut store, &files).recover()?;
        Ok(())
    }
    #[test]
    fn external_edit_aborts_only_confirmed_writes() -> Result<()> {
        let (_temp, mut store, files, project, op) = fixture()?;
        files.apply(&project, &op.changes[..1])?;
        std::fs::write(project.join("b"), "human")?;
        Journal::new(&mut store, &files).recover()?;
        assert_eq!(std::fs::read_to_string(project.join("a"))?, "before");
        assert_eq!(std::fs::read_to_string(project.join("b"))?, "human");
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["status"],
            "conflict"
        );
        assert!(!Journal::new(&mut store, &files).blocked("p")?);
        Ok(())
    }
    #[test]
    fn rollback_updates_adoption_only_for_its_own_task() -> Result<()> {
        for (retained, adopted_by_other) in [(false, false), (true, false), (false, true)] {
            let (_temp, mut store, files, _project, operation) = fixture()?;
            Journal::new(&mut store, &files).recover()?;
            let adopted = json!({"taskId":if adopted_by_other {"other"} else {"t"},"version":"2"});
            store.put("feature", "p:save", &adopted)?;
            let reverse = operation
                .changes
                .iter()
                .map(|change| Change {
                    path: change.path.clone(),
                    before: change.after.clone(),
                    after: change.before.clone(),
                })
                .collect();
            Journal::new(&mut store, &files).apply("p", reverse, json!({
                "id":"t", "status":"rolledBack", "retainedFiles":if retained {vec!["a"]} else {vec![]},
                "feature":{"id":"save","previous":{"version":"1","taskId":"previous"}}
            }), OperationKind::Rollback)?;
            let result = store.get::<Value>("feature", "p:save")?;
            if adopted_by_other {
                assert_eq!(result, Some(adopted));
            } else if retained {
                assert!(result.is_none());
            } else {
                assert_eq!(result.unwrap()["version"], "1");
            }
        }
        Ok(())
    }
    #[test]
    fn unavailable_project_keeps_durable_blocker() -> Result<()> {
        let (_temp, mut store, files, project, _op) = fixture()?;
        std::fs::rename(&project, project.with_extension("moved"))?;
        Journal::new(&mut store, &files).recover()?;
        assert!(Journal::new(&mut store, &files).blocked("p")?);
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["status"],
            "conflict"
        );
        std::fs::rename(project.with_extension("moved"), &project)?;
        Journal::new(&mut store, &files).recover()?;
        assert!(!Journal::new(&mut store, &files).blocked("p")?);
        Ok(())
    }
}
