use super::{
    agent::checkpoint_server,
    project_references::{capture, project},
};
use anyhow::Result;
use beaver_core::{
    asset_agent, asset_feedback::Submission, asset_preview::Client, asset_submission, asset_task,
    blender_session::Request, project_runtime::ProjectRuntime, project_storage::ProjectStore,
};
use serde_json::{json, Value};
use std::{fs, net::TcpListener, sync::atomic::AtomicBool};

pub(super) fn context(
    runtime: &ProjectRuntime,
    id: &str,
    port: u16,
) -> Result<asset_agent::Context> {
    let files = runtime.files();
    let checkpoints = files.checkpoints(id)?;
    fs::create_dir_all(&checkpoints)?;
    Ok(asset_agent::Context {
        store: runtime.store(),
        files,
        client: Client::new(
            port,
            "fixture-token".into(),
            id.into(),
            "project".into(),
            "session".into(),
            checkpoints,
        )?,
    })
}

#[tokio::test]
async fn checkpoint_and_followup_restore_survive_project_move() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let root = runtime.project_root().to_owned();
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let agent = context(&runtime, &id, listener.local_addr()?.port())?;
    let server = checkpoint_server(listener, agent.clone(), false);
    let recorded = agent.checkpoint().await?;
    server.join().unwrap()?;
    assert_eq!(
        recorded,
        format!(".beaver/workspaces/.codex/{id}/asset-checkpoints/checkpoint.blend")
    );
    let reference = capture(&runtime, &id)?;
    let destination = {
        let mut db = agent.store.lock().unwrap();
        let mut task: Value = db.get("task", &id)?.unwrap();
        task["status"] = json!("completed");
        db.put("task", &id, &task)?;
        asset_submission::submit(
            &mut db,
            &agent.files,
            Submission {
                id: id.clone(),
                feedback_id: "followup-checkpoint".into(),
                timing: "now".into(),
                text: "Adjust the saved model".into(),
                reference_id: reference.id,
                annotations: vec![],
            },
            None,
            &Value::Null,
            &Value::Null,
        )?
        .task_id
    };
    drop(agent);
    drop(runtime);
    let moved = temp.path().join("moved");
    fs::rename(&root, &moved)?;
    let reopened = ProjectStore::open(&moved, "project")?.into_runtime();
    let files = reopened.files();
    let handle = reopened.store();
    let db = handle.lock().unwrap();
    let source = asset_task::get(&db, &id)?;
    let followup = asset_task::get(&db, &destination)?;
    let task: Value = db.get("task", &destination)?.unwrap();
    for path in [
        source.checkpoint.as_deref(),
        followup.checkpoint.as_deref(),
        task["assetRestore"].as_str(),
    ] {
        assert_eq!(path, Some(recorded.as_str()));
        let restored = files.resolve_checkpoint(path.unwrap())?;
        assert!(restored.starts_with(fs::canonicalize(&moved)?));
        assert_eq!(fs::read(restored)?, b"BLENDER fixture scene");
    }
    // The seed is unapplied; a checkpoint is recorded on feedback only after application.
    assert!(task["assetFeedbackSeed"]["checkpoint"].is_null());
    Ok(())
}

#[tokio::test]
async fn foreign_checkpoint_directory_is_rejected_before_pruning_or_request() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    for directory in [
        temp.path().join("outside"),
        runtime.files().checkpoints("other-task")?,
    ] {
        fs::create_dir_all(&directory)?;
        for index in 0..5 {
            fs::write(
                directory.join(format!("{index}.blend")),
                b"BLENDER preserve outside",
            )?;
        }
        let mut agent = context(&runtime, &id, 0)?;
        agent.client.checkpoints = fs::canonicalize(&directory)?;
        let error = agent.checkpoint().await.unwrap_err().to_string();
        assert!(error.contains("does not belong"), "{error}");
        assert_eq!(fs::read_dir(&directory)?.count(), 5);
        assert!(asset_task::get(&agent.store.lock().unwrap(), &id)?
            .checkpoint
            .is_none());
    }
    Ok(())
}

#[test]
fn restore_rejects_absolute_foreign_and_noncanonical_locations_before_launch() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let files = runtime.files();
    let home = files.codex_home(&id)?;
    fs::create_dir_all(files.checkpoints(&id)?)?;
    let valid = format!(".beaver/workspaces/.codex/{id}/asset-checkpoints/valid.blend");
    let valid_path = files.resolve_checkpoint(&valid)?;
    fs::write(&valid_path, b"BLENDER saved scene")?;
    let handle = runtime.store();
    let task: Value = handle.lock().unwrap().get("task", &id)?.unwrap();
    for invalid in [
        valid_path.to_string_lossy().into_owned(),
        "../other/.beaver/workspaces/.codex/task/asset-checkpoints/valid.blend".into(),
        valid.replace('/', "\\"),
        valid.replace("/valid.blend", "/../valid.blend"),
        valid.replace("/valid.blend", "/nested/valid.blend"),
        valid.replace(".blend", ".txt"),
        ".beaver/workspaces/.codex".into(),
    ] {
        let before = fs::read(&valid_path)?;
        {
            let db = handle.lock().unwrap();
            let mut state = asset_task::get(&db, &id)?;
            state.checkpoint = Some(invalid);
            asset_task::save(&db, &state)?;
        }
        let request = Request::prepare(&home, &files.workspace(&id)?, &std::env::current_exe()?)?;
        let error = match request.start_logged(
            handle.clone(),
            files.clone(),
            &task,
            &AtomicBool::new(true),
        ) {
            Ok(_) => panic!("invalid restore started Blender"),
            Err(error) => error.to_string(),
        };
        assert!(
            !error.contains("preparation interrupted"),
            "restore was not checked: {error}"
        );
        assert_eq!(fs::read(&valid_path)?, before);
    }
    // A valid reference reaches the cancellation boundary without starting an engine.
    {
        let db = handle.lock().unwrap();
        let mut state = asset_task::get(&db, &id)?;
        state.checkpoint = Some(valid);
        asset_task::save(&db, &state)?;
    }
    let request = Request::prepare(&home, &files.workspace(&id)?, &std::env::current_exe()?)?;
    let error = request
        .start_logged(handle, files, &task, &AtomicBool::new(true))
        .err()
        .unwrap();
    assert!(error.to_string().contains("preparation interrupted"));
    Ok(())
}
