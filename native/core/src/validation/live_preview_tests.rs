use super::{
    repository,
    service::{Service, Tools},
    test_support::Fixture,
};
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn wait_frame(service: &Service, input: &Value, revision: u64) -> Result<Value> {
    let start = Instant::now();
    loop {
        let value = service.preview("validation.preview.read", input)?;
        ensure!(
            value["status"] != "closed",
            "Preview closed: {}",
            value["error"]
        );
        if value["frame"]["revision"].as_u64() == Some(revision) {
            return Ok(value);
        }
        ensure!(
            start.elapsed() < Duration::from_secs(120),
            "Frame timed out: {}",
            value["error"]
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
#[ignore = "requires BEAVER_TEST_GODOT and a graphical Godot engine"]
fn live_preview_real_camera_close_and_lease() -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_GODOT").context("Set BEAVER_TEST_GODOT")?;
    let mut f = Fixture::new()?;
    fs::remove_file(f.project.join("export_presets.cfg"))?;
    fs::write(
        f.project.join("scene.tscn"),
        concat!(
        "[gd_scene load_steps=3 format=3]\n",
        "[sub_resource type=\"StandardMaterial3D\" id=\"Mat\"]\n",
        "shading_mode = 0\nalbedo_color = Color(0.1, 0.8, 0.2, 1)\n",
        "[sub_resource type=\"BoxMesh\" id=\"Box\"]\nmaterial = SubResource(\"Mat\")\n",
        "[node name=\"Scene\" type=\"Node3D\"]\n",
        "[node name=\"Box\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\n",
        "[node name=\"Occluded\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\nposition = Vector3(0, 0, -2)\n",
        "[node name=\"Outside\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\nposition = Vector3(100, 0, 0)\n",
        "[node name=\"BehindCamera\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\nposition = Vector3(0, 0, 8)\n",
        "[node name=\"BeyondFar\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\nposition = Vector3(0, 0, -10000)\n",
        "[node name=\"Camera\" type=\"Camera3D\" parent=\".\"]\n",
        "position = Vector3(0, 0, 5)\ncurrent = true\n"
    ),
    )?;
    {
        use std::io::Write;
        let mut scene = fs::OpenOptions::new()
            .append(true)
            .open(f.project.join("scene.tscn"))?;
        for index in 0..35 {
            writeln!(scene, "[node name=\"Crowd{index}\" type=\"MeshInstance3D\" parent=\".\"]\nmesh = SubResource(\"Box\")\nposition = Vector3(1.5, 0, 0)\nscale = Vector3(0.1, 0.1, 0.1)")?;
        }
    }
    let mut flow = f.flow()?;
    flow.definition.entry = "scene.tscn".into();
    flow.definition.config.width = 960;
    flow.definition.config.height = 540;
    let mut run = f.run(Some(flow))?;
    run.kind = "objectPreview".into();
    super::runner::execute(
        &f.files,
        std::path::Path::new(&engine),
        None,
        &mut run,
        &std::sync::atomic::AtomicBool::new(false),
        |_| {},
    );
    ensure!(
        run.status == "completed",
        "Source capture failed: {:?}",
        run
    );
    f.save(&run)?;
    f.store.put("objectScenePreviewSource", &run.id, &json!({
        "runId":run.id,"sourceDigest":run.snapshot_id,"projectConfig":"frozen",
        "target":{"projectId":"p","objectId":"object","versionId":"version","path":"scene.tscn","sha256":run.snapshot["scene.tscn"]}
    }))?;
    let original = serde_json::to_value(&run)?;
    fs::remove_file(f.project.join("scene.tscn"))?;
    let store = Arc::new(Mutex::new(f.store));
    let files = Arc::new(f.files);
    let service = Service::start(
        store.clone(),
        files.clone(),
        Arc::new(move |_| {
            Ok(Tools {
                engine: engine.clone().into(),
                ffmpeg: None,
            })
        }),
        Arc::new(|| {}),
    )?;
    let result = (|| -> Result<()> {
        let input = json!({"projectId":"p","runId":run.id,"requestId":"live"});
        let opened = service.preview("validation.preview.open", &input)?;
        let id = opened["sessionId"].clone();
        assert_eq!(
            service.preview("validation.preview.open", &input)?["sessionId"],
            id
        );
        let other = json!({"projectId":"p","runId":run.id,"requestId":"other"});
        assert!(service
            .preview("validation.preview.open", &other)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_SESSION_LIMIT"));
        let read = json!({"projectId":"p","sessionId":id});
        let first = wait_frame(&service, &read, 0)?;
        assert_eq!(first["frame"]["width"], 960);
        let mut view = read.clone();
        view["revision"] = json!(1);
        view["width"] = json!(960);
        view["height"] = json!(540);
        view["camera"] = json!({"yaw":25,"pitch":10,"panX":0.1,"panY":0,"zoom":-0.2});
        service.preview("validation.preview.view", &view)?;
        let moved = wait_frame(&service, &read, 1)?;
        assert_ne!(first["frame"]["sha256"], moved["frame"]["sha256"]);
        assert_ne!(
            first["frame"]["camera"]["transform"],
            moved["frame"]["camera"]["transform"]
        );
        assert_eq!(moved["snapshotId"], run.snapshot_id);
        let mut proof = vec![("initial", first), ("moved", moved.clone())];
        for (revision, width, height, name) in [
            (2, 1280, 720, "720p"),
            (3, 1920, 1080, "1080p"),
            (4, 960, 540, "540p"),
        ] {
            view["revision"] = json!(revision);
            view["width"] = json!(width);
            view["height"] = json!(height);
            service.preview("validation.preview.view", &view)?;
            service.preview("validation.preview.view", &view)?;
            let resized = wait_frame(&service, &read, revision)?;
            assert_eq!(resized["sessionId"], id);
            assert_eq!(resized["frame"]["width"], width);
            assert_eq!(resized["frame"]["height"], height);
            assert_eq!(
                resized["frame"]["camera"]["transform"],
                moved["frame"]["camera"]["transform"]
            );
            proof.push((name, resized));
        }
        let mut invalid = view.clone();
        let capture = json!({"projectId":"p","runId":run.id,"sessionId":id,"revision":4,"requestId":"save-frame"});
        let saved_frame = service.preview("validation.preview.capture", &capture)?;
        assert_eq!(saved_frame["frame"]["revision"], 4);
        assert_eq!(saved_frame["source"]["target"]["versionId"], "version");
        assert_eq!(
            service.preview("validation.preview.capture", &capture)?,
            saved_frame
        );
        let saved_input = json!({"projectId":"p","runId":run.id});
        assert_eq!(
            service.preview("validation.preview.saved", &saved_input)?["frames"],
            json!([saved_frame.clone()])
        );
        let mut wrong_capture = capture.clone();
        wrong_capture["projectId"] = json!("another");
        assert!(service
            .preview("validation.preview.capture", &wrong_capture)
            .is_err());
        invalid["width"] = json!(1280);
        assert!(service
            .preview("validation.preview.view", &invalid)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_RESOLUTION_INVALID"));
        invalid["height"] = json!(720);
        assert!(service
            .preview("validation.preview.view", &invalid)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_REVISION_CONFLICT"));
        let mut stale = view.clone();
        stale["revision"] = json!(0);
        assert!(service.preview("validation.preview.view", &stale).is_err());
        let mut wrong = read.clone();
        wrong["projectId"] = json!("another");
        assert!(service.preview("validation.preview.read", &wrong).is_err());
        view["revision"] = json!(5);
        view["frozen"] = json!(true);
        view["camera"] = json!({"yaw":0,"pitch":0,"panX":0,"panY":0,"zoom":0});
        service.preview("validation.preview.view", &view)?;
        service.preview("validation.preview.view", &view)?;
        let frozen = wait_frame(&service, &read, 5)?;
        assert_eq!(frozen["frame"]["frozen"], true);
        std::thread::sleep(Duration::from_secs(2));
        assert_eq!(
            service.preview("validation.preview.read", &read)?["frame"],
            frozen["frame"]
        );
        let selection =
            super::live_preview_pick_tests::exercise(&service, &read, &frozen["frame"])?;
        let frozen = service.preview("validation.preview.read", &read)?;
        let frozen_capture = json!({"projectId":"p","runId":run.id,"sessionId":id,"revision":5,"requestId":"frozen-save","selection":selection});
        let archived = service.preview("validation.preview.capture", &frozen_capture)?;
        assert_eq!(archived["frame"], frozen["frame"]);
        assert_eq!(archived["selection"], selection);
        proof.push(("frozen", frozen.clone()));
        view["revision"] = json!(6);
        view["frozen"] = json!(false);
        service.preview("validation.preview.view", &view)?;
        let resumed = wait_frame(&service, &read, 6)?;
        assert_eq!(resumed["frame"]["frozen"], false);
        assert!(resumed["frame"]["sequence"].as_u64() > frozen["frame"]["sequence"].as_u64());
        proof.push(("resumed", resumed));
        view["revision"] = json!(7);
        view["frozen"] = json!(true);
        service.preview("validation.preview.view", &view)?;
        wait_frame(&service, &read, 7)?;
        if let Ok(output) = std::env::var("BEAVER_PREVIEW_PROOF") {
            fs::create_dir_all(&output)?;
            for (name, mut value) in proof {
                let data = value["frame"]["dataUrl"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("data:image/png;base64,")
                    .unwrap();
                fs::write(
                    std::path::Path::new(&output).join(format!("{name}.png")),
                    STANDARD.decode(data)?,
                )?;
                value["frame"].as_object_mut().unwrap().remove("dataUrl");
                fs::write(
                    std::path::Path::new(&output).join(format!("{name}.json")),
                    serde_json::to_vec_pretty(&value)?,
                )?;
            }
        }
        service.preview("validation.preview.close", &read)?;
        assert_eq!(
            service.preview("validation.preview.capture", &capture)?,
            saved_frame
        );
        let start = Instant::now();
        while service.preview("validation.preview.read", &read)?["status"] != "closed" {
            ensure!(start.elapsed() < Duration::from_secs(10), "Close timed out");
            std::thread::sleep(Duration::from_millis(100));
        }
        let reopened = loop {
            match service.preview("validation.preview.open", &other) {
                Ok(value) => break value,
                Err(error) => {
                    ensure!(start.elapsed() < Duration::from_secs(10), "{error}");
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        };
        let second = json!({"projectId":"p","sessionId":reopened["sessionId"]});
        wait_frame(&service, &second, 0)?;
        service.preview("validation.preview.close", &read)?;
        assert_eq!(
            service.preview("validation.preview.read", &second)?["status"],
            "ready"
        );
        std::thread::sleep(Duration::from_secs(17));
        let lease_shutdown = Instant::now();
        while service.preview("validation.preview.read", &second)?["status"] != "closed" {
            ensure!(
                lease_shutdown.elapsed() < Duration::from_secs(15),
                "Expired viewer did not close"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        let saved: super::model::Run =
            repository::get(&store.lock().unwrap(), "validationRun", &run.id)?;
        assert_eq!(serde_json::to_value(saved)?, original);
        service.shutdown()?;
        let reopened_store = Arc::new(Mutex::new(crate::store::Store::open(
            &f._temp.path().join("data"),
        )?));
        let reader = Service::start(
            reopened_store.clone(),
            files.clone(),
            Arc::new(|_| anyhow::bail!("Saved replay must not launch Godot")),
            Arc::new(|| {}),
        )?;
        assert_eq!(
            reader.preview("validation.preview.saved", &saved_input)?["frames"],
            json!([saved_frame.clone(), archived])
        );
        let mut corrupt = saved_frame;
        corrupt["frame"]["camera"]["near"] = json!(123);
        reopened_store
            .lock()
            .unwrap()
            .put("objectPreviewFrames", &run.id, &json!([corrupt]))?;
        assert!(reader
            .preview("validation.preview.saved", &saved_input)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_SAVED_DIGEST_MISMATCH"));
        reader.shutdown()?;
        Ok(())
    })();
    service.shutdown()?;
    result
}
