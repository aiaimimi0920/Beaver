use super::{code, model::Run, repository, sandbox::Sandbox, visual};
use crate::files::Files;
use anyhow::Result;
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn execute(
    files: &Files,
    engine: &Path,
    ffmpeg: Option<&Path>,
    run: &mut Run,
    cancelled: &AtomicBool,
    progress: impl Fn(&Run),
) {
    run.status = "running".into();
    run.phase = "preparing".into();
    progress(run);
    let result = (|| -> Result<()> {
        if super::blender_preview::engine(run) == "blender" {
            return super::blender_preview::execute(files, engine, run, cancelled, &progress);
        }
        let sandbox = Sandbox::prepare(files, run)?;
        let version = sandbox.execute(engine, &["--version"], 15, cancelled)?;
        run.engine_version = version.text.trim().into();
        let minor = run
            .engine_version
            .split('.')
            .nth(1)
            .and_then(|s| s.parse::<u32>().ok());
        if version.code != 0 || !run.engine_version.starts_with("4.") || minor.is_none_or(|n| n < 4)
        {
            anyhow::bail!("Validation requires a Godot 4.4 or newer 4.x editor (GUT 9.4.0)");
        }
        if run.kind == "code" {
            code::execute(engine, &sandbox, run, cancelled, &progress)
        } else {
            visual::execute(engine, ffmpeg, &sandbox, run, cancelled, &progress)
        }
    })();
    run.finished_at = Some(repository::now());
    match result {
        Ok(()) => {
            run.status = "completed".into();
            run.phase = "completed".into();
        }
        Err(error) => {
            run.status = if cancelled.load(Ordering::SeqCst) {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            run.phase = run.status.clone();
            run.verdict = "needsReview".into();
            run.error = Some(error.to_string());
        }
    }
    progress(run);
}
