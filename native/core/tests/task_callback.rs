use anyhow::Result;
use beaver_core::{asset_stages, asset_task, store::Store, task_callback};
use serde_json::{json, Value};

fn fixture() -> Result<(tempfile::TempDir, Store)> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    for id in ["task", "other"] {
        store.put(
            "task",
            id,
            &json!({"id":id,"projectId":"project","status":"running",
            "threadId":format!("thread-{id}"),"turnId":"turn","prompt":"Make a character"}),
        )?;
    }
    Ok((temp, store))
}

fn report(id: &str, revision: u64) -> Value {
    json!({"operation":"report","requestId":id,"expectedRevision":revision,"report":{
        "kind":"result","summary":"Draft saved; visual review pending",
        "inputs":{"design":"design.md"},"outputs":{"candidate":"character.blend"},
        "tools":[{"kind":"plugin","name":"example","version":"observed-1"}]}})
}

fn call(store: &mut Store, input: &Value) -> Result<Value> {
    task_callback::call(store, "task", "thread-task", "turn", input)
}

fn stage(id: &str, status: &str) -> Value {
    json!({"id":id,"name":id,"status":status,"dependencies":[],"objects":[],"evidence":"", "round":1})
}

#[test]
fn callback_retries_share_receipt_across_adapters_and_survive_restart() -> Result<()> {
    let (temp, mut store) = fixture()?;
    let request = report("first", 0);
    let original = task_callback::dynamic(
        &mut store,
        "task",
        &json!({"threadId":"thread-task",
        "turnId":"turn","arguments":request}),
    )?;
    call(&mut store, &report("second", 1))?;
    let through_owner = task_callback::business(
        &mut store,
        "task.callback",
        &json!({
        "id":"task","threadId":"thread-task","turnId":"turn","request":request}),
    )?;
    assert_eq!(original, through_owner);
    drop(store);
    let mut store = Store::open(temp.path())?;
    assert_eq!(call(&mut store, &request)?, original);
    let state = call(&mut store, &json!({"operation":"state"}))?;
    assert_eq!(state["revision"], 2);
    assert_eq!(state["receipts"].as_array().unwrap().len(), 2);
    assert_eq!(store.events("task")?.len(), 2);
    assert_eq!(state["task"]["status"], "running");
    assert_eq!(original["acceptance"], "notEvaluated");
    let receipt = task_callback::inspect(&store, "task", Some("first"))?;
    assert_eq!(receipt["receipt"]["request"], request);
    assert_eq!(receipt["receipt"]["response"], original);
    Ok(())
}

#[test]
fn callback_rejects_stale_cross_task_conflicting_and_unknown_arguments() -> Result<()> {
    let (_temp, mut store) = fixture()?;
    let request = report("first", 0);
    call(&mut store, &request)?;
    let mut changed = request.clone();
    changed["report"]["summary"] = json!("Different result");
    assert!(call(&mut store, &changed)
        .unwrap_err()
        .to_string()
        .contains("conflict"));
    assert!(call(&mut store, &report("new", 0))
        .unwrap_err()
        .to_string()
        .contains("revision"));
    for (id, thread, turn) in [
        ("other", "thread-task", "turn"),
        ("task", "thread-task", "old"),
        ("task", "", "turn"),
    ] {
        assert!(task_callback::call(&mut store, id, thread, turn, &request).is_err());
    }
    let mut injected = report("new", 1);
    injected["taskId"] = json!("other");
    assert!(call(&mut store, &injected).is_err());
    injected = report("new", 1);
    injected["report"]["accepted"] = json!(true);
    assert!(call(&mut store, &injected).is_err());
    assert_eq!(task_callback::inspect(&store, "task", None)?["revision"], 1);
    assert_eq!(
        task_callback::inspect(&store, "other", None)?["revision"],
        0
    );
    Ok(())
}

#[test]
fn callback_owner_can_recover_receipts_but_old_turn_cannot_mutate_or_read() -> Result<()> {
    let (_temp, mut store) = fixture()?;
    let request = report("first", 0);
    let original = call(&mut store, &request)?;
    store.recover_tasks()?;
    assert!(call(&mut store, &request).is_err());
    assert!(call(&mut store, &json!({"operation":"state"})).is_err());
    let receipt = task_callback::business(
        &mut store,
        "task.callbackState",
        &json!({"id":"task","requestId":"first"}),
    )?;
    assert_eq!(receipt["receipt"]["response"], original);
    let mut task: Value = store.get("task", "task")?.unwrap();
    task["status"] = json!("running");
    task["turnId"] = json!("new-turn");
    store.put("task", "task", &task)?;
    assert!(
        task_callback::call(&mut store, "task", "thread-task", "new-turn", &request)
            .unwrap_err()
            .to_string()
            .contains("conflict")
    );
    assert_eq!(
        task_callback::call(
            &mut store,
            "task",
            "thread-task",
            "new-turn",
            &json!({"operation":"receipt","requestId":"first"})
        )?["receipt"]["response"],
        original
    );
    let resumed = task_callback::call(
        &mut store,
        "task",
        "thread-task",
        "new-turn",
        &report("resumed", 1),
    )?;
    assert_eq!(resumed["revision"], 2);
    assert_eq!(
        task_callback::inspect(&store, "task", Some("resumed"))?["receipt"]["turnId"],
        "new-turn"
    );
    Ok(())
}

#[test]
fn callback_stages_reuse_asset_revision_and_reports_do_not_advance_stages() -> Result<()> {
    let (_temp, mut store) = fixture()?;
    let task: Value = store.get("task", "task")?.unwrap();
    asset_task::enable(&store, &task)?;
    let request = json!({"operation":"stages","requestId":"plan","expectedRevision":0,
        "assetRevision":0,"stages":[stage("design","running")]});
    let saved = call(&mut store, &request)?;
    assert_eq!(saved["assetRevision"], 1);
    let mut input = report("draft", 1);
    input["report"]["stageId"] = json!("design");
    input["report"]["assetRevision"] = json!(1);
    call(&mut store, &input)?;
    assert_eq!(asset_task::get(&store, "task")?.stages[0].status, "running");
    let mut state = asset_task::get(&store, "task")?;
    let stages = state.stages.clone();
    asset_stages::update(&mut state, 1, stages)?;
    asset_task::save(&store, &state)?;
    assert_eq!(call(&mut store, &request)?, saved);
    input["requestId"] = json!("stale");
    input["expectedRevision"] = json!(2);
    assert!(call(&mut store, &input)
        .unwrap_err()
        .to_string()
        .contains("Asset revision"));
    input["report"]["assetRevision"] = json!(2);
    input["report"]["stageId"] = json!("unknown");
    assert!(call(&mut store, &input).is_err());
    assert_eq!(task_callback::inspect(&store, "task", None)?["revision"], 2);
    Ok(())
}

#[test]
fn callback_commit_failure_rolls_back_stage_receipt_revision_and_event() -> Result<()> {
    let (temp, mut store) = fixture()?;
    let task: Value = store.get("task", "task")?.unwrap();
    asset_task::enable(&store, &task)?;
    let db = rusqlite::Connection::open(temp.path().join("beaver.sqlite"))?;
    db.execute_batch(
        "CREATE TRIGGER reject_callback_event BEFORE INSERT ON events
        WHEN NEW.kind='taskCallback' BEGIN SELECT RAISE(ABORT, 'simulated disk failure'); END;",
    )?;
    let request = json!({"operation":"stages","requestId":"plan","expectedRevision":0,
        "assetRevision":0,"stages":[stage("design","running")]});
    assert!(call(&mut store, &request).is_err());
    assert_eq!(asset_task::get(&store, "task")?.revision, 0);
    assert!(asset_task::get(&store, "task")?.stages.is_empty());
    assert!(task_callback::inspect(&store, "task", Some("plan"))?["receipt"].is_null());
    assert_eq!(task_callback::inspect(&store, "task", None)?["revision"], 0);
    db.execute_batch("DROP TRIGGER reject_callback_event")?;
    assert_eq!(call(&mut store, &request)?["revision"], 1);
    Ok(())
}

#[test]
fn callback_input_limits_and_bounded_query_preserve_full_receipt_history() -> Result<()> {
    let (_temp, mut store) = fixture()?;
    let mut oversized = report("large", 0);
    oversized["report"]["outputs"] = json!({"data":"x".repeat(65536)});
    assert!(call(&mut store, &oversized).is_err());
    for n in 0..21 {
        call(&mut store, &report(&format!("report-{n}"), n))?;
    }
    let state = task_callback::inspect(&store, "task", None)?;
    assert_eq!(state["receipts"].as_array().unwrap().len(), 20);
    assert_eq!(state["receipts"][0]["requestId"], "report-20");
    assert!(state["receipts"][0].get("request").is_none());
    assert!(state["receipts"][0].get("response").is_none());
    assert!(task_callback::inspect(&store, "task", Some("report-0"))?["receipt"].is_object());
    assert!(task_callback::inspect(&store, "task", Some("missing"))?["receipt"].is_null());
    let mut long = report("long-summary", 21);
    long["report"]["summary"] = json!("z".repeat(1000));
    call(&mut store, &long)?;
    let state = task_callback::inspect(&store, "task", None)?;
    assert_eq!(state["receipts"][0]["summary"].as_str().unwrap().len(), 200);
    assert_eq!(
        task_callback::inspect(&store, "task", Some("long-summary"))?["receipt"]["request"],
        long
    );
    Ok(())
}
