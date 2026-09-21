use super::fixture::{frame, png};
use anyhow::Result;
use beaver_core::{
    asset_agent,
    asset_feedback::Submission,
    asset_preview::Client,
    asset_reference, asset_submission,
    asset_task::{self, Annotation, Feedback, Reference},
    files::Files,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
    task_create,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub(super) fn project(parent: &Path) -> Result<(ProjectRuntime, String)> {
    let root = parent.join("game");
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(&root, "project")?.into_runtime();
    let handle = runtime.store();
    let mut db = handle.lock().unwrap();
    db.put("project", "project", &json!({"id":"project","path":root}))?;
    let mut task = task_create::create(
        &mut db,
        &runtime.files(),
        json!({"projectId":"project","prompt":"Create a character","assetTask":true}),
        &Value::Null,
        &Value::Null,
    )?;
    let id = task["id"].as_str().unwrap().to_owned();
    task["status"] = json!("running");
    db.put("task", &id, &task)?;
    let mut state = asset_task::enable(&db, &task)?;
    state.session_id = Some("session".into());
    asset_task::save(&db, &state)?;
    Ok((runtime, id))
}

pub(super) fn capture(runtime: &ProjectRuntime, task: &str) -> Result<Reference> {
    asset_reference::capture(
        &runtime.store().lock().unwrap(),
        &runtime.files(),
        task,
        frame(),
        &png()?,
    )
}

#[tokio::test]
async fn moved_project_keeps_frozen_frames_feedback_and_followup_model_images() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let root = runtime.project_root().to_owned();
    let reference = capture(&runtime, &id)?;
    assert_eq!(
        reference.image_path,
        format!(
            ".beaver/evidence/asset-observer/{id}/references/{}.png",
            reference.id
        )
    );
    assert!(root.join(&reference.image_path).is_file());
    let annotations = vec![Annotation {
        kind: "box".into(),
        points: vec![[0.25, 0.25], [0.75, 0.75]],
    }];
    let destination = {
        let handle = runtime.store();
        let mut db = handle.lock().unwrap();
        let mut task: Value = db.get("task", &id)?.unwrap();
        task["status"] = json!("completed");
        db.put("task", &id, &task)?;
        let checkpoint = runtime.files().workspace(&id)?.join("saved.blend");
        fs::write(&checkpoint, b"BLENDER fixture checkpoint")?;
        let mut state = asset_task::get(&db, &id)?;
        state.checkpoint = Some(checkpoint.to_string_lossy().into_owned());
        asset_task::save(&db, &state)?;
        let receipt = asset_submission::submit(
            &mut db,
            &runtime.files(),
            Submission {
                id: id.clone(),
                feedback_id: "followup".into(),
                timing: "now".into(),
                text: "Change the marked area".into(),
                reference_id: reference.id.clone(),
                annotations: annotations.clone(),
            },
            None,
            &Value::Null,
            &Value::Null,
        )?;
        receipt.task_id
    };
    drop(runtime);
    let moved = temp.path().join("moved");
    fs::rename(&root, &moved)?;
    let reopened = ProjectStore::open(&moved, "project")?.into_runtime();
    let files = reopened.files();
    let handle = reopened.store();
    {
        let db = handle.lock().unwrap();
        let source = asset_task::get(&db, &id)?;
        let mut task: Value = db.get("task", &destination)?.unwrap();
        let seed: Feedback = serde_json::from_value(task["assetFeedbackSeed"].clone())?;
        let mut state = asset_task::get(&db, &destination)?;
        for saved in [
            source.last_frame.as_ref().unwrap(),
            &source.feedback[0].reference,
            &seed.reference,
            &state.feedback[0].reference,
        ] {
            assert_eq!(saved.image_path, reference.image_path);
            assert_eq!(saved.task_id, id);
            assert_eq!(asset_reference::read(&files, saved)?, png()?);
        }
        task["status"] = json!("running");
        task["threadId"] = json!("thread");
        task["turnId"] = json!("turn");
        db.put("task", &destination, &task)?;
        state.session_id = Some("session".into());
        asset_task::save(&db, &state)?;
    }
    let annotated = asset_reference::annotated(&files, &reference, &annotations)?;
    assert_eq!(
        image::load_from_memory(&annotated)?
            .to_rgba8()
            .get_pixel(15, 11)
            .0,
        [255, 195, 50, 255]
    );
    let agent = asset_agent::Context {
        store: handle,
        files: files.clone(),
        client: Client::new(
            0,
            "unused".into(),
            destination,
            "project".into(),
            "session".into(),
            moved.join("checkpoints"),
        )?,
    };
    let reply = agent
        .call(&json!({"threadId":"thread","turnId":"turn",
        "arguments":{"operation":"poll"}}))
        .await?;
    let image = reply.value["contentItems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["type"] == "inputImage")
        .unwrap();
    use base64::Engine;
    assert_eq!(
        image["imageUrl"],
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&annotated)
        )
    );
    assert_eq!(asset_reference::read(&files, &reference)?, png()?);
    Ok(())
}

#[test]
fn project_reads_reject_missing_corrupt_and_foreign_paths_without_host_fallback() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let reference = capture(&runtime, &id)?;
    let files = runtime.files();
    let path = runtime.project_root().join(&reference.image_path);
    let host = temp.path().join("host");
    let host_path = host.join(format!(
        "asset-observer/{id}/references/{}.png",
        reference.id
    ));
    fs::create_dir_all(host_path.parent().unwrap())?;
    fs::write(&host_path, png()?)?;
    let mut legacy = reference.clone();
    legacy.image_path = host_path.to_string_lossy().into_owned();
    assert_eq!(asset_reference::read(&Files::new(host), &legacy)?, png()?);
    assert!(asset_reference::read(&files, &legacy).is_err());
    for invalid in [
        path.to_string_lossy().into_owned(),
        reference.image_path.replace(&id, "another-task"),
        reference
            .image_path
            .replace("/references/", "/references/../references/"),
        reference.image_path.replace('/', "\\"),
    ] {
        let mut changed = reference.clone();
        changed.image_path = invalid;
        assert!(asset_reference::read(&files, &changed).is_err());
    }
    fs::write(&path, b"corrupt")?;
    assert!(asset_reference::read(&files, &reference).is_err());
    fs::remove_file(&path)?;
    assert!(asset_reference::read(&files, &reference).is_err());
    assert!(!path.exists());
    assert_eq!(
        asset_reference::read(&Files::new(temp.path().into()), &legacy)?,
        png()?
    );
    Ok(())
}

#[test]
fn project_retention_preserves_submitted_frames_and_removes_only_owned_unused_images() -> Result<()>
{
    let temp = tempfile::tempdir()?;
    let (runtime, id) = project(temp.path())?;
    let pinned = capture(&runtime, &id)?;
    let handle = runtime.store();
    let mut db = handle.lock().unwrap();
    let files = runtime.files();
    asset_submission::submit(
        &mut db,
        &files,
        Submission {
            id: id.clone(),
            feedback_id: "pinned".into(),
            timing: "now".into(),
            text: "Keep this view".into(),
            reference_id: pinned.id.clone(),
            annotations: vec![],
        },
        Some(&frame()),
        &Value::Null,
        &Value::Null,
    )?;
    let mut old = asset_reference::capture(&db, &files, &id, frame(), &png()?)?;
    let owned_path = runtime.project_root().join(&old.image_path);
    let unrelated = temp.path().join("unrelated.png");
    fs::write(&unrelated, png()?)?;
    old.image_path = unrelated.to_string_lossy().into_owned();
    db.put("asset-reference", &old.id, &old)?;
    for index in 1..=12 {
        let mut next = frame();
        next.captured_at += index;
        asset_reference::capture(&db, &files, &id, next, &png()?)?;
    }
    assert!(!owned_path.exists());
    assert_eq!(fs::read(&unrelated)?, png()?);
    assert!(asset_reference::get(&db, &id, &old.id).is_err());
    let saved = db.list::<Reference>("asset-reference")?;
    assert_eq!(saved.iter().filter(|r| !r.used).count(), 8);
    assert_eq!(saved.iter().filter(|r| r.used).count(), 1);
    assert_eq!(asset_reference::read(&files, &pinned)?, png()?);
    Ok(())
}
