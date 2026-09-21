use super::*;
use crate::{call_log, framework_evidence, store::Store, validation::requests};
use serde_json::json;
use std::{fs::File, sync::Arc};

fn request() -> Request {
    Request {
        request_id: "derive".into(),
        source: std::env::temp_dir().join("source"),
        source_project_id: "source".into(),
        target_project_id: "target".into(),
    }
}

#[test]
fn composition_preserves_history_and_archives_receipts_without_target_replay() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut source = Store::open(&temp.path().join("source"))?;
    source.put(
        "project",
        "source",
        &json!({"id":"source","name":"source task"}),
    )?;
    source.put(
        "task",
        "task",
        &json!({"id":"task","projectId":"source","status":"running","prompt":"source task"}),
    )?;
    source.put(
        "validationSettings",
        "source",
        &json!({"projectId":"source","revision":3}),
    )?;
    source.put(
        "asset-reference",
        "reference",
        &json!({"id":"reference","taskId":"task","projectId":"source",
            "imagePath":"source/reference.png","sha256":"content","used":false,
            "frame":{"id":"frame","sessionId":"session","generation":"generation",
                "sceneRevision":1,"viewRevision":2,"capturedAt":3,"width":100,"height":100,
                "viewMatrix":vec![0.0;16],"projectionMatrix":vec![0.0;16]},"pick":null}),
    )?;
    let input = json!({"projectId":"source","requestId":"save","revision":3});
    let receipt = requests::Request::new("settings.save", &input)?;
    let result = json!({"id":"source","taskId":"task"});
    let (_, receipt_id, receipt_value) = receipt.record(&result);
    requests::commit(&mut source, vec![receipt.record(&result)])?;
    let raw = format!("  {}\n", serde_json::to_string_pretty(&receipt_value)?);
    source.connection.execute(
        "UPDATE entities SET value=? WHERE kind='validationRequest' AND id=?",
        params![raw, receipt_id],
    )?;
    let call = call_log::begin(&source, "api", "tool", Some("task"), Some("source"), &input)?;
    framework_evidence::start(&source, "task", &call, "tool", &input, None, false)?;
    source.connection.execute(
        "INSERT INTO events VALUES(17,'task','then','note','source task')",
        [],
    )?;
    let map = project_derivation_identity::build(&source.connection, &request())?;
    let staged = compose(&source.connection, &request(), &map)?;
    assert_eq!(
        staged.archive.records,
        vec![ArchivedRecord {
            source: Key {
                kind: "validationRequest".into(),
                id: receipt_id
            },
            raw_json: raw,
        }]
    );
    let encoded = serde_json::to_string(&staged.archive)?;
    assert_eq!(serde_json::from_str::<Archive>(&encoded)?, staged.archive);
    let target = Store::project(
        staged.connection,
        Arc::new(File::create(temp.path().join("lock"))?),
    );
    assert_eq!(receipt.replay(&source)?, Some(result));
    assert!(receipt.replay(&target)?.is_none());
    let target_receipt = requests::Request::new(
        "settings.save",
        &json!({"projectId":"target","requestId":"save","revision":3}),
    )?;
    assert!(target_receipt.replay(&target)?.is_none());
    assert!(target.list::<Value>("validationRequest")?.is_empty());
    let task = Rewrite(&map).key("task", "task")?.id;
    let converted: Value = target.get("task", &task)?.unwrap();
    assert_eq!(converted["projectId"], "target");
    assert_eq!(converted["prompt"], "source task");
    assert_eq!(converted["status"], "running");
    let reference = Rewrite(&map).key("asset-reference", "reference")?;
    assert_eq!(
        target
            .get::<Value>(&reference.kind, &reference.id)?
            .unwrap()["taskId"],
        task
    );
    let trace: Value = target
        .get(&format!("framework-trace/{task}"), &map.calls[&call])?
        .unwrap();
    assert_eq!(trace["id"], map.calls[&call]);
    let calls = call_log::query(&target, &json!({"taskId":task,"projectId":"target"}))?;
    assert_eq!(calls["records"][0]["id"], trace["id"]);
    assert_eq!(target.events(&task)?.len(), 1);
    assert_eq!(
        target
            .get::<Value>("validationSettings", "target")?
            .unwrap()["revision"],
        3
    );
    assert!(target.get::<Value>("project", "source")?.is_none());
    assert_eq!(
        source.get::<Value>("task", "task")?.unwrap()["projectId"],
        "source"
    );
    project_derivation_validation::validate(&target.connection, "target")?;
    Ok(())
}

#[test]
fn refuses_tampered_map_and_late_conversion_failure_without_source_mutation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(temp.path())?;
    source.put("project", "source", &json!({"id":"source"}))?;
    source.put("task", "task", &json!({"id":"task","projectId":"source"}))?;
    let mut map = project_derivation_identity::build(&source.connection, &request())?;
    map.entities.pop();
    assert!(compose(&source.connection, &request(), &map).is_err());
    source.put("task-callback-revision", "task", &json!("invalid revision"))?;
    let map = project_derivation_identity::build(&source.connection, &request())?;
    assert!(compose(&source.connection, &request(), &map).is_err());
    assert!(source.connection.is_autocommit());
    assert_eq!(
        source.get::<Value>("task-callback-revision", "task")?,
        Some(json!("invalid revision"))
    );
    source.put("task-callback-revision", "task", &json!(7))?;
    let map = project_derivation_identity::build(&source.connection, &request())?;
    let staged = compose(&source.connection, &request(), &map)?;
    assert!(staged.archive.records.is_empty());
    Ok(())
}
