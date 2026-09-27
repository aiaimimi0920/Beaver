//! Frozen Blender geometry previews share the managed queue and immutable evidence store.
use super::{
    model::{Evidence, Run},
    repository,
    sandbox::engine_path,
};
use crate::{
    files::{file_hash, safe_path, Files},
    process,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, sync::atomic::AtomicBool, time::Duration};

pub(crate) fn engine(run: &Run) -> &'static str {
    if run.kind == "objectPreview"
        && run
            .flow
            .as_ref()
            .is_some_and(|flow| flow.definition.entry.ends_with(".blend"))
    {
        "blender"
    } else {
        "godot"
    }
}

pub(crate) fn execute(
    files: &Files,
    engine: &Path,
    run: &mut Run,
    cancelled: &AtomicBool,
    progress: &impl Fn(&Run),
) -> Result<()> {
    let flow = run.flow.as_ref().context("PREVIEW_FLOW_MISSING")?.clone();
    flow.definition.validate()?;
    let output = repository::run_dir(files, &run.id)?;
    fs::create_dir_all(&output)?;
    let temporary = tempfile::Builder::new()
        .prefix("blender-preview-")
        .tempdir_in(&output)?;
    let project = temporary.path().join("project");
    files.restore_copy(&run.snapshot, &project)?;
    let source = safe_path(&project, &flow.definition.entry)?;
    ensure!(source.is_file(), "BLENDER_PREVIEW_SOURCE_MISSING");
    let script = temporary.path().join("preview.py");
    fs::write(
        &script,
        include_str!("../../../../resources/preview/blender.py"),
    )?;
    let user = temporary.path().join("user");
    fs::create_dir_all(&user)?;
    let environment: BTreeMap<String, String> = [
        "BLENDER_USER_CONFIG",
        "BLENDER_USER_SCRIPTS",
        "BLENDER_USER_DATAFILES",
    ]
    .into_iter()
    .map(|key| (key.into(), engine_path(&user)))
    .collect();
    let version = process::run_cancellable_env(
        engine,
        &["--version"],
        Some(&project),
        Duration::from_secs(15),
        cancelled,
        &environment,
    )?;
    ensure!(
        version.code == 0 && version.text.starts_with("Blender "),
        "BLENDER_PREVIEW_ENGINE_INVALID"
    );
    run.engine_version = version.text.lines().next().unwrap_or_default().into();
    run.runner_version = "beaver-blender-solid-v1".into();
    run.phase = "capturing".into();
    progress(run);
    let args = [
        "--background".to_owned(),
        "--factory-startup".into(),
        "--disable-autoexec".into(),
        "--python-exit-code".into(),
        "1".into(),
        "--python".into(),
        engine_path(&script),
        "--".into(),
        engine_path(&source),
        engine_path(&project),
        engine_path(&output),
        flow.definition.config.width.to_string(),
        flow.definition.config.height.to_string(),
    ];
    let result = process::run_cancellable_env(
        engine,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        Some(&project),
        Duration::from_secs(180),
        cancelled,
        &environment,
    )?;
    fs::write(output.join("blender.log"), &result.text)?;
    run.log = result.text;
    ensure!(
        result.code == 0,
        "BLENDER_PREVIEW_FAILED: {}",
        run.log
            .chars()
            .rev()
            .take(4000)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    );
    let state: Value = serde_json::from_slice(&fs::read(output.join("blender.json"))?)?;
    ensure!(
        state["complete"] == true && state["renderer"] == "BLENDER_WORKBENCH",
        "BLENDER_PREVIEW_INCOMPLETE"
    );
    let path = output.join("blender.png");
    let image = image::open(&path)?;
    ensure!(
        image.width() == flow.definition.config.width
            && image.height() == flow.definition.config.height,
        "BLENDER_PREVIEW_RESOLUTION_MISMATCH"
    );
    run.evidence.push(Evidence {
        id: repository::id(),
        file: "blender.png".into(),
        sha256: file_hash(&path)?.context("BLENDER_PREVIEW_IMAGE_MISSING")?,
        kind: "image".into(),
        point: "preview:capture".into(),
        start: 0.0,
        end: 0.0,
        references: vec![],
        state,
    });
    run.completed_steps = flow.definition.steps.len();
    Ok(())
}
