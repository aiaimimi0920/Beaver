use super::{
    coverage, model::Run, repository, task_completion, task_control, test_support::Fixture,
};
use crate::{executor::Outcome, files::Files, task_actions, task_finish};
use anyhow::Result;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn delivery(fixture: &Fixture, automatic: bool) -> Result<PathBuf> {
    let baseline = fixture.files.capture(&fixture.project)?;
    let workspace = fixture.files.root().join("workspaces/t");
    fixture.files.restore_copy(&baseline, &workspace)?;
    fs::write(
        workspace.join("game.gd"),
        "extends Node\nvar value = 1\nvar ready = true\n",
    )?;
    let snapshot = fixture.files.capture(&workspace)?;
    let changes = Files::changes(&baseline, &snapshot);
    fixture.store.put(
        "task",
        "t",
        &json!({
            "id":"t","projectId":"p","title":"Show game state","prompt":"Implement game state",
            "status":"running","capability":"code","autoAccept":automatic,"validationVersion":1,
            "workspace":workspace,"baseline":baseline,"changes":changes,"validationPrepared":true,
            "turnId":"original-turn","report":"Delivered game state"
        }),
    )?;
    pass(fixture, snapshot)?;
    Ok(workspace)
}

fn pass(fixture: &Fixture, snapshot: crate::files::Snapshot) -> Result<String> {
    let mut run = repository::new_run(&fixture.store, "p", snapshot, None, Some("t".into()), None)?;
    fixture.passing_code(&mut run)?;
    let mut task: Value = repository::get(&fixture.store, "task", "t")?;
    task["codeValidation"] =
        json!({"status":"completed","runId":run.id,"snapshotId":run.snapshot_id});
    fixture.store.put("task", "t", &task)?;
    Ok(run.id)
}

fn finish(fixture: &mut Fixture) -> Result<Value> {
    task_finish::finish(
        &mut fixture.store,
        &fixture.files,
        "t",
        Outcome::Completed,
        &AtomicBool::new(false),
    )
}

#[test]
fn code_delivery_does_not_wait_for_visuals_and_preserves_manual_approval() -> Result<()> {
    for automatic in [true, false] {
        let mut fixture = Fixture::new()?;
        let flow = fixture.flow()?;
        let mut definition = flow.definition.clone();
        definition.task_ids = vec!["t".into()];
        repository::save_flow(&mut fixture.store, "p", definition, flow.revision)?;
        delivery(&fixture, automatic)?;
        let task = finish(&mut fixture)?;
        assert_eq!(task["status"], "completed");
        assert_eq!(task["accepted"] == true, automatic);
        assert!(fs::read_to_string(fixture.project.join("game.gd"))?.contains("ready = true"));
        let mut coverage: Value = repository::get(&fixture.store, "validationCoverage", "t")?;
        assert_eq!(coverage["status"], "pending");
        assert_eq!(fixture.store.list::<Run>("validationRun")?.len(), 1);
        if !automatic {
            coverage::refresh(&mut fixture.store, fixture.files.root(), &mut coverage)?;
            assert_eq!(coverage["status"], "pending");
            task_actions::accept(&mut fixture.store, "t")?;
        }
        let task: Value = repository::get(&fixture.store, "task", "t")?;
        assert_eq!(
            task["approvalSource"],
            if automatic { "automatic" } else { "user" }
        );
        coverage::refresh(&mut fixture.store, fixture.files.root(), &mut coverage)?;
        assert_eq!(coverage["status"], "queued");
        let id = coverage["runIds"][0].as_str().unwrap();
        let mut visual: Run = repository::get(&fixture.store, "validationRun", id)?;
        assert_eq!(visual.status, "queued");
        visual.status = "failed".into();
        visual.error = Some("Capture unavailable".into());
        fixture.save(&visual)?;
        coverage::refresh(&mut fixture.store, fixture.files.root(), &mut coverage)?;
        assert_eq!(coverage["status"], "failed");
        assert_eq!(repository::get::<Value>(&fixture.store, "task", "t")?, task);
    }
    Ok(())
}

#[test]
fn late_user_instructions_survive_validation_and_prevent_premature_merge() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let workspace = delivery(&fixture, true)?;
    let before = fs::read(fixture.project.join("game.gd"))?;
    for text in ["Keep the button visible", "Preserve the default value"] {
        task_control::steer(&mut fixture.store, "t", text)?;
    }
    let task = finish(&mut fixture)?;
    assert_eq!(task["status"], "queued");
    assert_eq!(task["validationOnly"], false);
    assert_eq!(task["validationPrepared"], false);
    assert!(task["turnId"].is_null());
    assert!(task["validationSteering"].is_null());
    assert!(task["prompt"]
        .as_str()
        .unwrap()
        .contains("Keep the button visible"));
    assert!(task["prompt"]
        .as_str()
        .unwrap()
        .contains("Preserve the default value"));
    assert_eq!(fs::read(fixture.project.join("game.gd"))?, before);
    assert!(fixture.store.list::<Value>("operation")?.is_empty());
    assert!(fixture
        .store
        .list::<Value>("validationCoverage")?
        .is_empty());
    let user_events: Vec<_> = fixture
        .store
        .events("t")?
        .into_iter()
        .filter(|event| event.kind == "user")
        .map(|event| event.text)
        .collect();
    assert_eq!(
        user_events,
        ["Keep the button visible", "Preserve the default value"]
    );

    fs::write(
        workspace.join("game.gd"),
        "extends Node\nvar value = 1\nvar button_visible = true\n",
    )?;
    let mut task = task;
    task["status"] = json!("running");
    fixture.store.put("task", "t", &task)?;
    pass(&fixture, fixture.files.capture(&workspace)?)?;
    let resumed = finish(&mut fixture)?;
    assert_eq!(resumed["status"], "completed");
    assert_eq!(resumed["accepted"], true);
    assert!(fs::read_to_string(fixture.project.join("game.gd"))?.contains("button_visible"));
    Ok(())
}

#[test]
fn concurrent_project_changes_require_fresh_code_before_recorded_output_merges() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let workspace = delivery(&fixture, true)?;
    let initial: Value = repository::get(&fixture.store, "task", "t")?;
    fs::write(
        fixture.project.join("player.gd"),
        "extends Node\nvar health = 5\n",
    )?;
    let original = fs::read(fixture.project.join("game.gd"))?;
    let mut task = finish(&mut fixture)?;
    assert_eq!(task["status"], "queued");
    assert_eq!(task["validationOnly"], true);
    assert_eq!(fs::read(fixture.project.join("game.gd"))?, original);
    assert!(fixture.store.list::<Value>("operation")?.is_empty());
    let mut snapshot = fixture.files.capture(&fixture.project)?;
    snapshot.insert(
        "game.gd".into(),
        fixture.files.capture(&workspace)?["game.gd"].clone(),
    );
    task["status"] = json!("running");
    fixture.store.put("task", "t", &task)?;
    let fresh_run = pass(&fixture, snapshot)?;
    assert_ne!(initial["codeValidation"]["runId"], fresh_run);
    fs::write(workspace.join("unrecorded.gd"), "extends Node\n")?;
    let task = finish(&mut fixture)?;
    assert_eq!(task["status"], "completed");
    assert_eq!(task["accepted"], true);
    assert_eq!(task["codeValidation"]["runId"], fresh_run);
    assert!(fs::read_to_string(fixture.project.join("game.gd"))?.contains("ready = true"));
    assert!(fs::read_to_string(fixture.project.join("player.gd"))?.contains("health = 5"));
    assert!(!fixture.project.join("unrecorded.gd").exists());
    Ok(())
}

#[test]
fn green_code_receipt_never_overwrites_a_human_conflict() -> Result<()> {
    let mut fixture = Fixture::new()?;
    delivery(&fixture, true)?;
    let human = "extends Node\nvar value = 8\n";
    fs::write(fixture.project.join("game.gd"), human)?;
    let task = finish(&mut fixture)?;
    assert_eq!(task["status"], "conflict");
    assert_eq!(task["conflicts"], json!(["game.gd"]));
    assert_ne!(task["accepted"], true);
    assert_eq!(fs::read_to_string(fixture.project.join("game.gd"))?, human);
    assert!(fixture.store.list::<Value>("operation")?.is_empty());
    assert!(fixture
        .store
        .list::<Value>("validationCoverage")?
        .is_empty());
    Ok(())
}

#[test]
fn parent_auto_completion_requires_integrated_code_and_accepted_children() -> Result<()> {
    for changed_child in [false, true] {
        let mut fixture = Fixture::new()?;
        let snapshot = fixture.files.capture(&fixture.project)?;
        let mut parent = json!({
            "id":"t","projectId":"p","title":"Deliver game","prompt":"Deliver both features",
            "status":"waitingChildren","capability":"code","decompose":true,"autoAccept":true,
            "validationVersion":1,"baseline":snapshot,"changes":[],
            "plan":{"summary":"Deliver both features","steps":[
                {"title":"Player","prompt":"Implement player","direction":"gameplay","acceptance":"Player can move"},
                {"title":"UI","prompt":"Implement HUD","direction":"visual","acceptance":"HUD shows health"}
            ]}
        });
        fixture.store.put("task", "t", &parent)?;
        crate::task_plan::expand(&mut fixture.store, fixture.files.root(), &mut parent)?;
        for id in parent["subtaskIds"].as_array().unwrap() {
            let id = id.as_str().unwrap();
            let mut child: Value = repository::get(&fixture.store, "task", id)?;
            child["status"] = json!("completed");
            child["accepted"] = json!(true);
            fixture.store.put("task", id, &child)?;
        }
        crate::task_plan::reconcile(&mut fixture.store, &fixture.files)?;
        parent = repository::get(&fixture.store, "task", "t")?;
        assert_eq!(parent["status"], "queued");
        assert_eq!(parent["integrationValidation"], true);
        assert_ne!(parent["accepted"], true);
        parent["status"] = json!("running");
        fixture.store.put("task", "t", &parent)?;
        pass(&fixture, snapshot)?;
        if changed_child {
            let id = parent["subtaskIds"][0].as_str().unwrap();
            let mut child: Value = repository::get(&fixture.store, "task", id)?;
            child["status"] = json!("interrupted");
            fixture.store.put("task", id, &child)?;
        }
        let task = finish(&mut fixture)?;
        assert_eq!(
            task["status"],
            if changed_child { "failed" } else { "completed" }
        );
        assert_eq!(task["accepted"] == true, !changed_child);
        assert_eq!(
            fixture.store.list::<Value>("validationCoverage")?.len(),
            usize::from(!changed_child)
        );
    }
    Ok(())
}

#[test]
fn automatic_repairs_stop_on_repetition_budget_or_missing_engine() -> Result<()> {
    for mode in ["budget", "repeated", "missingEngine"] {
        let mut fixture = Fixture::new()?;
        delivery(&fixture, true)?;
        let mut task: Value = repository::get(&fixture.store, "task", "t")?;
        for attempt in 0..3 {
            task["status"] = json!("failed");
            task["validationRepair"] = json!({
                "runId":"failed-run","snapshotId":if mode == "repeated" { 0 } else { attempt },
                "engineVersion":if mode == "missingEngine" { "" } else { "4.4.1.stable" },
                "error":"Expected counter to increment by one"
            });
            task_completion::repair(&mut fixture.store, &fixture.files, &mut task)?;
            let retry = mode != "missingEngine" && attempt < if mode == "repeated" { 1 } else { 2 };
            assert_eq!(task["status"], if retry { "queued" } else { "failed" });
            assert_ne!(task["accepted"], true);
        }
        let events = fixture.store.events("t")?;
        assert!(events.iter().all(|event| event.kind != "user"));
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind == "validationRepair")
                .count(),
            match mode {
                "budget" => 2,
                "repeated" => 1,
                _ => 0,
            }
        );
        assert!(fixture.store.list::<Value>("operation")?.is_empty());
    }
    Ok(())
}
