use super::sandbox::{engine_path, Sandbox};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path, sync::atomic::AtomicBool};

#[derive(Deserialize)]
struct Presentation {
    window: [u32; 2],
    viewport: [u32; 2],
    content: [u32; 4],
}

/// Match the captured window, without stretching or cropping away game content.
pub(super) fn filter(
    ffmpeg: &Path,
    sandbox: &Sandbox,
    record: &Value,
    cancelled: &AtomicBool,
) -> Result<String> {
    let source = engine_path(&sandbox.output.join("recording.avi"));
    let probe = sandbox.output.join("recording-source.png");
    let output = sandbox.execute(
        ffmpeg,
        &[
            "-nostdin",
            "-y",
            "-i",
            &source,
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-update",
            "1",
            &engine_path(&probe),
        ],
        60,
        cancelled,
    )?;
    fs::write(sandbox.output.join("recording-probe.log"), &output.text)?;
    ensure!(output.code == 0, "Cannot inspect Movie Maker output");
    let dimensions = image::image_dimensions(&probe)?;
    presentation_filter(record, dimensions)
}

fn presentation_filter(record: &Value, source: (u32, u32)) -> Result<String> {
    let layout: Presentation = serde_json::from_value(record["presentation"].clone())
        .context("Missing captured window presentation")?;
    let [width, height] = layout.window;
    let [render_width, render_height] = layout.viewport;
    let [x, y, content_width, content_height] = layout.content;
    ensure!(
        width > 0
            && height > 0
            && render_width > 0
            && render_height > 0
            && content_width > 0
            && content_height > 0
            && u64::from(x) + u64::from(content_width) <= u64::from(width)
            && u64::from(y) + u64::from(content_height) <= u64::from(height),
        "Invalid captured window presentation"
    );
    // Movie Maker versions can crop when their configured aspect differs from
    // the viewport. Padding cannot recover missing pixels: reject that evidence.
    ensure!(
        u64::from(source.0) * u64::from(render_height)
            == u64::from(source.1) * u64::from(render_width),
        "Movie Maker aspect differs from captured viewport; recording may be cropped"
    );
    Ok(format!(
        "scale={content_width}:{content_height}:flags=bilinear,pad={width}:{height}:{x}:{y}:color=black,setsar=1"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn video_presentation_preserves_bars_and_rejects_cropped_movies() -> Result<()> {
        let record = json!({"presentation": {
            "window": [960, 720], "viewport": [960, 540], "content": [0, 90, 960, 540]
        }});
        assert_eq!(
            presentation_filter(&record, (1280, 720))?,
            "scale=960:540:flags=bilinear,pad=960:720:0:90:color=black,setsar=1"
        );
        assert!(presentation_filter(&record, (960, 720)).is_err());
        Ok(())
    }
}
