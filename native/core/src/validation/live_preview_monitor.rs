use super::live_preview::View;
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub(super) fn execute(
    output: &Path,
    session: &str,
    stop: &AtomicBool,
    view: &Arc<Mutex<View>>,
    launch: impl FnOnce() -> Result<()> + Send,
) -> Result<()> {
    thread::scope(|scope| {
        let process = scope.spawn(launch);
        let mut command = Value::Null;
        let mut sequence = 0;
        let mut failure = None;
        while !process.is_finished() {
            let update = (|| -> Result<()> {
                let mut state = view
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
                if state.touched.elapsed() > Duration::from_secs(15) {
                    stop.store(true, Ordering::SeqCst);
                }
                let next = json!({"revision":state.revision,"camera":state.camera,
                    "pick": if state.pick_request.is_null() { json!({}) } else { state.pick_request.clone() },
                    "frozen":state.frozen,
                    "width":state.width,"height":state.height,
                    "active":state.touched.elapsed() < Duration::from_secs(2)});
                if next != command {
                    let temporary = output.join("command.tmp");
                    fs::write(&temporary, serde_json::to_vec(&next)?)?;
                    fs::rename(temporary, output.join("command.json"))?;
                    command = next;
                }
                if let Ok(frame) = read_frame(
                    &output,
                    session,
                    sequence,
                    state.width,
                    state.height,
                    state.revision,
                ) {
                    if let Some(error) = frame["error"].as_str() {
                        anyhow::bail!("{error}");
                    }
                    ensure!(frame["frozen"] == state.frozen, "PREVIEW_FREEZE_MISMATCH");
                    sequence = frame["sequence"]
                        .as_u64()
                        .context("PREVIEW_FRAME_SEQUENCE")?;
                    state.frame = frame;
                    state.status = "ready".into();
                }
                super::live_preview_pick::receive(&output, &mut state)?;
                Ok(())
            })();
            if let Err(error) = update {
                if let Ok(mut state) = view.lock() {
                    state.error = Some(error.to_string());
                }
                failure = Some(error);
                stop.store(true, Ordering::SeqCst);
            }
            thread::sleep(Duration::from_millis(125));
        }
        let result = process
            .join()
            .map_err(|_| anyhow::anyhow!("Preview process worker failed"))?;
        if let Some(error) = failure {
            return Err(error);
        }
        result
    })
}

fn read_frame(
    output: &Path,
    session: &str,
    previous: u64,
    width: u32,
    height: u32,
    revision: u64,
) -> Result<Value> {
    let mut frame: Value = serde_json::from_slice(&fs::read(output.join("frame.json"))?)?;
    ensure!(
        frame["sessionId"] == session,
        "PREVIEW_FRAME_SESSION_MISMATCH"
    );
    if frame["error"].is_string() {
        return Ok(frame);
    }
    let sequence = frame["sequence"]
        .as_u64()
        .context("PREVIEW_FRAME_SEQUENCE")?;
    ensure!(
        sequence > previous
            && frame["width"] == width
            && frame["height"] == height
            && frame["revision"] == revision,
        "PREVIEW_STALE_FRAME"
    );
    let path = output.join(format!("frame-{sequence}.png"));
    ensure!(
        fs::metadata(&path)?.len() <= 16 * 1024 * 1024,
        "PREVIEW_FRAME_TOO_LARGE"
    );
    let bytes = fs::read(path)?;
    let image = image::load_from_memory(&bytes)?;
    ensure!(
        image.width() == width && image.height() == height,
        "PREVIEW_FRAME_DIMENSIONS"
    );
    frame["sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
    frame["dataUrl"] = json!(format!("data:image/png;base64,{}", STANDARD.encode(bytes)));
    Ok(frame)
}
