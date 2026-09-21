use super::*;
use crate::{
    call_log, framework_evidence, project_derivation_copy::Request,
    project_derivation_framework_records, project_derivation_identity, store::Store,
};
use serde_json::json;

#[test]
fn history_keeps_real_reader_cursors_summaries_and_trace_correlation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(&temp.path().join("source"))?;
    source.put("project", "source", &json!({"id":"source"}))?;
    source.put("task", "task", &json!({"id":"task","projectId":"source"}))?;
    let item = json!({"id":"external-item","taskId":"opaque source task"});
    let call = call_log::begin(
        &source,
        "codex-tool",
        "tool",
        Some("task"),
        Some("source"),
        &item,
    )?;
    framework_evidence::start(&source, "task", &call, "tool", &item, None, false)?;
    call_log::finish(&source, &call, "succeeded", 7, &json!({"id":"source"}))?;
    let pruned = call_log::begin(
        &source,
        "codex-tool",
        "tool",
        Some("task"),
        Some("source"),
        &item,
    )?;
    framework_evidence::start(&source, "task", &pruned, "tool", &item, None, false)?;
    source
        .connection
        .execute("DELETE FROM calls WHERE id=?", [&pruned])?;
    let project_call = call_log::begin(
        &source,
        "api",
        "project.inspect",
        None,
        Some("source"),
        &json!({}),
    )?;
    let text = "{\"taskId\":\"task\",\"requestId\":\"historical-pointer\"}";
    let tail = call_log::begin(&source, "api", "removed", None, Some("source"), &json!({}))?;
    let last_sequence: i64 =
        source
            .connection
            .query_row("SELECT seq FROM calls WHERE id=?", [&tail], |row| {
                row.get(0)
            })?;
    source
        .connection
        .execute("DELETE FROM calls WHERE id=?", [&tail])?;
    source.connection.execute(
        "INSERT INTO events(seq,task,time,kind,text) VALUES(42,'task','then','decision',?)",
        [text],
    )?;
    let request = Request {
        request_id: "derive".into(),
        source: temp.path().join("source"),
        source_project_id: "source".into(),
        target_project_id: "target".into(),
    };
    let map = project_derivation_identity::build(&source.connection, &request)?;
    let before = call_log::query(&source, &json!({}))?;
    let mut target = Store::open(&temp.path().join("target"))?;
    let tx = target.connection.transaction()?;
    copy(&source.connection, &map, &tx)?;
    tx.commit()?;
    let task = Rewrite(&map).key("task", "task")?.id;
    let found = call_log::query(&target, &json!({"taskId":task,"projectId":"target"}))?;
    let record = &found["records"][0];
    assert_eq!(found["records"].as_array().unwrap().len(), 1);
    assert_eq!(record["id"], map.calls[&call]);
    for field in ["input", "output", "durationMs", "status", "seq"] {
        assert_eq!(record[field], before["records"][0][field]);
    }
    let next = call_log::query(&target, &json!({"after":found["nextAfter"]}))?;
    assert_eq!(next["records"][0]["id"], map.calls[&project_call]);
    assert!(next["records"][0]["taskId"].is_null());
    for id in [&call, &pruned] {
        let original: Value = source.get("framework-trace/task", id)?.unwrap();
        let (key, converted) = project_derivation_framework_records::rewrite(
            &source.connection,
            &map,
            "framework-trace/task",
            id,
            &original,
        )?;
        assert_eq!(
            key.id,
            project_derivation_identity::generated(&request, "calls", id)?
        );
        assert_eq!(converted["id"], key.id);
        assert_eq!(converted["itemId"], "external-item");
        target.put(&key.kind, &key.id, &converted)?;
    }
    let trace: Value = target
        .get(&format!("framework-trace/{task}"), &map.calls[&call])?
        .unwrap();
    assert_eq!(trace["context"]["taskId"], task);
    assert_eq!(target.events(&task)?[0].text, text);
    assert!(target.events("task")?.is_empty());
    call_log::begin(&target, "api", "new", None, Some("target"), &json!({}))?;
    let fresh = call_log::query(&target, &json!({"after":last_sequence}))?;
    assert_eq!(fresh["records"].as_array().unwrap().len(), 1);
    assert_eq!(fresh["records"][0]["method"], "new");
    assert_eq!(call_log::query(&source, &json!({}))?, before);
    assert_eq!(source.events("task")?[0].text, text);
    Ok(())
}

#[test]
fn failed_history_conversion_rolls_back_events_and_rejects_nonempty_target() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(&temp.path().join("source"))?;
    source.put("project", "source", &json!({"id":"source"}))?;
    source.put("task", "task", &json!({"id":"task","projectId":"source"}))?;
    let call = call_log::begin(
        &source,
        "api",
        "tool",
        Some("task"),
        Some("source"),
        &json!({}),
    )?;
    source.connection.execute(
        "INSERT INTO events(task,time,kind,text) VALUES('task','then','user','keep')",
        [],
    )?;
    let mut map = project_derivation_identity::build(
        &source.connection,
        &Request {
            request_id: "derive".into(),
            source: temp.path().join("source"),
            source_project_id: "source".into(),
            target_project_id: "target".into(),
        },
    )?;
    let mapped = map.calls.remove(&call).unwrap();
    let mut target = Store::open(&temp.path().join("target"))?;
    {
        let tx = target.connection.transaction()?;
        assert!(copy(&source.connection, &map, &tx).is_err());
    }
    let count: i64 = target
        .connection
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
    assert_eq!(count, 0);
    map.calls.insert(call.clone(), mapped);
    source
        .connection
        .execute("UPDATE calls SET method='corrupt' WHERE id=?", [&call])?;
    {
        let tx = target.connection.transaction()?;
        assert!(copy(&source.connection, &map, &tx).is_err());
    }
    source
        .connection
        .execute("UPDATE calls SET method='tool' WHERE id=?", [&call])?;
    let tx = target.connection.transaction()?;
    copy(&source.connection, &map, &tx)?;
    tx.commit()?;
    let original = call_log::query(&target, &json!({}))?;
    {
        let tx = target.connection.transaction()?;
        assert!(copy(&source.connection, &map, &tx).is_err());
    }
    assert_eq!(call_log::query(&target, &json!({}))?, original);
    Ok(())
}
