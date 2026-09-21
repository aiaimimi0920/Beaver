use super::*;
use crate::{
    project_derivation_copy::Request, project_derivation_identity, store::Store, task_callback,
};
use serde_json::json;

#[test]
fn converted_callback_receipt_preserves_exact_retry_and_rejects_conflicts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut source = Store::open(&temp.path().join("source"))?;
    source.put("project", "source", &json!({"id":"source"}))?;
    source.put(
        "task",
        "task",
        &json!({
            "id":"task","projectId":"source","status":"running",
            "threadId":"thread","turnId":"turn"
        }),
    )?;
    let input = json!({"operation":"report","requestId":"report","expectedRevision":0,
        "report":{"kind":"progress","summary":"Keep source task text",
            "inputs":{"taskId":"opaque reported data"},"outputs":{},"tools":[]}});
    let response = task_callback::call(&mut source, "task", "thread", "turn", &input)?;
    let original: Value = source.get("task-callback/task", "report")?.unwrap();
    let map = project_derivation_identity::build(
        &source.connection,
        &Request {
            request_id: "derive".into(),
            source: temp.path().join("source"),
            source_project_id: "source".into(),
            target_project_id: "target".into(),
        },
    )?;
    let mut target = Store::open(&temp.path().join("target"))?;
    for kind in ["task", "task-callback-revision", "task-callback/task"] {
        for (id, value) in source.list_with_ids::<Value>(kind)? {
            let (key, converted) = rewrite(&map, kind, &id, &value)?;
            target.put(&key.kind, &key.id, &converted)?;
        }
    }
    let task = Rewrite(&map).key("task", "task")?.id;
    let state = task_callback::inspect(&target, &task, Some("report"))?;
    let receipt = &state["receipt"];
    assert_eq!(receipt["taskId"], task);
    assert_eq!(receipt["projectId"], "target");
    assert_eq!(receipt["request"], input);
    assert_eq!(receipt["id"], original["id"]);
    assert_eq!(state["revision"], 1);
    assert_eq!(
        task_callback::call(&mut target, &task, "thread", "turn", &input)?,
        response
    );
    let mut conflicting = input.clone();
    conflicting["report"]["summary"] = json!("changed");
    assert!(
        task_callback::call(&mut target, &task, "thread", "turn", &conflicting)
            .unwrap_err()
            .to_string()
            .contains("Request ID conflict")
    );
    assert!(task_callback::call(&mut target, &task, "other", "turn", &input).is_err());
    for (path, bad) in [
        ("/taskId", json!("other")),
        ("/requestId", json!("other")),
        ("/request/requestId", json!("other")),
        ("/response/requestId", json!("other")),
        ("/request", json!({"operation":"state"})),
    ] {
        let mut corrupt = original.clone();
        *corrupt.pointer_mut(path).unwrap() = bad;
        assert!(
            rewrite(&map, "task-callback/task", "report", &corrupt).is_err(),
            "{path}"
        );
    }
    assert_eq!(
        source
            .get::<Value>("task-callback/task", "report")?
            .unwrap(),
        original
    );
    Ok(())
}
