use super::fixture::Fixture;
use anyhow::Result;
use beaver_core::{
    asset_stages, asset_task, asset_withdrawal, executor::Outcome, files::Files, task_finish,
};
use serde_json::{json, Value};
use std::{fs, path::Path, sync::atomic::AtomicBool};

#[test]
fn receipt_before_finalization_blocks_merge_and_preserves_workspace() -> Result<()> {
    let mut fixture = Fixture::new()?;
    fixture.finishable()?;
    let workspace = Path::new(fixture.task["workspace"].as_str().unwrap()).to_path_buf();
    fs::write(workspace.join("original.txt"), "unverified modification")?;
    let input = fixture.input("arriving", "now")?;
    fixture.submit(input.clone())?;
    let finished = task_finish::finish(
        &mut fixture.store,
        &Files::new(fixture.root.clone()),
        &fixture.id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(finished["status"], "queued");
    assert_eq!(finished["assetResume"], true);
    assert_eq!(
        fs::read_to_string(fixture.project.join("original.txt"))?,
        "original project"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("original.txt"))?,
        "unverified modification"
    );
    assert_eq!(fixture.submit(input)?.task_id, fixture.id);
    assert_eq!(fixture.store.list::<Value>("task")?.len(), 1);
    Ok(())
}

#[test]
fn finalization_before_receipt_creates_one_isolated_followup_with_crash_repair() -> Result<()> {
    let mut fixture = Fixture::new()?;
    fixture.finishable()?;
    let finished = task_finish::finish(
        &mut fixture.store,
        &Files::new(fixture.root.clone()),
        &fixture.id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(finished["status"], "completed");
    fs::write(
        fixture.project.join("other-task.txt"),
        "new concurrent result",
    )?;
    let input = fixture.input("late", "now")?;
    let receipt = fixture.submit(input.clone())?;
    assert_eq!(receipt.status, "forwarded");
    assert_ne!(receipt.task_id, fixture.id);
    let destination: Value = fixture.store.get("task", &receipt.task_id)?.unwrap();
    assert_eq!(destination["relation"], "followup");
    assert_eq!(destination["parentTaskId"], fixture.id);
    assert_ne!(destination["workspace"], finished["workspace"]);
    assert_eq!(destination["baseline"]["other-task.txt"].is_string(), true);
    assert_eq!(
        fs::read_to_string(
            Path::new(destination["workspace"].as_str().unwrap()).join("other-task.txt")
        )?,
        "new concurrent result"
    );
    assert_eq!(
        fixture.store.get::<Value>("task", &fixture.id)?.unwrap(),
        finished
    );
    let mut source = fixture.state()?;
    source.feedback.clear();
    asset_task::save(&fixture.store, &source)?;
    fixture.store.remove("asset-task", &receipt.task_id)?;
    let repaired = fixture.submit(input.clone())?;
    assert_eq!(repaired.task_id, receipt.task_id);
    assert_eq!(fixture.submit(input)?.task_id, receipt.task_id);
    assert_eq!(fixture.store.list::<Value>("task")?.len(), 2);
    assert_eq!(fixture.state()?.feedback.len(), 1);
    assert_eq!(
        asset_task::get(&fixture.store, &receipt.task_id)?.feedback[0].status,
        "received"
    );
    let mut colliding = fixture.input("late", "now")?;
    colliding.id = receipt.task_id;
    assert!(fixture.submit(colliding).is_err());
    Ok(())
}

#[test]
fn cancelling_late_feedback_repairs_missing_source_and_pauses_unstarted_followup() -> Result<()> {
    let mut fixture = Fixture::new()?;
    fixture.finishable()?;
    fixture.task["status"] = json!("completed");
    fixture.store.put("task", &fixture.id, &fixture.task)?;
    let input = fixture.input("late-cancel", "now")?;
    let receipt = fixture.submit(input.clone())?;
    let mut source = fixture.state()?;
    source.feedback.clear();
    asset_task::save(&fixture.store, &source)?;
    fixture.store.remove("asset-task", &receipt.task_id)?;
    asset_withdrawal::cancel(&fixture.store, &fixture.id, "late-cancel")?;
    assert_eq!(
        asset_task::get(&fixture.store, &receipt.task_id)?.feedback[0].status,
        "cancelled"
    );
    assert_eq!(
        fixture
            .store
            .get::<Value>("task", &receipt.task_id)?
            .unwrap()["status"],
        "interrupted"
    );
    assert!(fixture.submit(input).is_err());
    assert_eq!(fixture.store.list::<Value>("task")?.len(), 2);
    Ok(())
}

#[test]
fn deferred_round_advances_before_a_new_turn_and_unverified_edits_never_deliver() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("deferred", "afterRound")?;
    fixture.submit(input)?;
    let mut state = fixture.state()?;
    state.stages = vec![super::fixture::stage("body", "completed", &[])];
    asset_stages::complete_round(&mut state)?;
    asset_task::save(&fixture.store, &state)?;
    let queued = task_finish::finish(
        &mut fixture.store,
        &Files::new(fixture.root.clone()),
        &fixture.id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(queued["status"], "queued");
    assert_eq!(fixture.state()?.round, 2);
    fixture.task["status"] = json!("running");
    fixture.store.put("task", &fixture.id, &fixture.task)?;
    state = fixture.delivered("deferred")?;
    state.feedback[0].status = "executing".into();
    asset_task::save(&fixture.store, &state)?;
    let failed = task_finish::finish(
        &mut fixture.store,
        &Files::new(fixture.root.clone()),
        &fixture.id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(failed["status"], "failed");
    assert_eq!(fixture.state()?.feedback[0].status, "pendingVerification");
    assert_eq!(
        fs::read_to_string(fixture.project.join("original.txt"))?,
        "original project"
    );
    Ok(())
}
