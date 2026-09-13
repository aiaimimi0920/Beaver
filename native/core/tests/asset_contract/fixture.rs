use anyhow::Result;
use beaver_core::{
    asset_feedback::{self, Submission},
    asset_reference, asset_submission,
    asset_task::{self, Frame, Reference, Stage, State},
    store::Store,
    task_create,
};
use image::ImageEncoder;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

pub struct Fixture {
    pub store: Store,
    pub root: PathBuf,
    pub project: PathBuf,
    pub task: Value,
    pub id: String,
    pub _temp: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("original.txt"), "original project")?;
        let mut store = Store::open(&root)?;
        store.put(
            "project",
            "project",
            &json!({"id":"project","name":"Fixture","path":project}),
        )?;
        let mut task = task_create::create(
            &mut store,
            &root,
            json!({"projectId":"project","prompt":"Create a character","assetTask":true}),
            &Value::Null,
            &Value::Null,
        )?;
        let id = task["id"].as_str().unwrap().to_owned();
        task["status"] = json!("running");
        task["threadId"] = json!("thread");
        task["turnId"] = json!("turn");
        store.put("task", &id, &task)?;
        let mut state = asset_task::enable(&store, &task)?;
        state.session_id = Some("session".into());
        asset_task::save(&store, &state)?;
        Ok(Self {
            store,
            root,
            project,
            task,
            id,
            _temp: temp,
        })
    }

    pub fn state(&self) -> Result<State> {
        asset_task::get(&self.store, &self.id)
    }

    pub fn reference(&self) -> Result<Reference> {
        asset_reference::capture(&self.store, &self.root, &self.id, frame(), &png()?)
    }

    pub fn input(&self, feedback_id: &str, timing: &str) -> Result<Submission> {
        Ok(Submission {
            id: self.id.clone(),
            feedback_id: feedback_id.into(),
            timing: timing.into(),
            text: "Make the nose smaller".into(),
            reference_id: self.reference()?.id,
            annotations: Vec::new(),
        })
    }

    pub fn submit(&mut self, input: Submission) -> Result<asset_task::Feedback> {
        asset_submission::submit(
            &mut self.store,
            &self.root,
            input,
            Some(&frame()),
            &Value::Null,
            &Value::Null,
        )
    }

    pub fn delivered(&self, id: &str) -> Result<State> {
        let mut state = self.state()?;
        asset_feedback::reserve_delivery(&mut state);
        let feedback = state.feedback.iter_mut().find(|f| f.id == id).unwrap();
        assert_eq!(feedback.status, "delivering");
        feedback.status = "waitingSwitch".into();
        feedback.delivered_at = Some(asset_task::now());
        asset_task::save(&self.store, &state)?;
        Ok(state)
    }

    pub fn finishable(&mut self) -> Result<()> {
        let checkpoint = self.root.join("fixture.blend");
        fs::write(&checkpoint, "BLENDER fixture checkpoint")?;
        let mut state = self.state()?;
        state.stages = vec![stage("body", "completed", &[])];
        state.phase = "ready".into();
        state.checkpoint = Some(checkpoint.to_string_lossy().into_owned());
        asset_task::save(&self.store, &state)
    }
}

pub fn frame() -> Frame {
    Frame {
        id: "frame".into(),
        session_id: "session".into(),
        generation: "generation".into(),
        scene_revision: 4,
        view_revision: 3,
        captured_at: 1000,
        width: 64,
        height: 48,
        view_matrix: [0.0; 16],
        projection_matrix: [0.0; 16],
    }
}

pub fn png() -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes).write_image(
        &vec![255; 64 * 48 * 4],
        64,
        48,
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(bytes)
}

pub fn stage(id: &str, status: &str, dependencies: &[&str]) -> Stage {
    Stage {
        id: id.into(),
        name: id.into(),
        status: status.into(),
        dependencies: dependencies.iter().map(|s| (*s).into()).collect(),
        objects: Vec::new(),
        evidence: if status == "completed" {
            "Geometry inspected".into()
        } else {
            String::new()
        },
        round: 1,
    }
}

pub fn acknowledge(state: &mut State, id: &str, affected: &[&str]) -> Result<()> {
    asset_feedback::receipt(
        state,
        id,
        "acknowledge",
        &json!({
            "frameId":"frame", "imageObservation":"The nose projects from the face",
            "impact":"Local nose change; retain the current hair silhouette", "affectedStages":affected,
        }),
    )
}
