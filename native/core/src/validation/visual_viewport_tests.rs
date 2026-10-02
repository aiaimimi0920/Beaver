use super::{repository, runner, test_support::Fixture};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::{fs, path::Path, sync::atomic::AtomicBool, time::Duration};

#[test]
#[ignore = "requires BEAVER_TEST_GODOT and a graphical Godot engine"]
fn visual_preserves_canvas_layout_and_clicks_at_capture_resolutions() -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_GODOT").context("Set BEAVER_TEST_GODOT")?;
    let ffmpeg = std::env::var("BEAVER_TEST_FFMPEG").context("Set BEAVER_TEST_FFMPEG")?;
    let mut f = Fixture::new()?;
    fs::write(f.project.join("export_presets.cfg"), "")?;
    fs::write(
        f.project.join("viewport_fixture.gd"),
        include_str!("../../../../resources/validation/tests/viewport_fixture.gd"),
    )?;
    fs::write(
        f.project.join("viewport_fixture.tscn"),
        concat!(
            "[gd_scene load_steps=2 format=3]\n",
            "[ext_resource type=\"Script\" path=\"res://viewport_fixture.gd\" id=\"1\"]\n",
            "[node name=\"Fixture\" type=\"Node2D\"]\nscript = ExtResource(\"1\")\n"
        ),
    )?;
    let source_flow = f.flow()?;
    for (mode, width, height, video) in [
        ("canvas_items", 960, 540, false),
        ("canvas_items", 960, 720, true),
        ("canvas_items", 1280, 720, false),
        ("viewport", 960, 540, false),
        ("viewport", 960, 720, true),
        ("viewport", 1280, 720, false),
    ] {
        fs::write(f.project.join("project.godot"), format!(
            "config_version=5\n[display]\nwindow/size/viewport_width=1280\nwindow/size/viewport_height=720\nwindow/stretch/mode=\"{mode}\"\nwindow/stretch/aspect=\"keep\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n"
        ))?;
        let original = f.files.capture(&f.project)?;
        let mut flow = source_flow.clone();
        flow.definition.video = video;
        flow.definition.entry = "viewport_fixture.tscn".into();
        flow.definition.config.width = width;
        flow.definition.config.height = height;
        flow.definition.steps = serde_json::from_value(json!([
            {"id":"before","kind":"capture"},
            {"id":"click","kind":"click","node":"/root/Fixture/UI/Button"},
            {"id":"clicked","kind":"waitFor","node":"/root/Fixture","property":"clicks","equals":1,"timeout":15},
            {"id":"after","kind":"capture"}
        ]))?;
        let mut run = f.run(Some(flow))?;
        if mode == "viewport" && width == 1280 {
            run.kind = "objectPreview".into();
        }
        runner::execute(
            &f.files,
            Path::new(&engine),
            Some(Path::new(&ffmpeg)),
            &mut run,
            &AtomicBool::new(false),
            |_| {},
        );
        let directory = repository::run_dir(&f.files, &run.id)?;
        fs::write(
            directory.join("test-run.json"),
            serde_json::to_vec_pretty(&run)?,
        )?;
        if run.status != "completed" {
            let retained = f._temp.keep();
            bail!(
                "Viewport {mode} {width}x{height}: {:?}; evidence {}; fixture {}",
                run.error,
                directory.display(),
                retained.display()
            );
        }
        assert_eq!(run.completed_steps, 4);
        let frames: Vec<_> = run.evidence.iter().filter(|e| e.kind == "image").collect();
        assert_eq!(frames.len(), if video { 4 } else { 2 });
        for frame in frames {
            let image = image::open(directory.join(&frame.file))?.to_rgb8();
            check_frame(&image, width, height, 0);
        }
        if video {
            assert_eq!(run.evidence.iter().filter(|e| e.kind == "video").count(), 1);
            let frame_path = directory.join("decoded-video.png");
            let output = crate::process::run(
                Path::new(&ffmpeg),
                &[
                    "-nostdin",
                    "-y",
                    "-sseof",
                    "-0.05",
                    "-i",
                    &directory.join("recording.webm").to_string_lossy(),
                    "-frames:v",
                    "1",
                    "-update",
                    "1",
                    &frame_path.to_string_lossy(),
                ],
                Some(&directory),
                Duration::from_secs(30),
            )?;
            anyhow::ensure!(output.code == 0, "{}", output.text);
            check_frame(&image::open(frame_path)?.to_rgb8(), width, height, 10);
        }
        assert_eq!(f.files.capture(&f.project)?, original);
        println!("Viewport {mode} {width}x{height}: steps, button, marker, bars and video={video} passed");
    }
    println!("Viewport evidence retained: {}", f._temp.keep().display());
    Ok(())
}

fn check_frame(image: &image::RgbImage, width: u32, height: u32, tolerance: u8) {
    assert_eq!(image.dimensions(), (width, height));
    // The bottom-right marker is clipped if the runner replaces the design canvas.
    let scale = (width as f32 / 1280.0).min(height as f32 / 720.0);
    let x = ((width as f32 - 1280.0 * scale) / 2.0 + 1210.0 * scale) as u32;
    let y = ((height as f32 - 720.0 * scale) / 2.0 + 670.0 * scale) as u32;
    let color = image.get_pixel(x, y).0;
    assert!(
        color[0] <= tolerance && color[1] >= 255 - tolerance && color[2] <= tolerance,
        "Bottom-right marker is missing: {color:?}"
    );
    if height == 720 && width == 960 {
        for y in [10, 710] {
            assert!(image.get_pixel(480, y).0.iter().all(|c| *c <= tolerance));
        }
    }
}
