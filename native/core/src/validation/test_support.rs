use super::{
    flow::Definition,
    model::{CodeReport, Evidence, Flow, Run},
    repository,
};
use crate::{files::Files, store::Store};
use anyhow::Result;
use serde_json::json;
use std::{fs, path::PathBuf};

pub struct Fixture {
    pub store: Store,
    pub files: Files,
    pub project: PathBuf,
    _temp: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let store = Store::open(&data)?;
        let files = Files::new(data);
        let project = temp.path().join("project");
        fs::create_dir_all(project.join("tests"))?;
        fs::write(project.join("project.godot"), "config_version=5\n")?;
        fs::write(project.join("game.gd"), "extends Node\nvar value = 1\n")?;
        fs::write(project.join("tests/test_game.gd"),
            "extends GutTest\nfunc test_default_value():\n\tvar subject = load(\"res://game.gd\").new()\n\tassert_eq(subject.value, 1)\n\tsubject.free()\n")?;
        fs::write(project.join("export_presets.cfg"),
            "[preset.0]\nname=\"Windows Desktop\"\nplatform=\"Windows Desktop\"\nexport_filter=\"all_resources\"\n")?;
        store.put(
            "project",
            "p",
            &json!({"id":"p","name":"Validation fixture","path":project}),
        )?;
        Ok(Self {
            store,
            files,
            project,
            _temp: temp,
        })
    }

    pub fn flow(&mut self) -> Result<Flow> {
        let definition: Definition = serde_json::from_value(json!({
            "key":"game","name":"Game state","category":"feature","purpose":"Show game state",
            "config":{"width":64,"height":64},
            "steps":[{"id":"before","kind":"capture"},{"id":"after","kind":"capture"}],
            "references":[{"path":"game.gd","node":"Game","source":"author"}]
        }))?;
        repository::save_flow(&mut self.store, "p", definition, 0)
    }

    pub fn run(&self, flow: Option<Flow>) -> Result<Run> {
        repository::build_run(
            &self.store,
            "p",
            self.files.capture(&self.project)?,
            flow,
            None,
            None,
        )
    }

    pub fn save(&self, run: &Run) -> Result<()> {
        self.store.put("validationRun", &run.id, run)
    }

    // Synthetic media/reports exercise persistence and gates; the adapter contract uses Godot.
    pub fn visual(&self, flow: &Flow) -> Result<Run> {
        let mut run = self.run(Some(flow.clone()))?;
        self.complete_visual(&mut run)?;
        Ok(run)
    }

    pub fn complete_visual(&self, run: &mut Run) -> Result<()> {
        let flow = run.flow.as_ref().unwrap();
        run.status = "completed".into();
        run.engine_version = "4.4.1.stable".into();
        run.completed_steps = flow.definition.steps.len();
        let directory = repository::run_dir(self.files.root(), &run.id)?;
        fs::create_dir_all(&directory)?;
        for step in &flow.definition.steps {
            let id = repository::id();
            let file = format!("{id}.png");
            let path = directory.join(&file);
            image::RgbImage::from_pixel(64, 64, image::Rgb([32, 64, 96])).save(&path)?;
            run.evidence.push(Evidence {
                id,
                file,
                sha256: crate::files::file_hash(&path)?.unwrap(),
                kind: "image".into(),
                point: format!("{}:capture", step.id),
                start: 0.0,
                end: 0.0,
                references: flow.definition.references.clone(),
                state: json!({"scene":"res://game.tscn"}),
            });
        }
        self.save(run)
    }

    pub fn passing_code(&self, run: &mut Run) -> Result<()> {
        let directory = repository::run_dir(self.files.root(), &run.id)?;
        fs::create_dir_all(&directory)?;
        let xml = "<testsuites tests=\"1\"><testsuite><testcase name=\"test_default_value\" classname=\"tests/test_game.gd\" status=\"pass\" assertions=\"1\"/></testsuite></testsuites>";
        fs::write(directory.join("gut.xml"), xml)?;
        fs::write(directory.join("gut.log"), "Unit fixture: 1 passed")?;
        run.code = Some(CodeReport {
            gut_version: super::GUT_VERSION.into(),
            directories: vec!["tests".into()],
            cases: super::code::parse_report(xml)?,
            passed: 1,
            failed: 0,
            skipped: 0,
            report_sha256: crate::files::file_hash(&directory.join("gut.xml"))?.unwrap(),
            output_sha256: crate::files::file_hash(&directory.join("gut.log"))?.unwrap(),
            exit_code: Some(0),
        });
        run.status = "completed".into();
        run.verdict = "autoPassed".into();
        run.engine_version = "4.4.1.stable".into();
        self.save(run)
    }
}
