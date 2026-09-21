use super::{
    fixture,
    project_references::{capture, project},
};
use anyhow::Result;
use beaver_core::{
    asset_checkpoint, asset_feedback::Submission, asset_submission, asset_task, asset_work::Attempt,
};
use serde_json::{json, Value};
use std::{
    fs,
    time::{Duration, SystemTime},
};

#[test]
fn project_retention_pins_attempts_feedback_and_unstarted_followups_after_move() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let files = runtime.files();
    let directory = files.checkpoints(&id)?;
    fs::create_dir_all(&directory)?;
    let names = [
        "current",
        "feedback",
        "attempt-start",
        "attempt-end",
        "restore",
        "seed",
        "unused-0",
        "unused-1",
        "unused-2",
        "unused-3",
    ];
    let mut locations = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let path = directory.join(format!("{name}.blend"));
        fs::write(&path, b"BLENDER retained checkpoint")?;
        fs::OpenOptions::new().write(true).open(&path)?.set_times(
            fs::FileTimes::new().set_modified(
                SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + index as u64),
            ),
        )?;
        locations.push(format!(
            ".beaver/workspaces/.codex/{id}/asset-checkpoints/{name}.blend"
        ));
    }
    let reference = capture(&runtime, &id)?;
    {
        let handle = runtime.store();
        let mut db = handle.lock().unwrap();
        asset_submission::submit(
            &mut db,
            &files,
            Submission {
                id: id.clone(),
                feedback_id: "retained".into(),
                timing: "now".into(),
                text: "Keep this scene".into(),
                reference_id: reference.id,
                annotations: vec![],
            },
            Some(&fixture::frame()),
            &Value::Null,
            &Value::Null,
        )?;
        let mut state = asset_task::get(&db, &id)?;
        state.checkpoint = Some(locations[0].clone());
        state.feedback[0].checkpoint = Some(locations[1].clone());
        let attempt: Attempt = serde_json::from_value(json!({
            "id":"attempt", "subtaskId":"work", "stageId":"body", "definition":{"title":"Model","goal":"Make model","acceptance":"Saved scene"},
            "status":"completed", "threadId":"thread", "turnId":"turn", "sessionId":"session",
            "checkpoint": locations[2], "endCheckpoint":locations[3], "assetRevision":0, "inputCandidates":[],
            "inputs":{},"outputs":{},"tools":[],"summary":"Saved", "recoveryNote":"", "startedAt":"now","endedAt":"later"
        }))?;
        state.work.attempts.push(attempt);
        asset_task::save(&db, &state)?;
        db.put("task", "queued", &json!({"id":"queued", "projectId":"project", "status":"queued", "assetRestore":locations[4], "assetFeedbackSeed":{"checkpoint":locations[5]}}))?;
    }
    let root = runtime.project_root().to_owned();
    drop(files);
    drop(runtime);
    let moved = temp.path().join("moved");
    fs::rename(root, &moved)?;
    let reopened =
        beaver_core::project_storage::ProjectStore::open(&moved, "project")?.into_runtime();
    let files = reopened.files();
    let directory = files.checkpoints(&id)?;
    asset_checkpoint::prune(&reopened.store().lock().unwrap(), &files, &id, &directory)?;
    for name in names.iter().take(6).chain(names.iter().skip(8)) {
        assert_eq!(
            fs::read(directory.join(format!("{name}.blend")))?,
            b"BLENDER retained checkpoint"
        );
    }
    for name in &names[6..8] {
        assert!(!directory.join(format!("{name}.blend")).exists());
    }
    assert_eq!(fs::read_dir(directory)?.count(), 8);
    Ok(())
}

#[test]
fn legacy_recorded_checkpoint_paths_remain_readable_and_pinned() -> Result<()> {
    let fixture = fixture::Fixture::new()?;
    let directory = fixture.root.join("historical-checkpoints");
    fs::create_dir(&directory)?;
    let archived = directory.join("archived.blend");
    fs::write(&archived, b"BLENDER archived")?;
    let mut state = fixture.state()?;
    state.checkpoint = Some(archived.to_string_lossy().into_owned());
    asset_task::save(&fixture.store, &state)?;
    for index in 0..5 {
        fs::write(
            directory.join(format!("orphan-{index}.blend")),
            b"BLENDER unused",
        )?;
    }
    asset_checkpoint::prune(&fixture.store, &fixture.files, &fixture.id, &directory)?;
    assert_eq!(
        fixture
            .files
            .resolve_checkpoint(state.checkpoint.as_deref().unwrap())?,
        archived
    );
    assert_eq!(fs::read(archived)?, b"BLENDER archived");
    assert_eq!(fs::read_dir(directory)?.count(), 3);
    Ok(())
}
