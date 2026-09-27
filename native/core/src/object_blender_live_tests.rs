use super::*;
use serde_json::Value;
#[path = "object_blender_pick_tests.rs"]
mod picking;

fn wait(service: &Service, input: &Value, revision: u64) -> Result<Value> {
    let start = Instant::now();
    loop {
        let value = service.preview("validation.preview.read", input)?;
        ensure!(
            value["status"] != "closed",
            "Preview closed: {}",
            value["error"]
        );
        if value["frame"]["revision"].as_u64() == Some(revision) {
            return Ok(value["frame"].clone());
        }
        ensure!(
            start.elapsed() < Duration::from_secs(120),
            "Frame timeout: {value}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
#[ignore = "requires BEAVER_TEST_BLENDER and a graphical Blender render device"]
fn object_blender_live_camera_archive_and_lease() -> Result<()> {
    exercise("automatic")
}

#[test]
#[ignore = "requires BEAVER_TEST_BLENDER and a graphical Blender render device"]
fn object_blender_real_mesh_picking_and_archive() -> Result<()> {
    exercise("picking")
}

fn exercise(mode: &str) -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_BLENDER").context("Set BEAVER_TEST_BLENDER")?;
    let (f, target) = capture(&engine, mode)?;
    let receipt = enqueue(
        &f.runtime,
        &json!({"projectId":target.project_id,"target":target,"requestId":"render","resolution":"540p"}),
    )?;
    let service = Service::start(
        f.runtime.store(),
        f.runtime.files(),
        Arc::new(move |context| {
            ensure!(context.engine == "blender", "Wrong live engine selected");
            Ok(Tools {
                engine: engine.clone().into(),
                ffmpeg: None,
            })
        }),
        Arc::new(|| {}),
    )?;
    let result = (|| -> Result<()> {
        let start = Instant::now();
        loop {
            let value = latest(&f.runtime, &target)?;
            if value["run"]["status"] == "completed" {
                break;
            }
            ensure!(value["run"]["status"] != "failed", "{value}");
            ensure!(
                start.elapsed() < Duration::from_secs(180),
                "Capture timeout"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        let open =
            json!({"projectId":target.project_id,"runId":receipt["runId"],"requestId":"live"});
        let opened = service.preview("validation.preview.open", &open)?;
        assert_eq!(
            service.preview("validation.preview.open", &open)?["sessionId"],
            opened["sessionId"]
        );
        let mut other = open.clone();
        other["requestId"] = json!("other");
        assert!(service
            .preview("validation.preview.open", &other)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_SESSION_LIMIT"));
        let read = json!({"projectId":target.project_id,"sessionId":opened["sessionId"]});
        let first = wait(&service, &read, 0)?;
        assert_eq!(first["width"], 960);
        assert_eq!(first["picking"]["capability"], "frozen-static-mesh-ray");
        let mut view = read.clone();
        view["revision"] = json!(1);
        view["width"] = json!(960);
        view["height"] = json!(540);
        view["camera"] = json!({"yaw":25,"pitch":10,"panX":0.1,"panY":0.05,"zoom":-0.2});
        if mode == "picking" {
            // Deliberately distant clipping fixtures enlarge the orbit bounds.
            view["camera"] = json!({"yaw":0.025,"pitch":0.01,"panX":0,"panY":0,"zoom":0});
        }
        service.preview("validation.preview.view", &view)?;
        let moved = wait(&service, &read, 1)?;
        assert_ne!(moved["sha256"], first["sha256"]);
        assert_ne!(moved["camera"]["transform"], first["camera"]["transform"]);
        {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let bytes = STANDARD.decode(
                moved["dataUrl"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("data:image/png;base64,")
                    .unwrap(),
            )?;
            let image = image::load_from_memory(&bytes)?.to_rgb8();
            let green = image
                .pixels()
                .filter(|p| {
                    u16::from(p[1]) > u16::from(p[0]) + 30 && u16::from(p[1]) > u16::from(p[2]) + 30
                })
                .count();
            assert!(
                green > 1000,
                "Orbit must keep the fixture geometry visible: {green}"
            );
        }
        for (revision, width, height) in [(2, 1280, 720), (3, 1920, 1080), (4, 960, 540)] {
            view["revision"] = json!(revision);
            view["width"] = json!(width);
            view["height"] = json!(height);
            service.preview("validation.preview.view", &view)?;
            let resized = wait(&service, &read, revision)?;
            assert_eq!(resized["width"], width);
            assert_eq!(resized["height"], height);
            assert_eq!(resized["camera"]["transform"], moved["camera"]["transform"]);
        }
        view["revision"] = json!(5);
        view["camera"] = json!({"yaw":0,"pitch":0,"panX":0,"panY":0,"zoom":0});
        service.preview("validation.preview.view", &view)?;
        let reset = wait(&service, &read, 5)?;
        assert_eq!(reset["camera"], first["camera"]);
        // Blender PNG metadata includes Date and RenderTime; compare decoded pixels.
        {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let pixels = |frame: &Value| -> Result<_> {
                Ok(image::load_from_memory(
                    &STANDARD.decode(
                        frame["dataUrl"]
                            .as_str()
                            .unwrap()
                            .strip_prefix("data:image/png;base64,")
                            .unwrap(),
                    )?,
                )?
                .to_rgba8())
            };
            assert_eq!(pixels(&reset)?, pixels(&first)?);
        }
        view["revision"] = json!(6);
        view["frozen"] = json!(true);
        service.preview("validation.preview.view", &view)?;
        let frozen = wait(&service, &read, 6)?;
        assert_eq!(frozen["frozen"], true);
        assert_eq!(frozen["sha256"], reset["sha256"]);
        std::thread::sleep(Duration::from_secs(2));
        assert_eq!(
            service.preview("validation.preview.read", &read)?["frame"],
            frozen
        );
        let mut selection = json!({"kind":"image-regions","sequence":frozen["sequence"],"sha256":frozen["sha256"],
            "regions":[{"x":0.2,"y":0.2,"width":0.5,"height":0.5,"prompt":"Preserve the green cube"}],
            "prompt":"Historical Blender image only","coordinateSpace":"normalized-image","hitCapability":"unavailable"});
        if mode == "picking" {
            selection = picking::exercise(&service, &read, &frozen)?;
        }
        let frozen = service.preview("validation.preview.read", &read)?["frame"].clone();
        let capture = json!({"projectId":target.project_id,"runId":receipt["runId"],"sessionId":opened["sessionId"],
            "revision":6,"requestId":"archive","selection":selection});
        let archived = service.preview("validation.preview.capture", &capture)?;
        assert_eq!(archived["frame"], frozen);
        assert_eq!(archived["selection"], selection);
        assert_eq!(archived["snapshotId"], opened["snapshotId"]);
        assert_eq!(
            service.preview("validation.preview.capture", &capture)?,
            archived
        );
        if let Ok(output) = std::env::var("BEAVER_PREVIEW_PROOF") {
            use base64::{engine::general_purpose::STANDARD, Engine};
            fs::create_dir_all(&output)?;
            for (name, frame) in [("initial", &first), ("moved", &moved), ("frozen", &frozen)] {
                fs::write(
                    Path::new(&output).join(format!("blender-live-{name}.png")),
                    STANDARD.decode(
                        frame["dataUrl"]
                            .as_str()
                            .unwrap()
                            .strip_prefix("data:image/png;base64,")
                            .unwrap(),
                    )?,
                )?;
            }
            fs::write(
                Path::new(&output).join("blender-live-archive.json"),
                serde_json::to_vec_pretty(&archived)?,
            )?;
        }
        service.preview("validation.preview.close", &read)?;
        let start = Instant::now();
        while service.preview("validation.preview.read", &read)?["status"] != "closed" {
            ensure!(start.elapsed() < Duration::from_secs(10), "Close timeout");
            std::thread::sleep(Duration::from_millis(100));
        }
        let reopened = service.preview("validation.preview.open", &other)?;
        let second = json!({"projectId":target.project_id,"sessionId":reopened["sessionId"]});
        wait(&service, &second, 0)?;
        std::thread::sleep(Duration::from_secs(18));
        assert_eq!(
            service.preview("validation.preview.read", &second)?["status"],
            "closed"
        );
        service.shutdown()?;
        let reader = Service::start(
            f.runtime.store(),
            f.runtime.files(),
            Arc::new(|_| anyhow::bail!("Archive must not launch an engine")),
            Arc::new(|| {}),
        )?;
        let saved = reader.preview(
            "validation.preview.saved",
            &json!({"projectId":target.project_id,"runId":receipt["runId"]}),
        )?;
        assert_eq!(saved["frames"], json!([archived]));
        reader.shutdown()?;
        Ok(())
    })();
    service.shutdown()?;
    result
}
