use super::{prepare, required};
use crate::{
    asset_task,
    executor::Outcome,
    files::Change,
    task_finish,
    validation::{model::Run, repository, test_support::Fixture},
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{fs, sync::atomic::AtomicBool};

fn addition(path: &str) -> Change {
    Change {
        path: path.into(),
        before: None,
        after: Some("new".into()),
    }
}

#[test]
fn asset_scope_preserves_runtime_and_explicit_integration_gates() {
    let asset = json!({"assetTask":true});
    let assets: Vec<_> = [
        "assets/red_ball.blend",
        "assets/red_ball.GLB",
        "assets/preview.png",
        "docs/design.md",
        "beaver.validation.json",
    ]
    .into_iter()
    .map(addition)
    .collect();
    assert!(!required(&asset, &assets));
    assert!(required(&json!({}), &assets));
    assert!(required(
        &json!({"assetTask":true,"integrationValidation":true}),
        &assets
    ));
    for path in [
        "game.gd",
        "main.tscn",
        "material.tres",
        "project.godot",
        "export_presets.cfg",
        "settings.json",
        "addons/plugin.gd",
        "assets/unknown.bin",
    ] {
        let mut mixed = assets.clone();
        mixed.push(addition(path));
        assert!(required(&asset, &mixed), "Runtime gate missing: {path}");
    }
    for path in ["assets/red_ball.glb", "beaver.validation.json"] {
        let deleted = Change {
            path: path.into(),
            before: Some("old".into()),
            after: None,
        };
        assert!(required(&asset, &[deleted]));
    }
    let changed_policy = Change {
        before: Some("old-policy".into()),
        ..addition("beaver.validation.json")
    };
    assert!(required(&asset, &[changed_policy]));
}

#[test]
fn asset_only_output_merges_without_gut_or_coverage_job() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let baseline = fixture.files.capture(&fixture.project)?;
    let workspace = fixture.files.root().join("workspaces/t");
    fixture.files.restore_copy(&baseline, &workspace)?;
    let outputs = [
        ("red_ball.blend", "fixture blend bytes"),
        ("red_ball.glb", "fixture glb bytes"),
        ("preview.png", "fixture preview bytes"),
        (
            "beaver.validation.json",
            r#"{"schemaVersion":1,"code":{"directories":[]},"flows":[]}"#,
        ),
    ];
    for (path, bytes) in outputs {
        fs::write(workspace.join(path), bytes)?;
    }
    let task = json!({
        "id":"t","projectId":"p","status":"running","capability":"code",
        "autoAccept":true,"validationVersion":1,"workspace":workspace,
        "baseline":baseline,"changes":[]
    });
    fixture.store.put("task", "t", &task)?;
    asset_task::enable(&fixture.store, &task)?;
    assert!(prepare(&fixture.store, &fixture.files, "t")?.is_none());
    let pending: Value = repository::get(&fixture.store, "task", "t")?;
    assert_ne!(pending["validationPrepared"], true);
    let mut state = asset_task::get(&fixture.store, "t")?;
    state.phase = "ready".into();
    asset_task::save(&fixture.store, &state)?;
    assert!(prepare(&fixture.store, &fixture.files, "t")?.is_none());
    let task = task_finish::finish(
        &mut fixture.store,
        &fixture.files,
        "t",
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(task["status"], "completed", "{task}");
    assert_eq!(task["accepted"], true);
    assert_eq!(task["codeValidation"]["status"], "notRequired");
    assert!(fixture.store.list::<Run>("validationRun")?.is_empty());
    assert!(fixture
        .store
        .list::<Value>("validationCoverage")?
        .is_empty());
    for (path, bytes) in outputs {
        assert_eq!(fs::read_to_string(fixture.project.join(path))?, bytes);
    }
    assert_eq!(
        fs::read(fixture.project.join("game.gd"))?,
        fs::read(workspace.join("game.gd"))?
    );
    Ok(())
}
