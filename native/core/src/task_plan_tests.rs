use crate::{
    executor::Outcome, files::Files, store::Store, task_actions, task_create, task_finish,
    task_plan,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn plan() -> Value {
    json!({"summary":"根据确认的目标执行两个阶段", "steps":[
        {"title":"角色设定","prompt":"处理用户确认的角色","direction":"story","acceptance":"角色资料可读取"},
        {"title":"流程校验","prompt":"验证前置资料","direction":"review","acceptance":"前置资料被验证"}
    ]})
}
fn setup() -> Result<(tempfile::TempDir, Store, Files, Value)> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("data");
    let project = temp.path().join("project");
    fs::create_dir(&project)?;
    fs::write(project.join("before.txt"), "original")?;
    let mut store = Store::open(&root)?;
    let files = Files::new(root.clone());
    store.put(
        "project",
        "p",
        &json!({"id":"p","path":project,"name":"test"}),
    )?;
    let mut parent = task_create::create(
        &mut store,
        &files,
        json!({"projectId":"p","prompt":"综合目标","autoAccept":true}),
        &Value::Null,
        &Value::Null,
    )?;
    parent["status"] = json!("running");
    parent["threadId"] = json!("thread");
    parent["turnId"] = json!("turn");
    store.put("task", parent["id"].as_str().unwrap(), &parent)?;
    Ok((temp, store, files, parent))
}
fn submit(store: &Store, parent: &Value) -> Result<()> {
    task_plan::submit(
        store,
        parent["id"].as_str().unwrap(),
        &json!({"threadId":"thread","turnId":"turn","arguments":plan()}),
    )
}
#[test]
fn rejects_stale_duplicate_and_incomplete_plans() -> Result<()> {
    let (_temp, store, _files, parent) = setup()?;
    let id = parent["id"].as_str().unwrap();
    assert!(
        task_plan::submit(&store, id, &json!({"threadId":"other","arguments":plan()})).is_err()
    );
    let mut bad = plan();
    bad["steps"] = json!([]);
    assert!(task_plan::validate(bad).is_err());
    let mut bad = plan();
    bad["steps"][1]["title"] = bad["steps"][0]["title"].clone();
    assert!(task_plan::validate(bad).is_err());
    let mut pending = parent.clone();
    pending["clarifications"] = json!([{"id":"q"}]);
    store.put("task", id, &pending)?;
    assert!(submit(&store, &parent).is_err());
    store.put("task", id, &parent)?;
    submit(&store, &parent)?;
    assert!(submit(&store, &parent).is_err());
    Ok(())
}
#[test]
fn plan_creates_real_children_once_and_uses_latest_approved_files() -> Result<()> {
    let (_temp, mut store, files, parent) = setup()?;
    submit(&store, &parent)?;
    let id = parent["id"].as_str().unwrap();
    let parent = task_finish::finish(
        &mut store,
        &files,
        id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(parent["status"], "waitingChildren");
    let ids: Vec<String> = serde_json::from_value(parent["subtaskIds"].clone())?;
    assert_eq!(ids.len(), 2);
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(store.list::<Value>("task")?.len(), 3);
    let mut first: Value = store.get("task", &ids[0])?.unwrap();
    let mut second: Value = store.get("task", &ids[1])?.unwrap();
    assert!(!Path::new(second["workspace"].as_str().unwrap()).exists());
    assert!(!task_plan::eligible(&second, &store.list("task")?));
    task_plan::prepare(&mut store, &files, &mut first)?;
    fs::write(
        Path::new(first["workspace"].as_str().unwrap()).join("result.txt"),
        "stage one",
    )?;
    first["status"] = json!("running");
    store.put("task", &ids[0], &first)?;
    let first = task_finish::finish(
        &mut store,
        &files,
        &ids[0],
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(first["approvalSource"], "automatic");
    assert_eq!(first["accepted"], true);
    assert!(task_plan::eligible(&second, &store.list("task")?));
    task_plan::prepare(&mut store, &files, &mut second)?;
    assert_eq!(
        fs::read_to_string(Path::new(second["workspace"].as_str().unwrap()).join("result.txt"))?,
        "stage one"
    );
    second["status"] = json!("running");
    store.put("task", &ids[1], &second)?;
    task_finish::finish(
        &mut store,
        &files,
        &ids[1],
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    let parent: Value = store.get("task", id)?.unwrap();
    assert_eq!(parent["status"], "queued");
    assert_eq!(parent["integrationValidation"], true);
    assert_eq!(parent["validationOnly"], true);
    assert_ne!(parent["accepted"], true);
    Ok(())
}
#[test]
fn manual_approval_blocks_dependents_and_restart_preserves_plan() -> Result<()> {
    let (_temp, mut store, files, parent) = setup()?;
    let id = parent["id"].as_str().unwrap();
    task_plan::approval(&store, id, false)?;
    submit(&store, &parent)?;
    let parent = task_finish::finish(
        &mut store,
        &files,
        id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    let ids: Vec<String> = serde_json::from_value(parent["subtaskIds"].clone())?;
    let mut child: Value = store.get("task", &ids[0])?.unwrap();
    task_plan::prepare(&mut store, &files, &mut child)?;
    child["status"] = json!("running");
    store.put("task", &ids[0], &child)?;
    let child = task_finish::finish(
        &mut store,
        &files,
        &ids[0],
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_ne!(child["accepted"], true);
    let second: Value = store.get("task", &ids[1])?.unwrap();
    assert!(!task_plan::eligible(&second, &store.list("task")?));
    task_actions::accept(&mut store, &ids[0])?;
    assert!(task_plan::eligible(&second, &store.list("task")?));
    store.recover_tasks()?;
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(store.list::<Value>("task")?.len(), 3);
    assert_eq!(
        store.get::<Value>("task", &ids[1])?.unwrap()["status"],
        "interrupted"
    );
    assert_eq!(
        store.get::<Value>("task", id)?.unwrap()["status"],
        "waitingChildren"
    );
    Ok(())
}
#[test]
fn missing_plan_and_conflicts_never_create_or_approve_children() -> Result<()> {
    let (_temp, mut store, files, parent) = setup()?;
    let id = parent["id"].as_str().unwrap();
    let failed = task_finish::finish(
        &mut store,
        &files,
        id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(failed["status"], "failed");
    assert_eq!(store.list::<Value>("task")?.len(), 1);
    store.put("task", id, &parent)?;
    submit(&store, &parent)?;
    fs::write(
        Path::new(parent["workspace"].as_str().unwrap()).join("before.txt"),
        "ai",
    )?;
    let p: Value = store.get("project", "p")?.unwrap();
    fs::write(
        Path::new(p["path"].as_str().unwrap()).join("before.txt"),
        "human",
    )?;
    let conflict = task_finish::finish(
        &mut store,
        &files,
        id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(conflict["status"], "conflict");
    assert_ne!(conflict["accepted"], true);
    assert_eq!(store.list::<Value>("task")?.len(), 1);
    Ok(())
}
