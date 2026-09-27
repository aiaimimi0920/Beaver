use super::{
    live_preview::View,
    model::Run,
    sandbox::{check_output, engine_path, Sandbox},
};
use crate::files::Files;
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub(super) fn execute(
    files: &Files,
    engine: &Path,
    run: &Run,
    session: &str,
    stop: &AtomicBool,
    view: &Arc<Mutex<View>>,
) -> Result<()> {
    if super::blender_preview::engine(run) == "blender" {
        return super::blender_live_preview::execute(files, engine, run, session, stop, view);
    }
    let temporary = tempfile::Builder::new()
        .prefix("beaver-live-preview-")
        .tempdir()?;
    let sandbox = Sandbox::prepare_at(files, run, temporary.path().join("output"))?;
    let runtime = sandbox.project.join("beaver_live_preview");
    ensure!(!runtime.exists(), "PREVIEW_RESERVED_PATH");
    fs::create_dir(&runtime)?;
    fs::write(
        runtime.join("driver.gd"),
        include_str!("../../../../resources/preview/driver.gd"),
    )?;
    fs::write(
        runtime.join("camera.gd"),
        include_str!("../../../../resources/preview/camera.gd"),
    )?;
    fs::write(
        runtime.join("picking.gd"),
        include_str!("../../../../resources/preview/picking.gd"),
    )?;
    fs::write(
        runtime.join("box_picking.gd"),
        include_str!("../../../../resources/preview/box_picking.gd"),
    )?;
    let flow = &run
        .flow
        .as_ref()
        .context("PREVIEW_FLOW_MISSING")?
        .definition;
    let config = json!({"sessionId":session,"output":engine_path(&sandbox.output),
        "width":flow.config.width,"height":flow.config.height});
    fs::write(runtime.join("config.json"), serde_json::to_vec(&config)?)?;
    let mut overrides = fs::read_to_string(sandbox.project.join("override.cfg"))?;
    overrides.push_str(&format!("\n[autoload]\nBeaverLivePreview=\"*res://beaver_live_preview/driver.gd\"\n[application]\nrun/main_scene={}\n",
        serde_json::to_string(&format!("res://{}", super::flow::relative(&flow.entry)?))?));
    fs::write(sandbox.project.join("override.cfg"), overrides)?;
    let resolution = format!("{}x{}", flow.config.width, flow.config.height);
    super::live_preview_monitor::execute(&sandbox.output, session, stop, view, || {
        sandbox.import(engine, stop)?;
        let output = sandbox.execute(
            engine,
            &[
                "--path",
                ".",
                "--rendering-method",
                "gl_compatibility",
                "--audio-driver",
                "Dummy",
                "--resolution",
                &resolution,
                "--disable-vsync",
            ],
            3600,
            stop,
        )?;
        check_output(&output)
    })
}
