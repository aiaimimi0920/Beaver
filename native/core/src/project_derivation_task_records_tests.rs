use super::*;
use crate::{
    project_derivation_copy::Request, project_derivation_identity, store::Store, task_callback,
};
use serde_json::json;

fn fixture(store: &Store) -> Result<IdentityMap> {
    store.put("project", "source", &json!({"id":"source"}))?;
    store.put(
        "task",
        "parent",
        &json!({
            "id":"parent","projectId":"source","status":"completed","subtaskIds":["child"]
        }),
    )?;
    let child = json!({
        "id":"child","projectId":"source","status":"interrupted",
        "parentTaskId":"parent","dependsOn":["parent"],
        "threadId":"historical-thread","turnId":"historical-turn",
        "sessionHistory":[{"threadId":"old-thread"}],
        "workspace":"source/.beaver/workspaces/child",
        "prompt":"Keep source parent child text",
        "feature":{"id":"movement","version":"2","snapshot":{},
            "previous":{"id":"movement","version":"1","snapshot":{},"taskId":"parent"}}
    });
    store.put("task", "child", &child)?;
    store.put(
        "operation",
        "op",
        &json!({
            "id":"op","taskId":"child","projectId":"source","kind":"merge",
            "state":"complete","changes":[],"taskAfter":child
        }),
    )?;
    store.put(
        "feature",
        "source:movement",
        &json!({
            "id":"movement","version":"1","snapshot":{},"taskId":"parent"
        }),
    )?;
    store.put("task-callback-revision", "child", &7)?;
    project_derivation_identity::build(
        &store.connection,
        &Request {
            request_id: "derive".into(),
            source: std::env::temp_dir().join("source"),
            source_project_id: "source".into(),
            target_project_id: "target".into(),
        },
    )
}

#[test]
fn converted_tasks_support_callback_inspection_and_consistent_journal_adoption() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(&temp.path().join("source"))?;
    let map = fixture(&source)?;
    let target = Store::open(&temp.path().join("target"))?;
    target.put("project", "target", &json!({"id":"target"}))?;
    for kind in ["task", "operation", "feature", "task-callback-revision"] {
        for (id, value) in source.list_with_ids::<Value>(kind)? {
            let (key, converted) = rewrite(&map, kind, &id, &value)?;
            target.put(&key.kind, &key.id, &converted)?;
        }
    }
    let child = Rewrite(&map).key("task", "child")?.id;
    let parent = Rewrite(&map).key("task", "parent")?.id;
    let result = task_callback::inspect(&target, &child, None)?;
    assert_eq!(result["revision"], 7);
    assert_eq!(result["task"]["id"], child);
    assert_eq!(result["task"]["projectId"], "target");
    assert_eq!(result["task"]["parentTaskId"], parent);
    assert_eq!(result["task"]["threadId"], "historical-thread");
    let converted: Value = target.get("task", &child)?.unwrap();
    let operation: Value = target.list("operation")?.pop().unwrap();
    assert_eq!(operation["taskAfter"], converted);
    assert_eq!(operation["taskId"], child);
    assert_eq!(operation["projectId"], "target");
    let adoption: Value = target.get("feature", "target:movement")?.unwrap();
    assert_eq!(adoption, converted["feature"]["previous"]);
    assert_eq!(converted["dependsOn"], json!([parent]));
    let parent_record: Value = target.get("task", &parent)?.unwrap();
    assert_eq!(parent_record["subtaskIds"], json!([child]));
    let original: Value = source.get("task", "child")?.unwrap();
    for field in [
        "prompt",
        "workspace",
        "threadId",
        "turnId",
        "sessionHistory",
    ] {
        assert_eq!(original[field], converted[field]);
    }
    assert_eq!(original["projectId"], "source");
    assert_eq!(original["parentTaskId"], "parent");
    // Run the real ownership/reference preflight on the converted database as a new source.
    project_derivation_identity::build(
        &target.connection,
        &Request {
            request_id: "verify".into(),
            source: temp.path().join("target"),
            source_project_id: "target".into(),
            target_project_id: "third".into(),
        },
    )?;
    Ok(())
}

#[test]
fn refuses_inconsistent_journal_feature_and_missing_task_references() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(temp.path())?;
    let map = fixture(&source)?;
    let mut operation: Value = source.get("operation", "op")?.unwrap();
    operation["taskAfter"]["id"] = json!("parent");
    assert!(rewrite(&map, "operation", "op", &operation).is_err());
    let mut feature: Value = source.get("feature", "source:movement")?.unwrap();
    feature["id"] = json!("different");
    assert!(rewrite(&map, "feature", "source:movement", &feature).is_err());
    let mut child: Value = source.get("task", "child")?.unwrap();
    child["dependsOn"] = json!(["missing"]);
    assert!(rewrite(&map, "task", "child", &child).is_err());
    child["dependsOn"] = json!([123]);
    assert!(rewrite(&map, "task", "child", &child).is_err());
    assert!(rewrite(&map, "task-callback-revision", "child", &json!(-1)).is_err());
    Ok(())
}
