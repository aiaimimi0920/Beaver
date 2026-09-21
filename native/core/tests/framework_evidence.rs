#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/framework.rs"]
mod framework_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_delivery_files as artifacts, call_log::Activity, framework_evidence};
use delivery_support::Fixture;
use serde_json::{json, Value};

#[tokio::test]
async fn traces_keep_start_attribution_and_do_not_invent_missing_evidence() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let first = f.create("First attempt").await?;
    let first = f.begin(&first, "").await?;
    let mut activity = Activity::new(f.store.clone(), &f.task()?);
    let item = json!({"id":"paired","type":"mcpToolCall","server":"blender","tool":"execute_code","arguments":{"code":"private-source"}});
    activity.item_at(&item, false, Some(("thread", "turn")))?;
    f.finish(&first, "completed").await?;
    let second = f.create("Second attempt").await?;
    let second = f.begin(&second, "").await?;
    activity.item_at(&item, true, Some(("thread", "turn")))?;
    activity.item_at(&json!({"id":"gap","type":"dynamicToolCall","tool":"beaver_task","result":"private-result"}), true, Some(("thread", "turn")))?;
    activity.item_at(
        &json!({"id":"unfinished","type":"commandExecution"}),
        false,
        Some(("thread", "turn")),
    )?;
    let db = f.store.lock().unwrap();
    // Diagnostic pruning must not remove durable workflow evidence.
    rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?
        .execute("DELETE FROM calls", [])?;
    framework_evidence::recover(&db)?;
    let traces = db.list::<Value>("framework-trace/task")?;
    assert_eq!(traces.len(), 3);
    let paired = traces.iter().find(|t| t["itemId"] == "paired").unwrap();
    assert_eq!(paired["context"]["attemptId"], first);
    assert_eq!(paired["status"], "succeeded");
    assert_eq!(paired["skillEvidence"], "not-observed");
    assert!(paired["gap"].is_null());
    let gap = traces.iter().find(|t| t["itemId"] == "gap").unwrap();
    assert!(gap["context"]["attemptId"].is_null());
    assert!(gap["input"].is_null());
    assert!(gap["gap"]
        .as_str()
        .unwrap()
        .contains("completion-without-start"));
    let unfinished = traces.iter().find(|t| t["itemId"] == "unfinished").unwrap();
    assert_eq!(unfinished["context"]["attemptId"], second);
    assert_eq!(unfinished["status"], "interrupted");
    let encoded = serde_json::to_string(&traces)?;
    assert!(!encoded.contains("private-source"));
    assert!(!encoded.contains("private-result"));
    drop(db);
    drop(activity);
    Ok(())
}

#[tokio::test]
async fn checks_and_input_exports_preserve_distinct_historical_bytes_at_the_same_path() -> Result<()>
{
    let f = Fixture::new()?;
    f.plan().await?;
    let mut attempts = Vec::new();
    for version in ["original model", "edited model"] {
        std::fs::write(f.workspace.join("model.blend"), version)?;
        let id = f.create(version).await?;
        let result = f.work(json!({"action":"begin","subtaskId":id,"inputs":{},"inputFiles":[{"path":"model.blend","role":"source"}]})).await?;
        let attempt = result["attemptId"].as_str().unwrap().to_owned();
        f.finish(&attempt, "completed").await?;
        attempts.push(attempt);
    }
    let candidate = f.submit().await?;
    let checked = f.check(&candidate).await?;
    assert_eq!(checked["status"], "succeeded", "{checked}");
    let exports = checked["result"]["value"]["attemptInputs"]
        .as_array()
        .unwrap();
    assert_eq!(exports.len(), 2);
    assert_ne!(exports[0]["path"], exports[1]["path"]);
    for (attempt, expected) in attempts.iter().zip(["original model", "edited model"]) {
        let export = exports.iter().find(|e| e["attemptId"] == *attempt).unwrap();
        let path = std::path::Path::new(export["path"].as_str().unwrap());
        assert_eq!(std::fs::read_to_string(path.join("model.blend"))?, expected);
        let op = f
            .start(false, json!({"kind":"inputExport","attemptId":attempt}))
            .await?;
        let completed = f.terminal(op["id"].as_str().unwrap()).await?;
        assert_eq!(completed["status"], "succeeded", "{completed}");
        let value = &completed["result"]["value"];
        assert_eq!(value["source"], "frozen-attempt-inputs");
        assert_eq!(
            std::fs::read_to_string(
                std::path::Path::new(value["path"].as_str().unwrap()).join("model.blend")
            )?,
            expected
        );
    }
    Ok(())
}

#[tokio::test]
async fn final_dependency_closure_allows_approved_overwrites_but_rejects_unreviewed_drift(
) -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    std::fs::write(f.workspace.join("texture.png"), "original texture")?;
    let subtask = f.create("Use texture").await?;
    let result = f.work(json!({"action":"begin","subtaskId":subtask,"inputs":{},"inputFiles":[{"path":"texture.png","role":"dependency"}]})).await?;
    f.finish(result["attemptId"].as_str().unwrap(), "completed")
        .await?;
    let candidate = f.submit().await?;
    f.decide(f.decision(&candidate, "approve")?)?;
    f.resume("second-turn")?;
    let subtask = f.create("Update texture").await?;
    let result = f.work(json!({"action":"begin","subtaskId":subtask,"inputs":{},"inputFiles":[{"path":"texture.png","role":"source"}]})).await?;
    std::fs::write(f.workspace.join("texture.png"), "approved replacement")?;
    f.finish(result["attemptId"].as_str().unwrap(), "completed")
        .await?;
    let result = f.call(f.request("replacement", &["texture.png"])?).await?;
    f.decide(f.decision(result["candidateId"].as_str().unwrap(), "approve")?)?;
    let task = f.task()?;
    artifacts::verify_final(&f.store.lock().unwrap(), &f.files(), &task, &[])?;
    std::fs::write(f.workspace.join("texture.png"), "unreviewed drift")?;
    assert!(artifacts::verify_final(&f.store.lock().unwrap(), &f.files(), &task, &[]).is_err());
    Ok(())
}

#[tokio::test]
async fn stale_check_keeps_evidence_without_publishing_a_passing_gate() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    f.configure(json!([]), json!([framework_support::rule("slow")?]), false)
        .await?;
    let op = f
        .start(false, json!({"kind":"check","candidateId":candidate}))
        .await?;
    let mut state = f.state()?;
    state.revision += 1;
    f.store.lock().unwrap().put("asset-task", "task", &state)?;
    let completed = f.terminal(op["id"].as_str().unwrap()).await?;
    assert_eq!(completed["status"], "stale", "{completed}");
    assert_eq!(completed["result"]["value"]["passed"], true);
    assert_eq!(
        f.store
            .lock()
            .unwrap()
            .get::<Value>("framework-check/task", &candidate)?
            .unwrap()["passed"],
        false
    );
    assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    Ok(())
}
