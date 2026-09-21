use crate::store::Store;
use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Arbitrary arguments/results can contain credentials or large assets. Persist shape
/// and a digest, not their contents; task.events remains the redacted narrative log.
pub fn summary(value: &Value) -> Value {
    let bytes = value.to_string();
    json!({"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes.as_bytes())),
        "keys":value.as_object().map(|o|o.keys().take(32).cloned().collect::<Vec<_>>()),
        "workflow":value.get("workflow").filter(|v|v.as_str()==Some("npr-character")),
        "action":value.get("action").filter(|v|matches!(v.as_str(),Some("inspect"|"validate"|"preview"))),
        "ok":value.get("ok").and_then(Value::as_bool),
        "items":value.as_array().map(Vec::len)})
}

pub fn begin(
    store: &Store,
    source: &str,
    method: &str,
    task: Option<&str>,
    project: Option<&str>,
    input: &Value,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let value = json!({"id":id,"source":source,"method":method,"taskId":task,"projectId":project,
        "startedAt":now(),"status":"running","input":summary(input)});
    store.connection.execute(
        "INSERT INTO calls(id,task,project,method,value) VALUES(?,?,?,?,?)",
        params![id, task, project, method, value.to_string()],
    )?;
    // A bounded diagnostic history, independent of task conversation retention.
    store.connection.execute("DELETE FROM calls WHERE seq <= (SELECT COALESCE(MAX(seq),0)-50000 FROM calls) AND json_extract(value,'$.status') != 'running'", [])?;
    Ok(id)
}

pub fn finish(
    store: &Store,
    id: &str,
    status: &str,
    elapsed_ms: u64,
    output: &Value,
) -> Result<()> {
    let elapsed_ms = i64::try_from(elapsed_ms).unwrap_or(i64::MAX);
    store.connection.execute(
        "UPDATE calls SET value=json_set(value,'$.status',?,'$.finishedAt',?,'$.durationMs',?,'$.output',json(?)) WHERE id=?",
        params![status,now(),elapsed_ms,summary(output).to_string(),id])?;
    if status == "failed" {
        use crate::preferences::Vault;
        let error = output.get("error").or_else(|| output.get("errors"));
        if let Some(error) = error {
            let mut text = error
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| error.to_string());
            let mut secrets = store
                .list::<String>("secret")
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| crate::preferences::SystemVault.decrypt(&v).ok())
                .collect::<Vec<_>>();
            if let Ok(token) = std::env::var("BEAVER_API_TOKEN") {
                secrets.push(token);
            }
            secrets.sort_by_key(|v| std::cmp::Reverse(v.len()));
            for secret in secrets.into_iter().filter(|v| !v.is_empty()) {
                text = text.replace(&secret, "[REDACTED_SECRET]");
            }
            static BEARER: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            text = BEARER
                .get_or_init(|| regex::Regex::new(r"(?i)Bearer\s+[A-Za-z0-9._~+/\-]+").unwrap())
                .replace_all(&text, "Bearer [REDACTED_SECRET]")
                .into_owned();
            let text: String = text.chars().take(2000).collect();
            store.connection.execute(
                "UPDATE calls SET value=json_set(value,'$.error',?) WHERE id=?",
                params![text, id],
            )?;
        }
    }
    Ok(())
}

pub fn link(store: &Store, id: &str, task: Option<&str>, project: Option<&str>) -> Result<()> {
    store.connection.execute("UPDATE calls SET task=COALESCE(?,task),project=COALESCE(?,project),value=json_set(value,'$.taskId',COALESCE(?,task),'$.projectId',COALESCE(?,project)) WHERE id=?",
        params![task,project,task,project,id])?;
    Ok(())
}

/// Copy a completed call record into another runtime's log without changing its
/// correlation ID. The destination insert is idempotent so recovery retries do
/// not create duplicate records.
pub fn copy_record(source: &Store, destination: &Store, id: &str) -> Result<bool> {
    let record = source
        .connection
        .query_row(
            "SELECT id,task,project,method,value FROM calls WHERE id=?",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((id, task, project, method, value)) = record else {
        return Ok(false);
    };
    let inserted = destination.connection.execute(
        "INSERT OR IGNORE INTO calls(id,task,project,method,value) VALUES(?,?,?,?,?)",
        params![id, task, project, method, value],
    )?;
    Ok(inserted == 1)
}

pub fn recover(store: &Store) -> Result<()> {
    store.connection.execute("UPDATE calls SET value=json_set(value,'$.status','interrupted','$.finishedAt',?,'$.durationMs',null,'$.output',null) WHERE json_extract(value,'$.status')='running'",[now()])?;
    Ok(())
}

pub fn query(store: &Store, input: &Value) -> Result<Value> {
    let after = input
        .get("after")
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| anyhow::anyhow!("Invalid log cursor"))
        })
        .transpose()?
        .unwrap_or(0);
    let limit = input
        .get("limit")
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| anyhow::anyhow!("Invalid log limit"))
        })
        .transpose()?
        .unwrap_or(100);
    if !(1..=500).contains(&limit) || after > i64::MAX as u64 {
        bail!("Invalid log page");
    }
    let filter = |key: &str| -> Result<Option<&str>> {
        input
            .get(key)
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid log filter"))
            })
            .transpose()
    };
    let mut statement = store.connection.prepare("SELECT seq,value FROM calls WHERE seq>? AND (? IS NULL OR task=?) AND (? IS NULL OR project=?) AND (? IS NULL OR method=?) ORDER BY seq LIMIT ?")?;
    let task = filter("taskId")?;
    let project = filter("projectId")?;
    let method = filter("method")?;
    let rows = statement.query_map(
        params![
            after as i64,
            task,
            task,
            project,
            project,
            method,
            method,
            limit as i64
        ],
        |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?)),
    )?;
    let mut records = Vec::new();
    let mut cursor = after;
    for row in rows {
        let (seq, text) = row?;
        let mut value: Value = serde_json::from_str(&text)?;
        value["seq"] = json!(seq);
        cursor = seq;
        records.push(value);
    }
    Ok(
        json!({"records":records,"nextAfter":cursor,"retention":"latest 50000 sequence positions plus older active calls","payloadPolicy":"shape-and-sha256; raw arguments, credentials and generated assets are not stored"}),
    )
}

pub struct Activity {
    store: std::sync::Arc<std::sync::Mutex<Store>>,
    task: String,
    project: Option<String>,
    spans: std::collections::BTreeMap<String, (String, std::time::Instant)>,
}

impl Activity {
    pub fn new(store: std::sync::Arc<std::sync::Mutex<Store>>, task: &Value) -> Self {
        Self {
            store,
            task: task["id"].as_str().unwrap_or("").into(),
            project: task["projectId"].as_str().map(str::to_owned),
            spans: Default::default(),
        }
    }
    pub fn item(&mut self, item: &Value, completed: bool) -> Result<()> {
        self.item_at(item, completed, None)
    }
    pub fn item_at(
        &mut self,
        item: &Value,
        completed: bool,
        identity: Option<(&str, &str)>,
    ) -> Result<()> {
        let kind = item["type"].as_str().unwrap_or("");
        if !matches!(
            kind,
            "mcpToolCall"
                | "commandExecution"
                | "dynamicToolCall"
                | "webSearch"
                | "imageGeneration"
        ) {
            return Ok(());
        }
        let key = item["id"].as_str().context("Tool item has no id")?;
        if !self.spans.contains_key(key) {
            let name = if kind == "mcpToolCall" {
                format!(
                    "{}.{}",
                    item["server"].as_str().unwrap_or("mcp"),
                    item["tool"].as_str().unwrap_or("unknown")
                )
            } else if kind == "dynamicToolCall" {
                item["tool"].as_str().unwrap_or(kind).into()
            } else {
                kind.into()
            };
            let name: String = name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || "._-".contains(*c))
                .take(160)
                .collect();
            let store = self
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            let id = begin(
                &store,
                "codex-tool",
                &name,
                Some(&self.task),
                self.project.as_deref(),
                item,
            )?;
            store.connection.execute(
                "UPDATE calls SET value=json_set(value,'$.itemId',?) WHERE id=?",
                params![key, id],
            )?;
            crate::framework_evidence::start(
                &store, &self.task, &id, &name, item, identity, completed,
            )?;
            self.spans
                .insert(key.into(), (id, std::time::Instant::now()));
        }
        if completed {
            if let Some((id, start)) = self.spans.remove(key) {
                let failed = item["status"] == "failed"
                    || item["status"] == "declined"
                    || item["error"].is_object()
                    || item["error"].is_string()
                    || item["result"]["isError"] == true
                    || item["exitCode"].as_i64().is_some_and(|n| n != 0);
                let store = self
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
                finish(
                    &store,
                    &id,
                    if failed { "failed" } else { "succeeded" },
                    start.elapsed().as_millis() as u64,
                    item,
                )?;
                crate::framework_evidence::finish(
                    &store,
                    &self.task,
                    &id,
                    if failed { "failed" } else { "succeeded" },
                    item,
                )?;
            }
        }
        Ok(())
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        if let Ok(store) = self.store.lock() {
            for (id, start) in self.spans.values() {
                let _ = crate::framework_evidence::finish(
                    &store,
                    &self.task,
                    id,
                    "interrupted",
                    &Value::Null,
                );
                let _ = finish(
                    &store,
                    id,
                    "interrupted",
                    start.elapsed().as_millis() as u64,
                    &Value::Null,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correlates_pages_and_recovers_without_persisting_secret_payloads() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path())?;
        let id = begin(
            &store,
            "api",
            "settings.save",
            None,
            None,
            &json!({"keys":{"code":"sentinel-secret"}}),
        )?;
        finish(
            &store,
            &id,
            "failed",
            12,
            &json!({"error":"Bearer sentinel-secret"}),
        )?;
        let other = begin(
            &store,
            "codex",
            "blender.execute_code",
            Some("task-1"),
            Some("project-1"),
            &json!({"code":"secret-source"}),
        )?;
        link(&store, &id, Some("task-2"), Some("project-1"))?;
        let page = query(&store, &json!({"limit":1}))?;
        assert_eq!(page["records"][0]["durationMs"], 12);
        assert!(!page.to_string().contains("sentinel-secret"));
        let next = query(
            &store,
            &json!({"after":page["nextAfter"],"taskId":"task-1"}),
        )?;
        assert_eq!(next["records"][0]["id"], other);
        assert!(!next.to_string().contains("secret-source"));
        recover(&store)?;
        assert_eq!(
            query(&store, &json!({"taskId":"task-1"}))?["records"][0]["status"],
            "interrupted"
        );
        assert!(query(&store, &json!({"limit":0})).is_err());
        drop(store);
        assert_eq!(
            query(
                &Store::open(temp.path())?,
                &json!({"projectId":"project-1"})
            )?["records"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        Ok(())
    }

    #[test]
    fn tool_calls_pair_results_and_mark_unfinished_work_interrupted() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = std::sync::Arc::new(std::sync::Mutex::new(Store::open(temp.path())?));
        {
            let mut activity =
                Activity::new(store.clone(), &json!({"id":"task","projectId":"project"}));
            let mut item = json!({"id":"item-1","type":"mcpToolCall","server":"blender","tool":"execute_code","arguments":{"code":"private-code"},"status":"inProgress"});
            activity.item(&item, false)?;
            item["status"] = json!("failed");
            item["error"] = json!({"message":"Server unavailable"});
            activity.item(&item, true)?;
            activity.item(&json!({"id":"item-2","type":"commandExecution"}), false)?;
        }
        let store = store.lock().unwrap();
        let rows = query(&store, &json!({"taskId":"task"}))?;
        assert_eq!(rows["records"].as_array().unwrap().len(), 2);
        assert_eq!(rows["records"][0]["status"], "failed");
        assert_eq!(rows["records"][0]["method"], "blender.execute_code");
        assert_eq!(rows["records"][1]["status"], "interrupted");
        assert!(!rows.to_string().contains("private-code"));
        Ok(())
    }

    #[test]
    fn copies_completed_records_idempotently_between_runtime_stores() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = Store::open(&temp.path().join("source"))?;
        let destination = Store::open(&temp.path().join("destination"))?;
        let id = begin(
            &source,
            "api",
            "project.create",
            None,
            Some("project-a"),
            &json!({"name":"Demo"}),
        )?;
        link(&source, &id, None, Some("project-a"))?;
        finish(&source, &id, "succeeded", 7, &json!({"id":"project-a"}))?;

        assert!(copy_record(&source, &destination, &id)?);
        assert!(!copy_record(&source, &destination, &id)?);
        let page = query(&destination, &json!({"projectId":"project-a"}))?;
        assert_eq!(page["records"].as_array().unwrap().len(), 1);
        assert_eq!(page["records"][0]["id"], id);
        assert_eq!(page["records"][0]["status"], "succeeded");
        assert_eq!(page["records"][0]["durationMs"], 7);
        Ok(())
    }
}
