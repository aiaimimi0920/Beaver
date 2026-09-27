use super::*;
use crate::{
    object_catalog::ObjectFile,
    object_catalog_test_fixture::{capture_request, Fixture},
    object_registration, object_version_capture,
    validation::{
        self,
        service::{Service, Tools},
    },
};
use serde_json::json;
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

fn fixture() -> Result<(Fixture, Target)> {
    let f = Fixture::new()?;
    fs::write(
        f.temp.path().join("scene.tscn"),
        concat!(
            "[gd_scene format=3]\n\n[node name=\"Frozen\" type=\"ColorRect\"]\n",
            "offset_right = 960.0\noffset_bottom = 540.0\ncolor = Color(0.1, 0.7, 0.3, 1)\n"
        ),
    )?;
    let mut registration = f.request("register-scene");
    registration.files.push(ObjectFile {
        path: "scene.tscn".into(),
        role: "source".into(),
    });
    let object = object_registration::register(&f.runtime, &registration)?.object;
    let object =
        object_version_capture::capture(&f.runtime, &capture_request(&object, "capture-scene"))?
            .object;
    let version = &object.versions[0];
    let target = Target {
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        version_id: version.version_id.clone(),
        path: "scene.tscn".into(),
        sha256: version.manifest["files"][0]["sha256"]
            .as_str()
            .unwrap()
            .into(),
    };
    fs::remove_file(f.temp.path().join("scene.tscn"))?;
    Ok((f, target))
}

fn input(target: &Target, id: &str) -> Value {
    json!({"projectId":target.project_id,"target":target,"requestId":id})
}

#[test]
fn object_scene_preview_freezes_replays_and_recovers_without_acceptance() -> Result<()> {
    let (f, target) = fixture()?;
    let request = input(&target, "render");
    let receipt = enqueue(&f.runtime, &request)?;
    assert_eq!(enqueue(&f.runtime, &request)?, receipt);
    assert_eq!(
        enqueue(&f.runtime, &input(&target, "duplicate-active"))?,
        receipt
    );
    let result = latest(&f.runtime, &target)?;
    assert_eq!(result["run"]["status"], "queued");
    assert_eq!(result["projectConfig"], "minimal-v1");
    let handle = f.runtime.store();
    let mut store = handle.lock().unwrap();
    assert!(operations::list(&store, &target.project_id)?["runs"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(repository::flows(&store, &target.project_id)?.is_empty());
    let mut rerun =
        json!({"projectId":target.project_id,"requestId":"wrong-rerun","runId":receipt["runId"]});
    let error = operations::enqueue(
        &mut store,
        &f.runtime.files(),
        "validation.run.rerun",
        &rerun,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("frozen object preview operation"),
        "{error:#}"
    );
    rerun["snapshotId"] = result["run"]["snapshotId"].clone();
    assert!(validation::confirmation::confirm(
        &f.runtime.files(),
        &mut store,
        receipt["runId"].as_str().unwrap(),
        rerun["snapshotId"].as_str().unwrap(),
        &[],
        "confirm",
        "ui"
    )
    .is_err());
    repository::recover(&store)?;
    drop(store);
    assert_eq!(latest(&f.runtime, &target)?["run"]["status"], "interrupted");
    assert_ne!(enqueue(&f.runtime, &input(&target, "new-render"))?, receipt);
    Ok(())
}

#[test]
fn object_scene_preview_rejects_forged_member_and_corrupt_frozen_blob() -> Result<()> {
    let (f, target) = fixture()?;
    let mut wrong = target.clone();
    wrong.sha256 = "0".repeat(64);
    assert!(enqueue(&f.runtime, &input(&wrong, "wrong")).is_err());
    fs::write(f.runtime.files().blob(&target.sha256)?, "corrupt")?;
    assert!(enqueue(&f.runtime, &input(&target, "corrupt")).is_err());
    assert_eq!(f.count("validationRun")?, 0);
    Ok(())
}

#[test]
fn scene_preview_resolution_preserves_receipts_and_previous_runs() -> Result<()> {
    let (f, target) = fixture()?;
    let legacy = input(&target, "legacy");
    let first = enqueue(&f.runtime, &legacy)?;
    assert_eq!(
        latest(&f.runtime, &target)?["resolution"],
        json!({"width":960,"height":540})
    );
    let mut hd = input(&target, "hd");
    hd["resolution"] = json!("720p");
    assert!(enqueue(&f.runtime, &hd)
        .unwrap_err()
        .to_string()
        .contains("PREVIEW_RESOLUTION_ACTIVE_CONFLICT"));
    repository::recover(&f.runtime.store().lock().unwrap())?;
    let second = enqueue(&f.runtime, &hd)?;
    assert_ne!(first, second);
    assert_eq!(enqueue(&f.runtime, &legacy)?, first);
    assert_eq!(
        latest(&f.runtime, &target)?["resolution"],
        json!({"width":1280,"height":720})
    );
    let old: Run = repository::get(
        &f.runtime.store().lock().unwrap(),
        "validationRun",
        first["runId"].as_str().unwrap(),
    )?;
    assert_eq!(old.flow.unwrap().definition.config.width, 960);
    hd["resolution"] = json!("1080p");
    assert!(enqueue(&f.runtime, &hd).is_err());
    hd["resolution"] = json!("4k");
    assert!(enqueue(&f.runtime, &hd).is_err());
    Ok(())
}

#[test]
#[ignore = "requires BEAVER_TEST_GODOT and a real graphical Godot editor"]
fn object_scene_preview_real_godot_queue_capture() -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_GODOT").context("Set BEAVER_TEST_GODOT")?;
    let (f, target) = fixture()?;
    enqueue(&f.runtime, &input(&target, "real-render"))?;
    let service = Service::start(
        f.runtime.store(),
        f.runtime.files(),
        Arc::new(move |_| {
            Ok(Tools {
                engine: engine.clone().into(),
                ffmpeg: None,
            })
        }),
        Arc::new(|| {}),
    )?;
    let start = Instant::now();
    let result = loop {
        let result = latest(&f.runtime, &target)?;
        if !["queued", "running"].contains(&result["run"]["status"].as_str().unwrap()) {
            break result;
        }
        ensure!(
            start.elapsed() < Duration::from_secs(180),
            "Preview timed out"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    service.shutdown()?;
    assert_eq!(result["run"]["status"], "completed", "{result}");
    assert!(result["integrityError"].is_null(), "{result}");
    let run: Run = repository::get(
        &f.runtime.store().lock().unwrap(),
        "validationRun",
        result["run"]["id"].as_str().unwrap(),
    )?;
    let path = validation::evidence::media_path(&f.runtime.files(), &run, &run.evidence[0].id)?;
    let image = image::open(&path)?.to_rgb8();
    assert_eq!(image.dimensions(), (960, 540));
    let pixel = image.get_pixel(480, 270).0;
    assert!(
        pixel[1] > pixel[0] + 50 && pixel[1] > pixel[2] + 50,
        "Expected frozen green scene: {pixel:?}"
    );
    if let Ok(output) = std::env::var("BEAVER_PREVIEW_PROOF") {
        fs::create_dir_all(&output)?;
        fs::copy(path, std::path::Path::new(&output).join("frozen-scene.png"))?;
        fs::write(
            std::path::Path::new(&output).join("result.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
    }
    Ok(())
}
use crate::validation::{model::Run, operations};
