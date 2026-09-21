#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_delivery_files, asset_task, asset_work, store::Store, task_callback};
use delivery_support::Fixture;
use serde_json::{json, Value};

#[tokio::test]
async fn host_restart_recovers_running_work_without_a_blender_session() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Brief").await?;
    let attempt = f.begin(&id, "").await?;
    assert!(f.state()?.session_id.is_none());
    let old_revision = f.state()?.revision;
    {
        let reopened = Store::open(f.temp.path())?;
        asset_task::recover_all(&reopened)?;
        let view = task_callback::inspect(&reopened, "task", None)?;
        assert_eq!(
            view["asset"]["work"]["attempts"][0]["status"],
            "interrupted"
        );
    }
    assert!(f.state()?.revision > old_revision);
    assert!(f.state()?.work.attempts[0].ended_at.is_some());
    f.resume("turn-2")?;
    assert!(f.finish(&attempt, "completed").await.is_err());
    assert!(f.begin(&id, "").await.is_err());
    let next = f
        .begin(&id, "Inspected disk; no scene existed; continuing brief")
        .await?;
    assert_ne!(attempt, next);
    f.finish(&next, "completed").await?;
    assert_eq!(f.state()?.work.attempts[1].turn_id, "turn-2");
    Ok(())
}

#[tokio::test]
async fn turn_change_retires_attempt_and_rejects_cross_turn_finish() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Brief").await?;
    let first = f.begin(&id, "").await?;
    f.resume("turn-2")?;
    assert!(f.finish(&first, "completed").await.is_err());
    asset_work::end_turn(&f.store.lock().unwrap(), "task")?;
    let revision = f.state()?.revision;
    asset_work::end_turn(&f.store.lock().unwrap(), "task")?;
    assert_eq!(f.state()?.revision, revision);
    let second = f
        .begin(&id, "Inspected saved work after lost connection")
        .await?;
    asset_task::mark_uncertain(&f.store.lock().unwrap(), "task", "Stopped by user")?;
    assert!(f.finish(&second, "completed").await.is_err());
    assert!(f
        .state()?
        .work
        .attempts
        .iter()
        .all(|a| a.status == "interrupted"));
    Ok(())
}

#[tokio::test]
async fn reject_and_reopen_preserve_history_but_require_new_attempts_and_inputs() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let design = f.create("Design").await?;
    let first = f.begin(&design, "").await?;
    f.finish(&first, "completed").await?;
    let rejected = f.submit().await?;
    f.decide(f.decision(&rejected, "reject")?)?;
    assert_eq!(f.state()?.work.subtasks[0].status, "stale");
    f.resume("turn-2")?;
    assert!(f.submit().await.is_err());
    assert!(f.begin(&design, "").await.is_err());
    let revised = f
        .begin(&design, "Read rejection; inspected existing brief")
        .await?;
    f.finish(&revised, "completed").await?;
    let approved = f.submit().await?;
    f.decide(f.decision(&approved, "approve")?)?;
    f.resume("turn-3")?;
    let model = f.create("Model").await?;
    let model_attempt = f.begin(&model, "").await?;
    f.finish(&model_attempt, "completed").await?;
    let model_candidate = f.submit().await?;
    f.decide(f.decision(&model_candidate, "approve")?)?;
    let mut task = f.task()?;
    task["status"] = json!("interrupted");
    f.store.lock().unwrap().put("task", "task", &task)?;
    let history = serde_json::to_value(&f.state()?.work.attempts)?;
    f.decide(f.decision(&approved, "reopen")?)?;
    assert!(f.state()?.work.subtasks.iter().all(|s| s.status == "stale"));
    assert_eq!(serde_json::to_value(&f.state()?.work.attempts)?, history);
    let old = asset_delivery_files::get(&f.store.lock().unwrap(), "task", &rejected)?;
    assert_eq!(old.attempt_ids, vec![first]);
    f.resume("turn-4")?;
    let rebuilt = f
        .begin(&design, "Read owner rework; inspected brief")
        .await?;
    f.finish(&rebuilt, "completed").await?;
    let new_design = f.submit().await?;
    f.decide(f.decision(&new_design, "approve")?)?;
    f.resume("turn-5")?;
    assert!(f.submit().await.is_err());
    let next_model = f.begin(&model, "Inspected changed upstream design").await?;
    let state = f.state()?;
    assert_eq!(
        state.work.attempts.last().unwrap().input_candidates,
        vec![new_design]
    );
    assert_eq!(state.work.attempts[2].input_candidates, vec![approved]);
    assert_ne!(model_attempt, next_model);
    Ok(())
}

#[tokio::test]
async fn pre_work_states_and_candidates_remain_readable() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let mut old = serde_json::to_value(f.state()?)?;
    old.as_object_mut().unwrap().remove("work");
    f.store.lock().unwrap().put("asset-task", "task", &old)?;
    assert!(f.state()?.work.subtasks.is_empty());
    let id = f.submit().await?;
    let kind = asset_delivery_files::kind("task");
    let mut candidate: Value = f.store.lock().unwrap().get(&kind, &id)?.unwrap();
    candidate.as_object_mut().unwrap().remove("attemptIds");
    f.store.lock().unwrap().put(&kind, &id, &candidate)?;
    assert!(
        asset_delivery_files::get(&f.store.lock().unwrap(), "task", &id)?
            .attempt_ids
            .is_empty()
    );
    f.decide(f.decision(&id, "approve")?)?;
    Ok(())
}
