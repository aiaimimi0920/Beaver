use super::attempt_fixture::{attempts, fixture, start, workspace};
use crate::{
    object_attempt::{self, State},
    object_attempt_scene_preview as preview,
};
use anyhow::Result;
use serde_json::json;
use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "requires BEAVER_TEST_GODOT and a graphical Godot editor"]
fn attempt_scene_preview_real_hd_capture() -> Result<()> {
    use crate::validation::{
        self,
        model::Run,
        repository,
        service::{Service, Tools},
    };
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    let engine = std::env::var("BEAVER_TEST_GODOT")?;
    let f = fixture()?;
    let lease = start(&f)?;
    let root = workspace(&f, lease.record())?;
    std::fs::write(
        root.join("scene.tscn"),
        concat!(
            "[gd_scene format=3]\n\n[node name=\"Frozen\" type=\"ColorRect\"]\n",
            "offset_right = 1920.0\noffset_bottom = 1080.0\ncolor = Color(0.1, 0.7, 0.3, 1)\n"
        ),
    )?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    let snapshot = attempt.output.as_ref().unwrap();
    let target = json!({"projectId":"project-1","runId":attempt.preparation.run.id,
        "attemptId":attempt.id,"checkpoint":"output","path":"scene.tscn","sha256":snapshot["scene.tscn"]});
    let parsed = serde_json::from_value(target.clone())?;
    std::fs::remove_file(root.join("scene.tscn"))?;
    preview::enqueue(
        &f.runtime,
        &json!({"projectId":"project-1","requestId":"hd-real","target":target,"resolution":"720p"}),
    )?;
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
    let outcome = (|| -> Result<_> {
        let start = Instant::now();
        loop {
            let result = preview::latest(&f.runtime, &parsed)?;
            if !["queued", "running"].contains(&result["run"]["status"].as_str().unwrap()) {
                return Ok(result);
            }
            anyhow::ensure!(
                start.elapsed() < Duration::from_secs(180),
                "Preview timed out"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    })();
    service.shutdown()?;
    let result = outcome?;
    assert_eq!(result["run"]["status"], "completed", "{result}");
    assert!(result["integrityError"].is_null(), "{result}");
    assert_eq!(result["resolution"], json!({"width":1280,"height":720}));
    let run: Run = repository::get(
        &f.runtime.store().lock().unwrap(),
        "validationRun",
        result["run"]["id"].as_str().unwrap(),
    )?;
    let path = validation::evidence::media_path(&f.runtime.files(), &run, &run.evidence[0].id)?;
    let image = image::open(&path)?.to_rgb8();
    assert_eq!(image.dimensions(), (1280, 720));
    let pixel = image.get_pixel(640, 360).0;
    assert!(pixel[1] > pixel[0] + 50 && pixel[1] > pixel[2] + 50);
    if let Ok(output) = std::env::var("BEAVER_PREVIEW_PROOF") {
        std::fs::create_dir_all(&output)?;
        std::fs::copy(path, std::path::Path::new(&output).join("frozen-scene.png"))?;
        std::fs::write(
            std::path::Path::new(&output).join("result.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
    }
    Ok(())
}

#[test]
fn attempt_scene_preview_freezes_output_and_separates_checkpoints() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    let root = workspace(&f, lease.record())?;
    std::fs::write(root.join("scene.tscn"), "[gd_scene format=3]")?;
    std::fs::write(root.join("dependency.txt"), "frozen dependency")?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    let snapshot = attempt.output.as_ref().unwrap();
    let target = json!({"projectId":"project-1","runId":attempt.preparation.run.id,
        "attemptId":attempt.id,"checkpoint":"output","path":"scene.tscn","sha256":snapshot["scene.tscn"]});
    std::fs::remove_file(root.join("scene.tscn"))?;
    let input = json!({"projectId":"project-1","requestId":"preview-output","target":target});
    let receipt = preview::enqueue(&f.runtime, &input)?;
    assert_eq!(preview::enqueue(&f.runtime, &input)?, receipt);
    let saved = preview::latest(&f.runtime, &serde_json::from_value(target.clone())?)?;
    assert_eq!(saved["run"]["id"], receipt["runId"]);
    assert_eq!(saved["target"], target);
    let mut other = target.clone();
    other["checkpoint"] = json!("input");
    assert!(preview::latest(&f.runtime, &serde_json::from_value(other.clone())?)?.is_null());
    assert!(preview::enqueue(
        &f.runtime,
        &json!({"projectId":"project-1","requestId":"input","target":other})
    )
    .is_err());
    other = target.clone();
    other["runId"] = json!("foreign");
    assert!(preview::enqueue(
        &f.runtime,
        &json!({"projectId":"project-1","requestId":"foreign","target":other})
    )
    .is_err());
    // A new checkpoint key must verify dependencies, not just the selected scene.
    let mut changed = attempt.clone();
    changed.input = snapshot.clone();
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_attempt", &attempt.id, &changed)?;
    std::fs::write(
        f.runtime.files().blob(&snapshot["dependency.txt"])?,
        "corrupt",
    )?;
    other = target;
    other["checkpoint"] = json!("input");
    assert!(preview::enqueue(
        &f.runtime,
        &json!({"projectId":"project-1","requestId":"corrupt","target":other})
    )
    .unwrap_err()
    .to_string()
    .contains("HASH_MISMATCH"));
    Ok(())
}
