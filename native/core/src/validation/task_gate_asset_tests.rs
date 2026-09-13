use super::{prepare, validation_only};
use crate::{
    asset_feedback::{self, Submission},
    asset_task::{self, Frame, Reference},
    executor::Outcome,
    task_finish,
    validation::{model::Run, repository, test_support::Fixture},
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn delivery(fixture: &Fixture) -> Result<PathBuf> {
    let baseline = fixture.files.capture(&fixture.project)?;
    let workspace = fixture.files.root().join("workspaces/t");
    fixture.files.restore_copy(&baseline, &workspace)?;
    fs::write(
        workspace.join("game.gd"),
        "extends Node\nvar value = 1\nvar nose_scale = 0.8\n",
    )?;
    let task = json!({
        "id":"t","projectId":"p","title":"Character","prompt":"Make a character",
        "status":"running","capability":"code","autoAccept":true,"validationVersion":1,
        "workspace":workspace,"baseline":baseline,"changes":[]
    });
    fixture.store.put("task", "t", &task)?;
    asset_task::enable(&fixture.store, &task)?;
    Ok(workspace)
}

fn ready(fixture: &Fixture) -> Result<()> {
    let mut state = asset_task::get(&fixture.store, "t")?;
    state.phase = "ready".into();
    for feedback in &mut state.feedback {
        feedback.status = "completed".into();
    }
    asset_task::save(&fixture.store, &state)
}

fn feedback(fixture: &Fixture) -> Result<()> {
    let frame = Frame {
        id: "frame".into(),
        session_id: "session".into(),
        generation: "generation".into(),
        scene_revision: 1,
        view_revision: 1,
        captured_at: 1000,
        width: 64,
        height: 64,
        view_matrix: [0.0; 16],
        projection_matrix: [0.0; 16],
    };
    // The gate consumes the durable receipt, not its preview image.
    let reference = Reference {
        id: "reference".into(),
        task_id: "t".into(),
        project_id: "p".into(),
        frame,
        pick: None,
        image_path: "fixture.png".into(),
        sha256: "fixture".into(),
        used: false,
    };
    asset_feedback::accept(
        &fixture.store,
        Submission {
            id: "t".into(),
            feedback_id: "feedback".into(),
            timing: "now".into(),
            text: "Make the nose smaller".into(),
            reference_id: reference.id.clone(),
            annotations: vec![],
        },
        reference,
    )?;
    Ok(())
}

fn pass(fixture: &Fixture) -> Result<String> {
    let mut run = prepare(&fixture.store, &fixture.files, "t")?.expect("Code candidate");
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
fn assets_and_feedback_must_finish_before_preparing_code_validation() -> Result<()> {
    for phase in ["producing", "ready"] {
        let fixture = Fixture::new()?;
        delivery(&fixture)?;
        feedback(&fixture)?;
        let mut state = asset_task::get(&fixture.store, "t")?;
        state.phase = phase.into();
        asset_task::save(&fixture.store, &state)?;
        assert!(prepare(&fixture.store, &fixture.files, "t")?.is_none());
        let task: Value = repository::get(&fixture.store, "task", "t")?;
        assert_ne!(task["validationPrepared"], true);
        assert!(fixture.store.list::<Run>("validationRun")?.is_empty());
        ready(&fixture)?;
        assert!(prepare(&fixture.store, &fixture.files, "t")?.is_some());
    }
    Ok(())
}

#[test]
fn late_asset_feedback_invalidates_old_code_and_recaptures_resumed_edits() -> Result<()> {
    for validation_retry in [false, true] {
        let mut fixture = Fixture::new()?;
        let workspace = delivery(&fixture)?;
        ready(&fixture)?;
        let old_run = pass(&fixture)?;
        let original = fs::read(fixture.project.join("game.gd"))?;
        let mut task: Value = repository::get(&fixture.store, "task", "t")?;
        task["validationOnly"] = json!(validation_retry);
        fixture.store.put("task", "t", &task)?;

        feedback(&fixture)?;
        let mut task = finish(&mut fixture)?;
        assert_eq!(task["status"], "queued");
        assert_eq!(task["assetResume"], true);
        assert!(
            !validation_only(&task),
            "Resumption must launch the asset executor"
        );
        assert_eq!(task["validationPrepared"], false);
        assert_eq!(fs::read(fixture.project.join("game.gd"))?, original);
        assert!(fixture.store.list::<Value>("operation")?.is_empty());

        let adjusted = "extends Node\nvar value = 1\nvar nose_scale = 0.6\n";
        fs::write(workspace.join("game.gd"), adjusted)?;
        ready(&fixture)?;
        task["status"] = json!("running");
        fixture.store.put("task", "t", &task)?;
        task = finish(&mut fixture)?;
        assert_eq!(
            task["status"], "queued",
            "Old green receipts cannot merge new edits"
        );
        assert_eq!(task["validationOnly"], true);
        assert_eq!(task["codeValidation"]["runId"], old_run);
        assert_eq!(fs::read(fixture.project.join("game.gd"))?, original);
        assert!(fixture.store.list::<Value>("operation")?.is_empty());

        task["status"] = json!("running");
        fixture.store.put("task", "t", &task)?;
        let new_run = pass(&fixture)?;
        assert_ne!(new_run, old_run);
        let task = finish(&mut fixture)?;
        assert_eq!(task["status"], "completed");
        assert_eq!(task["accepted"], true);
        assert_eq!(task["codeValidation"]["runId"], new_run);
        assert_eq!(
            fs::read_to_string(fixture.project.join("game.gd"))?,
            adjusted
        );
    }
    Ok(())
}
