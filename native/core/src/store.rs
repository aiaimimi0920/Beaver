use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::Path, sync::Arc, time::Duration};

/// The schema remains readable by the existing TypeScript Store during migration.
pub struct Store {
    pub(crate) connection: Connection,
    // Close SQLite before releasing project ownership, including detached worker handles.
    _project_lock: Option<Arc<fs::File>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskEvent {
    pub time: String,
    pub kind: String,
    pub text: String,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        fs::create_dir_all(root).context("create Beaver data directory")?;
        let connection = Connection::open(root.join("beaver.sqlite"))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA journal_mode=WAL;")?;
        crate::store_schema::initialize(&connection)?;
        Ok(Self {
            connection,
            _project_lock: None,
        })
    }

    pub(crate) fn project(connection: Connection, lock: Arc<fs::File>) -> Self {
        Self {
            connection,
            _project_lock: Some(lock),
        }
    }

    pub fn get<T: DeserializeOwned>(&self, kind: &str, id: &str) -> Result<Option<T>> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM entities WHERE kind=? AND id=?",
                params![kind, id],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|s| serde_json::from_str(&s).context("invalid stored entity JSON"))
            .transpose()
    }

    pub fn list<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        Ok(self
            .list_with_ids::<T>(kind)?
            .into_iter()
            .map(|(_, value)| value)
            .collect())
    }

    /// List entities together with their database keys for identity validation.
    pub fn list_with_ids<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<(String, T)>> {
        let mut statement = self
            .connection
            .prepare("SELECT id,value FROM entities WHERE kind=? ORDER BY rowid DESC")?;
        let rows = statement.query_map([kind], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (id, json) = row?;
            Ok((id, serde_json::from_str(&json)?))
        })
        .collect()
    }

    pub fn put<T: Serialize>(&self, kind: &str, id: &str, value: &T) -> Result<()> {
        self.connection.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)
             ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
            params![kind, id, serde_json::to_string(value)?],
        )?;
        Ok(())
    }

    pub fn remove(&self, kind: &str, id: &str) -> Result<()> {
        self.connection.execute(
            "DELETE FROM entities WHERE kind=? AND id=?",
            params![kind, id],
        )?;
        Ok(())
    }

    pub fn transaction<T>(&mut self, work: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = work(&transaction)?;
        transaction.commit()?;
        Ok(value)
    }

    pub fn event(&self, task: &str, time: &str, kind: &str, text: &str) -> Result<()> {
        // JS used a UTF-16 limit; do not split a surrogate pair when writing Rust UTF-8.
        let mut units = 0;
        let text: String = text
            .chars()
            .take_while(|c| {
                units += c.len_utf16();
                units <= 32000
            })
            .collect();
        self.connection.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)",
            params![task, time, kind, text],
        )?;
        Ok(())
    }

    pub fn events(&self, task: &str) -> Result<Vec<TaskEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT time,kind,text FROM
             (SELECT seq,time,kind,text FROM events WHERE task=? ORDER BY seq DESC LIMIT 500)
             ORDER BY seq",
        )?;
        let rows = statement.query_map([task], |row| {
            Ok(TaskEvent {
                time: row.get(0)?,
                kind: row.get(1)?,
                text: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Call only after filesystem-journal recovery and exclusive application ownership.
    pub fn recover_tasks(&mut self) -> Result<usize> {
        crate::call_log::recover(self)?;
        crate::framework_evidence::recover(self)?;
        crate::framework_operations::recover(self)?;
        crate::asset_task::recover_all(self)?;
        self.transaction(|connection| {
            let mut statement =
                connection.prepare("SELECT id,value FROM entities WHERE kind='task'")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let tasks = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            let mut count = 0;
            for (id, json) in tasks {
                let mut task: Value = serde_json::from_str(&json)?;
                if task["status"] == "waitingChildren" {
                    task["planPaused"] = Value::Bool(true);
                    connection.execute(
                        "UPDATE entities SET value=? WHERE kind='task' AND id=?",
                        params![task.to_string(), id],
                    )?;
                }
                if matches!(
                    task.get("status").and_then(Value::as_str),
                    Some("running" | "queued")
                ) {
                    task["status"] = Value::String("interrupted".into());
                    task["error"] =
                        Value::String("应用退出或执行进程中断，可继续已有任务。".into());
                    connection.execute(
                        "UPDATE entities SET value=? WHERE kind='task' AND id=?",
                        params![serde_json::to_string(&task)?, id],
                    )?;
                    count += 1;
                }
            }
            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserves_legacy_rows_and_unknown_fields() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        let task = json!({"status":"running","prompt":"世界观","future":{"keep":true}});
        store.put("task", "first", &task)?;
        store.put("task", "second", &json!({"status":"awaitingInput"}))?;
        store.put("secret", "code", &"opaque-legacy-ciphertext")?;
        assert_eq!(store.recover_tasks()?, 1);
        assert_eq!(store.recover_tasks()?, 0);
        let saved = store.get::<Value>("task", "first")?.unwrap();
        assert_eq!(saved["future"], task["future"]);
        assert_eq!(saved["status"], "interrupted");
        assert_eq!(
            store.get::<String>("secret", "code")?.unwrap(),
            "opaque-legacy-ciphertext"
        );
        assert_eq!(store.list::<Value>("task")?[0]["status"], "awaitingInput");
        drop(store);
        let reopened = Store::open(temp.path())?;
        assert_eq!(reopened.get::<Value>("task", "first")?, Some(saved));
        Ok(())
    }

    #[test]
    fn failed_transaction_rolls_back_all_writes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        let result: Result<()> = store.transaction(|connection| {
            connection.execute("INSERT INTO entities VALUES('test','one','{}')", [])?;
            anyhow::bail!("injected failure");
        });
        assert!(result.is_err());
        assert!(store.get::<Value>("test", "one")?.is_none());
        Ok(())
    }

    #[test]
    fn events_keep_latest_five_hundred_in_chronological_order() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path())?;
        for i in 0..505 {
            store.event("one", &i.to_string(), "message", "正文")?;
        }
        let events = store.events("one")?;
        assert_eq!(events.len(), 500);
        assert_eq!(events[0].time, "5");
        assert_eq!(events[499].time, "504");
        assert!(store.events("other")?.is_empty());
        Ok(())
    }
}
