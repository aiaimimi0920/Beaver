#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_delivery_files, task_callback};
use delivery_support::Fixture;
use serde_json::json;

async fn begin(f: &Fixture, id: &str, note: &str) -> Result<String> {
    let result = f
        .work(
            json!({"action":"begin","subtaskId":id,"inputs":{},"recoveryNote":note,
        "inputFiles":[{"path":"ref.png","role":"dependency"}]}),
        )
        .await?;
    Ok(result["attemptId"].as_str().unwrap().into())
}

#[tokio::test]
async fn dependency_drift_blocks_success_but_failure_and_recapture_recover() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Reference").await?;
    std::fs::write(f.workspace.join("ref.png"), b"v1")?;
    let first = begin(&f, &id, "").await?;
    let before = serde_json::to_value(f.state()?)?;
    std::fs::write(f.workspace.join("ref.png"), b"v2")?;
    let request = f.work_request(json!({"action":"finish","attemptId":first,"outcome":"completed","summary":"Done","outputs":{}}))?;
    assert!(format!("{:#}", f.call(request.clone()).await.unwrap_err()).contains("dependency"));
    assert_eq!(serde_json::to_value(f.state()?)?, before);
    assert!(task_callback::inspect(
        &f.store.lock().unwrap(),
        "task",
        request["requestId"].as_str()
    )?["receipt"]
        .is_null());
    f.finish(&first, "failed").await?;
    let next = begin(&f, &id, "Inspected changed reference; use v2").await?;
    f.finish(&next, "completed").await?;
    let state = f.state()?;
    let old = &state.work.attempts[0].input_files.as_ref().unwrap()[0];
    let new = &state.work.attempts[1].input_files.as_ref().unwrap()[0];
    assert_ne!(old.sha256, new.sha256);
    assert_eq!(std::fs::read(f.files().blob(&old.sha256)?)?, b"v1");
    let candidate = f.submit().await?;
    f.decide(f.decision(&candidate, "approve")?)?;
    Ok(())
}

#[tokio::test]
async fn reopen_invalidates_later_work_and_unblocks_changed_inputs_before_submission() -> Result<()>
{
    let f = Fixture::new()?;
    f.plan().await?;
    let first = f.create("Reference").await?;
    let second = f.create("Model").await?;
    let cancelled = f.create("Optional").await?;
    f.work(json!({"action":"cancel","subtaskId":cancelled,"reason":"Not needed"}))
        .await?;
    std::fs::write(f.workspace.join("ref.png"), b"v1")?;
    let a = begin(&f, &first, "").await?;
    f.finish(&a, "completed").await?;
    let b = f.begin(&second, "").await?;
    assert!(f
        .work(json!({"action":"reopen","subtaskId":first,"reason":"Changed"}))
        .await
        .is_err());
    f.finish(&b, "completed").await?;
    let history = serde_json::to_value(f.state()?.work.attempts)?;
    std::fs::write(f.workspace.join("ref.png"), b"v2")?;
    assert!(f.submit().await.is_err());
    assert!(f
        .work(json!({"action":"reopen","subtaskId":first,"reason":" "}))
        .await
        .is_err());
    let reopen = f.work_request(
        json!({"action":"reopen","subtaskId":first,"reason":"Reference replaced; redo both steps"}),
    )?;
    let response = f.call(reopen.clone()).await?;
    assert_eq!(f.call(reopen).await?, response);
    let state = f.state()?;
    assert_eq!(serde_json::to_value(state.work.attempts)?, history);
    assert_eq!(
        state
            .work
            .subtasks
            .iter()
            .map(|s| s.status.as_str())
            .collect::<Vec<_>>(),
        vec!["stale", "stale", "cancelled"]
    );
    assert!(f.begin(&second, "Inspect").await.is_err());
    let next = begin(&f, &first, "Inspected v2").await?;
    f.finish(&next, "completed").await?;
    assert!(f.submit().await.is_err());
    let downstream = f.begin(&second, "Rebuild against v2").await?;
    f.finish(&downstream, "completed").await?;
    let candidate = f.submit().await?;
    assert!(f
        .work(json!({"action":"reopen","subtaskId":first,"reason":"Bypass owner"}))
        .await
        .is_err());
    let saved = asset_delivery_files::get(&f.store.lock().unwrap(), "task", &candidate)?;
    assert_eq!(saved.attempt_ids, vec![next, downstream]);
    f.decide(f.decision(&candidate, "approve")?)?;
    f.resume("next-turn")?;
    assert!(f
        .work(json!({"action":"reopen","subtaskId":first,"reason":"Old stage"}))
        .await
        .is_err());
    Ok(())
}

#[tokio::test]
async fn approval_rechecks_dependency_and_rejection_remains_available() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Reference").await?;
    std::fs::write(f.workspace.join("ref.png"), b"v1")?;
    let attempt = begin(&f, &id, "").await?;
    f.finish(&attempt, "completed").await?;
    let candidate = f.submit().await?;
    let before = serde_json::to_value(f.state()?)?;
    std::fs::remove_file(f.workspace.join("ref.png"))?;
    assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    assert_eq!(serde_json::to_value(f.state()?)?, before);
    f.decide(f.decision(&candidate, "reject")?)?;
    assert_eq!(f.state()?.work.subtasks[0].status, "stale");
    Ok(())
}

#[tokio::test]
async fn frozen_source_corruption_blocks_success_even_when_live_source_is_valid() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Source").await?;
    std::fs::write(f.workspace.join("model.blend"), b"saved model")?;
    let result = f
        .work(json!({"action":"begin","subtaskId":id,"inputs":{},
        "inputFiles":[{"path":"model.blend","role":"source"}]}))
        .await?;
    let state = f.state()?;
    let hash = &state.work.attempts[0].input_files.as_ref().unwrap()[0].sha256;
    std::fs::write(f.files().blob(hash)?, b"corrupted")?;
    assert!(f
        .finish(result["attemptId"].as_str().unwrap(), "completed")
        .await
        .is_err());
    f.finish(result["attemptId"].as_str().unwrap(), "failed")
        .await?;
    Ok(())
}

#[tokio::test]
async fn different_versions_of_one_dependency_cannot_overwrite_earlier_evidence() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let first = f.create("First").await?;
    let second = f.create("Second").await?;
    std::fs::write(f.workspace.join("ref.png"), b"v1")?;
    let a = begin(&f, &first, "").await?;
    f.finish(&a, "completed").await?;
    std::fs::write(f.workspace.join("ref.png"), b"v2")?;
    let b = begin(&f, &second, "").await?;
    f.finish(&b, "completed").await?;
    assert!(f.submit().await.is_err());
    Ok(())
}
