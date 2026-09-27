use super::{live_preview::View, model::Run, sandbox::engine_path};
use crate::{
    files::{safe_path, Files},
    process,
};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

pub(super) fn execute(
    files: &Files,
    engine: &Path,
    run: &Run,
    session: &str,
    stop: &AtomicBool,
    view: &Arc<Mutex<View>>,
) -> Result<()> {
    let temporary = tempfile::Builder::new()
        .prefix("beaver-blender-live-")
        .tempdir()?;
    let project = temporary.path().join("project");
    files.restore_copy(&run.snapshot, &project)?;
    let flow = &run
        .flow
        .as_ref()
        .context("PREVIEW_FLOW_MISSING")?
        .definition;
    let source = safe_path(&project, &flow.entry)?;
    ensure!(source.is_file(), "BLENDER_PREVIEW_SOURCE_MISSING");
    let output = temporary.path().join("output");
    fs::create_dir(&output)?;
    fs::write(
        temporary.path().join("preview.py"),
        include_str!("../../../../resources/preview/blender.py"),
    )?;
    let script = temporary.path().join("live.py");
    fs::write(
        temporary.path().join("blender_picking.py"),
        include_str!("../../../../resources/preview/blender_picking.py"),
    )?;
    fs::write(
        &script,
        include_str!("../../../../resources/preview/blender_live.py"),
    )?;
    let user = temporary.path().join("user");
    fs::create_dir(&user)?;
    let environment: BTreeMap<String, String> = [
        "BLENDER_USER_CONFIG",
        "BLENDER_USER_SCRIPTS",
        "BLENDER_USER_DATAFILES",
    ]
    .into_iter()
    .map(|key| (key.into(), engine_path(&user)))
    .collect();
    let args = [
        "--background".into(),
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
        flow.config.width.to_string(),
        flow.config.height.to_string(),
        session.into(),
    ];
    super::live_preview_monitor::execute(&output, session, stop, view, || {
        let result = process::run_cancellable_env(
            engine,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            Some(&project),
            Duration::from_secs(3600),
            stop,
            &environment,
        )?;
        ensure!(
            result.code == 0,
            "BLENDER_PREVIEW_FAILED: {}",
            result
                .text
                .chars()
                .rev()
                .take(4000)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
        Ok(())
    })
}
