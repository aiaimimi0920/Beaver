#[path = "support/asset_delivery.rs"]
mod support;
use anyhow::Result;
use beaver_core::{
    asset_delivery_files as artifacts, asset_delivery_review as review, asset_stages, asset_task,
    executor::Outcome, files::Files, store::Store, task_actions, task_callback, task_finish,
};
use serde_json::{json, Value};
use std::{fs, sync::atomic::AtomicBool};
use support::Fixture;

#[tokio::test]
async fn linear_delivery_freezes_bytes_pauses_and_requires_owner_approval() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "approved design")?;
    let mut stages = f.state()?.stages;
    stages[0].status = "completed".into();
    stages[0].evidence = "Model says done".into();
    assert!(asset_stages::update(&mut f.state()?, 1, stages).is_err());
    assert!(asset_stages::complete_round(&mut f.state()?).is_err());
    let request = f.request("design-submit", &["design.md"])?;
    let submitted = f.call(request.clone()).await?;
    let candidate = submitted["candidateId"].as_str().unwrap();
    assert_eq!(submitted["paused"], true);
    assert_eq!(f.call(request.clone()).await?, submitted);
    assert_eq!(f.task()?["status"], "awaitingInput");
    assert!(
        task_actions::continue_task(&mut f.store.lock().unwrap(), "task", "skip", false).is_err()
    );
    let mut altered = request.clone();
    altered["summary"] = json!("changed");
    assert!(f.call(altered).await.is_err());
    let saved = artifacts::get(&f.store.lock().unwrap(), "task", candidate)?;
    fs::write(f.workspace.join("design.md"), "unsubmitted changes")?;
    assert_eq!(
        artifacts::read(&f.files(), &saved, "design.md")?,
        b"approved design"
    );
    assert!(f.decide(f.decision(candidate, "approve")?).is_err());
    fs::write(f.workspace.join("design.md"), "approved design")?;
    let decision = f.decision(candidate, "approve")?;
    let approved = f.decide(decision.clone())?;
    assert_eq!(
        review::duplicate(&f.store.lock().unwrap(), &decision)?,
        Some(approved)
    );
    let mut conflict = decision;
    conflict.note = "different".into();
    assert!(review::duplicate(&f.store.lock().unwrap(), &conflict).is_err());
    assert_eq!(f.state()?.stages[1].status, "running");
    f.resume("turn-2")?;
    assert!(f.call(request).await.is_err());
    fs::write(f.workspace.join("model.txt"), "model output")?;
    let mut next = f.request("model-submit", &["model.txt"])?;
    next["inputCandidates"] = json!([]);
    assert!(f.call(next).await.is_err());
    let second = f.call(f.request("model-submit", &["model.txt"])?).await?;
    f.decide(f.decision(second["candidateId"].as_str().unwrap(), "approve")?)?;
    let mut state = f.state()?;
    asset_stages::complete_round(&mut state)?;
    asset_task::save(&f.store.lock().unwrap(), &state)?;
    let changes = Files::changes(&Default::default(), &f.files().capture(&f.workspace)?);
    let task = f.task()?;
    artifacts::verify_final(&f.store.lock().unwrap(), &f.files(), &task, &changes)?;
    fs::write(f.workspace.join("unreviewed.txt"), "must not merge")?;
    let changes = Files::changes(&Default::default(), &f.files().capture(&f.workspace)?);
    assert!(
        artifacts::verify_final(&f.store.lock().unwrap(), &f.files(), &task, &changes).is_err()
    );
    fs::write(f.workspace.join("model.txt"), "drift")?;
    assert!(artifacts::verify_final(&f.store.lock().unwrap(), &f.files(), &task, &[]).is_err());
    Ok(())
}

#[tokio::test]
async fn rejection_and_upstream_rework_invalidate_inputs_but_preserve_candidates() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "v1")?;
    let first = f.call(f.request("one", &["design.md"])?).await?;
    let original = first["candidateId"].as_str().unwrap();
    f.decide(f.decision(original, "reject")?)?;
    f.resume("turn-2")?;
    fs::write(f.workspace.join("design.md"), "v2")?;
    let second = f.call(f.request("two", &["design.md"])?).await?;
    let approved = second["candidateId"].as_str().unwrap();
    f.decide(f.decision(approved, "approve")?)?;
    f.resume("turn-3")?;
    fs::write(f.workspace.join("model.txt"), "v2 model")?;
    let third = f.call(f.request("three", &["model.txt"])?).await?;
    f.decide(f.decision(third["candidateId"].as_str().unwrap(), "approve")?)?;
    let mut task = f.task()?;
    task["status"] = json!("interrupted");
    f.store.lock().unwrap().put("task", "task", &task)?;
    f.decide(f.decision(approved, "reopen")?)?;
    let state = f.state()?;
    let flow = state.delivery.unwrap();
    assert!(flow.approved.is_empty());
    assert_eq!(flow.history.len(), 3);
    assert_eq!(state.stages[0].status, "running");
    assert_eq!(state.stages[1].status, "pending");
    let candidate = artifacts::get(&f.store.lock().unwrap(), "task", original)?;
    assert_eq!(artifacts::read(&f.files(), &candidate, "design.md")?, b"v1");
    Ok(())
}

#[tokio::test]
async fn pending_review_survives_finish_restart_and_interruption_without_merge() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "v1")?;
    let submitted = f.call(f.request("pending", &["design.md"])?).await?;
    let finished = task_finish::finish(
        &mut f.store.lock().unwrap(),
        &f.files(),
        "task",
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(finished["status"], "awaitingInput");
    assert!(!f.temp.path().join("project/design.md").exists());
    f.store.lock().unwrap().recover_tasks()?;
    asset_task::recover_all(&f.store.lock().unwrap())?;
    let mut reopened = Store::open(f.temp.path())?;
    let state = asset_task::get(&reopened, "task")?;
    assert_eq!(
        state.delivery.unwrap().pending,
        submitted["candidateId"].as_str().map(str::to_owned)
    );
    let mut task: Value = reopened.get("task", "task")?.unwrap();
    task["status"] = json!("interrupted");
    reopened.put("task", "task", &task)?;
    assert!(task_actions::continue_task(&mut reopened, "task", "skip", true).is_err());
    assert!(task_callback::inspect(&reopened, "task", Some("pending"))?["receipt"].is_object());
    Ok(())
}
