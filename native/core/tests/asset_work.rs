#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_delivery_files, asset_task, task_callback};
use delivery_support::Fixture;
use serde_json::{json, Value};

#[tokio::test]
async fn edits_are_current_stage_scoped_and_retries_keep_the_assigned_identity() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let request =
        f.work_request(json!({"action":"create","definition":support::definition("Brief")}))?;
    let created = f.call(request.clone()).await?;
    assert_eq!(created, f.call(request.clone()).await?);
    let id = created["subtaskId"].as_str().unwrap();
    let mut conflict = request.clone();
    conflict["change"]["definition"]["title"] = json!("Different");
    assert!(f.call(conflict).await.is_err());
    let mut stale = request;
    stale["requestId"] = json!("stale");
    assert!(f.call(stale).await.is_err());
    let mut wrong_stage =
        f.work_request(json!({"action":"create","definition":support::definition("Future")}))?;
    wrong_stage["stageId"] = json!("model");
    assert!(f.call(wrong_stage).await.is_err());
    let mut wrong_asset =
        f.work_request(json!({"action":"cancel","subtaskId":id,"reason":"Unneeded"}))?;
    wrong_asset["assetRevision"] = json!(0);
    assert!(f.call(wrong_asset).await.is_err());
    f.work(json!({"action":"revise","subtaskId":id,"definition":support::definition("Updated")}))
        .await?;
    assert!(f
        .work(json!({"action":"cancel","subtaskId":id,"reason":" "}))
        .await
        .is_err());
    f.work(json!({"action":"cancel","subtaskId":id,"reason":"Covered by another deliverable"}))
        .await?;
    let state = f.state()?;
    assert_eq!(state.work.subtasks.len(), 1);
    assert_eq!(state.work.subtasks[0].definition.title, "Updated");
    assert_eq!(state.work.subtasks[0].status, "cancelled");
    assert!(f.begin(id, "").await.is_err());
    assert!(state.work.attempts.is_empty());
    assert_eq!(f.store.lock().unwrap().list::<Value>("task")?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn serial_attempts_gate_submission_and_bind_frozen_candidate_without_approval() -> Result<()>
{
    let f = Fixture::new()?;
    f.plan().await?;
    let first = f.create("Brief").await?;
    let second = f.create("Optional reference").await?;
    assert!(f.begin(&second, "").await.is_err());
    assert!(f.submit().await.is_err());
    let attempt = f.begin(&first, "").await?;
    assert!(f.begin(&second, "").await.is_err());
    assert!(f
        .work(json!({"action":"cancel","subtaskId":first,"reason":"Skip"}))
        .await
        .is_err());
    f.finish(&attempt, "completed").await?;
    assert!(f.submit().await.is_err());
    f.work(json!({"action":"cancel","subtaskId":second,"reason":"Not required for this design"}))
        .await?;
    let candidate = f.submit().await?;
    let state = f.state()?;
    let flow = state.delivery.unwrap();
    assert!(flow.approved.is_empty());
    assert_eq!(flow.pending.as_deref(), Some(candidate.as_str()));
    assert_eq!(f.task()?["status"], "awaitingInput");
    let saved = asset_delivery_files::get(&f.store.lock().unwrap(), "task", &candidate)?;
    assert_eq!(saved.attempt_ids, vec![attempt]);
    assert!(saved.files.contains_key("docs/design.md"));
    assert!(f.create("Forbidden while reviewing").await.is_err());
    f.decide(f.decision(&candidate, "approve")?)?;
    assert_eq!(f.state()?.stages[1].status, "running");
    Ok(())
}

#[tokio::test]
async fn failed_retry_snapshots_definitions_and_keeps_prior_evidence() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Original").await?;
    let mut state = f.state()?;
    state.session_id = Some("scene-session".into());
    state.checkpoint = Some("before.blend".into());
    asset_task::save(&f.store.lock().unwrap(), &state)?;
    let first = f.begin(&id, "").await?;
    f.finish(&first, "failed").await?;
    let old = serde_json::to_value(&f.state()?.work.attempts[0])?;
    f.work(json!({"action":"revise","subtaskId":id,"definition":support::definition("Revised")}))
        .await?;
    assert!(f.begin(&id, "").await.is_err());
    let second = f
        .begin(
            &id,
            "Inspected saved files; previous operation failed before writing",
        )
        .await?;
    assert_ne!(first, second);
    let mut state = f.state()?;
    state.checkpoint = Some("after.blend".into());
    asset_task::save(&f.store.lock().unwrap(), &state)?;
    let finish_request = f.work_request(json!({"action":"finish","attemptId":second,"outcome":"completed","summary":"Saved","outputs":{}}))?;
    let finished = f.call(finish_request.clone()).await?;
    assert_eq!(finished, f.call(finish_request).await?);
    let work = f.state()?.work;
    assert_eq!(serde_json::to_value(&work.attempts[0])?, old);
    assert_eq!(work.attempts[1].definition.title, "Revised");
    assert_eq!(
        work.attempts[1].session_id.as_deref(),
        Some("scene-session")
    );
    assert_eq!(work.attempts[1].checkpoint.as_deref(), Some("before.blend"));
    assert_eq!(
        work.attempts[1].end_checkpoint.as_deref(),
        Some("after.blend")
    );
    assert!(f.finish(&first, "completed").await.is_err());
    Ok(())
}

#[tokio::test]
async fn receipt_failure_rolls_back_work_and_both_revisions() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let before = serde_json::to_value(f.state()?)?;
    let revision =
        task_callback::inspect(&f.store.lock().unwrap(), "task", None)?["revision"].clone();
    let db = rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?;
    db.execute_batch("CREATE TRIGGER reject_work_receipt BEFORE INSERT ON entities WHEN NEW.kind='task-callback/task' BEGIN SELECT RAISE(ABORT, 'receipt failure'); END;")?;
    let request =
        f.work_request(json!({"action":"create","definition":support::definition("Atomic")}))?;
    assert!(f.call(request.clone()).await.is_err());
    assert_eq!(serde_json::to_value(f.state()?)?, before);
    let view = task_callback::inspect(
        &f.store.lock().unwrap(),
        "task",
        request["requestId"].as_str(),
    )?;
    assert_eq!(view["revision"], revision);
    assert!(view["receipt"].is_null());
    db.execute_batch("DROP TRIGGER reject_work_receipt;")?;
    f.call(request).await?;
    assert_eq!(f.state()?.work.subtasks.len(), 1);
    Ok(())
}
