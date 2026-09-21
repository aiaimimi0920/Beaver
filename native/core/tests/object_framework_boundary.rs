use anyhow::Result;
use beaver_core::{
    asset_task, files::Files, object_framework, store::Store, task_actions, task_callback,
    task_create, task_finish, task_plan,
};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

fn marker() -> Value {
    json!({"schemaVersion":1,"layer":"coarse"})
}

fn disabled<T: std::fmt::Debug>(result: Result<T>) {
    assert!(format!("{:#}", result.unwrap_err()).contains("OBJECT_FRAMEWORK_DISABLED"));
}

#[test]
fn shared_identity_samples_roundtrip_and_never_fall_back_to_legacy() -> Result<()> {
    let samples: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/object-framework-identities.json"
    ))?;
    for sample in samples["accepted"].as_array().unwrap() {
        let task = json!({"objectFramework":sample});
        let identity = object_framework::identity(&task)?.unwrap();
        assert_eq!(serde_json::to_value(identity)?, *sample);
        disabled(object_framework::require_legacy(&task));
    }
    for sample in samples["rejected"].as_array().unwrap() {
        let task = json!({"objectFramework":sample});
        assert!(object_framework::marked(&task));
        assert!(
            format!("{:#}", object_framework::identity(&task).unwrap_err())
                .contains("INVALID_OBJECT_FRAMEWORK")
        );
    }
    for task in [
        json!({}),
        json!({"assetTask":true}),
        json!({"assetTask":false}),
    ] {
        assert!(!object_framework::marked(&task));
        object_framework::require_legacy(&task)?;
    }
    Ok(())
}

#[test]
fn create_rejects_marker_before_project_lookup_or_filesystem_work() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let root = temp.path().join("untouched");
    let files = Files::new(root.clone());
    for sample in [
        marker(),
        Value::Null,
        json!({"schemaVersion":99,"layer":"coarse"}),
    ] {
        let result = task_create::create(
            &mut store,
            &files,
            json!({"projectId":"missing","prompt":"Create","objectFramework":sample}),
            &json!({}),
            &json!({}),
        );
        assert!(format!("{:#}", result.unwrap_err()).contains("OBJECT_FRAMEWORK"));
        assert!(!root.exists());
        assert!(store.list::<Value>("task")?.is_empty());
    }
    Ok(())
}

#[test]
fn marked_tasks_cannot_continue_approve_merge_or_expand() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let root = temp.path().join("untouched");
    let files = Files::new(root.clone());
    let task = json!({"id":"task","projectId":"project","status":"completed",
        "objectFramework":marker(),"autoAccept":false});
    store.put("task", "task", &task)?;
    disabled(task_actions::continue_task(
        &mut store, "task", "edit", false,
    ));
    disabled(task_actions::accept(&mut store, "task"));
    disabled(task_actions::rollback(&mut store, &files, "task", vec![]));
    disabled(task_plan::approval(&store, "task", true));
    disabled(task_plan::submit(&store, "task", &json!({})));
    disabled(task_plan::expand(&mut store, &files, &mut task.clone()));
    disabled(task_plan::prepare(&mut store, &files, &mut task.clone()));
    disabled(task_finish::finish(
        &mut store,
        &files,
        "task",
        beaver_core::executor::Outcome::Completed,
        &AtomicBool::new(false),
    ));
    disabled(task_finish::retry_merge(
        &mut store,
        &files,
        "task",
        &AtomicBool::new(false),
    ));
    disabled(asset_task::enable(&store, &task));
    assert_eq!(store.get::<Value>("task", "task")?, Some(task));
    assert!(store.events("task")?.is_empty());
    assert!(!root.exists());
    Ok(())
}

#[test]
fn mixed_plan_records_do_not_prepare_or_inherit_legacy_approval() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let root = temp.path().join("untouched");
    let files = Files::new(root.clone());
    let parent = json!({"id":"parent","status":"waitingChildren","objectFramework":marker()});
    let mut child = json!({"id":"child","parentTaskId":"parent","workspacePrepared":false});
    store.put("task", "parent", &parent)?;
    assert!(!task_plan::eligible(&child, &[parent.clone()]));
    disabled(task_plan::prepare(&mut store, &files, &mut child));
    assert!(!root.exists());
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(store.get::<Value>("task", "parent")?, Some(parent));

    let parent =
        json!({"id":"parent","status":"waitingChildren","subtaskIds":["child"],"autoAccept":false});
    let child = json!({"id":"child","parentTaskId":"parent","workspacePrepared":true,
        "status":"completed","accepted":true,"objectFramework":marker(),"autoAccept":false});
    store.put("task", "parent", &parent)?;
    store.put("task", "child", &child)?;
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(store.get::<Value>("task", "parent")?, Some(parent));
    assert!(!task_plan::eligible(
        &json!({"dependsOn":["child"]}),
        &[child.clone()]
    ));
    let mut child = child;
    child["accepted"] = json!(false);
    store.put("task", "child", &child)?;
    task_plan::approval(&store, "parent", true)?;
    assert_eq!(store.get::<Value>("task", "child")?, Some(child));
    Ok(())
}

#[test]
fn marking_a_legacy_task_blocks_callback_replay_without_changing_receipts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let mut task = json!({"id":"task","projectId":"project","status":"running",
        "threadId":"thread","turnId":"turn","prompt":"Create"});
    store.put("task", "task", &task)?;
    let request = json!({"operation":"report","requestId":"report-1","expectedRevision":0,
        "report":{"kind":"result","summary":"Saved","inputs":{},"outputs":{},"tools":[]}});
    task_callback::call(&mut store, "task", "thread", "turn", &request)?;
    let before = task_callback::inspect(&store, "task", None)?;
    task["objectFramework"] = marker();
    store.put("task", "task", &task)?;
    disabled(task_callback::call(
        &mut store, "task", "thread", "turn", &request,
    ));
    let after = task_callback::inspect(&store, "task", None)?;
    assert_eq!(before["receipts"], after["receipts"]);
    assert_eq!(before["revision"], after["revision"]);
    assert_eq!(store.events("task")?.len(), 1);
    Ok(())
}
