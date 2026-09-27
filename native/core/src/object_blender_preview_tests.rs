use super::*;
#[path = "object_blender_live_tests.rs"]
mod live;
use crate::{
    object_catalog::ObjectFile,
    object_catalog_test_fixture::{capture_request, Fixture},
    object_registration, object_version_capture,
    validation::{
        self,
        model::Run,
        service::{Service, Tools},
    },
};
use serde_json::json;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

fn capture(engine: &str, mode: &str) -> Result<(Fixture, Target)> {
    let f = Fixture::new()?;
    let source = f.temp.path().join("scene.blend");
    let script = f.temp.path().join("fixture.py");
    fs::write(
        &script,
        r#"import bpy, sys
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_cube_add()
material = bpy.data.materials.new("Green")
material.diffuse_color = (0.02, 0.8, 0.04, 1)
bpy.context.object.data.materials.append(material)
mode, destination = sys.argv[-2:]
if mode in {"camera", "picking"}:
    bpy.ops.object.camera_add(location=(0, 0, 8))
    bpy.context.scene.camera = bpy.context.object
if mode == "picking":
    for name, location in [("Occluded", (0, 0, -3)), ("Outside", (100, 0, 0)),
                           ("BehindCamera", (0, 0, 12)), ("BeyondFar", (0, 0, -2000))]:
        bpy.ops.mesh.primitive_cube_add(location=location)
        bpy.context.object.name = name
    bpy.ops.mesh.primitive_cube_add(location=(3, 0, 0))
    bpy.context.object.name = "UnsupportedModifier"
    bpy.context.object.modifiers.new("Bevel", "BEVEL")
    for i in range(34):
        bpy.ops.mesh.primitive_cube_add(size=0.1, location=(1.5, 0, -i * 0.02))
        bpy.context.object.name = "Extra" + str(i)
if mode == "external":
    image = bpy.data.images.new("Unfrozen", 1, 1)
    image.filepath = "//missing.png"
    image.source = 'FILE'
    image.use_fake_user = True
bpy.ops.wm.save_as_mainfile(filepath=destination)
"#,
    )?;
    let result = crate::process::run_cancellable(
        Path::new(engine),
        &[
            "--background",
            "--factory-startup",
            "--disable-autoexec",
            "--python-exit-code",
            "1",
            "--python",
            script.to_str().unwrap(),
            "--",
            mode,
            source.to_str().unwrap(),
        ],
        None,
        Duration::from_secs(60),
        &AtomicBool::new(false),
    )?;
    ensure!(result.code == 0, "{}", result.text);
    let mut registration = f.request("register-blend");
    registration.files = vec![ObjectFile {
        path: "scene.blend".into(),
        role: "source".into(),
    }];
    let object = object_registration::register(&f.runtime, &registration)?.object;
    let object =
        object_version_capture::capture(&f.runtime, &capture_request(&object, "freeze-blend"))?
            .object;
    let version = &object.versions[0];
    let target = Target {
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        version_id: version.version_id.clone(),
        path: "scene.blend".into(),
        sha256: version.manifest["files"][0]["sha256"]
            .as_str()
            .unwrap()
            .into(),
    };
    fs::remove_file(source)?;
    Ok((f, target))
}

#[test]
#[ignore = "requires BEAVER_TEST_BLENDER and a graphical Blender render device"]
fn object_blender_preview_real_frozen_render_and_dependency_rejection() -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_BLENDER").context("Set BEAVER_TEST_BLENDER")?;
    for mode in ["camera", "automatic", "external"] {
        let (f, target) = capture(&engine, mode)?;
        let input = json!({"projectId":target.project_id,"target":target,"requestId":"render","resolution":"720p"});
        let receipt = enqueue(&f.runtime, &input)?;
        assert_eq!(receipt, enqueue(&f.runtime, &input)?);
        let executable = engine.clone();
        let service = Service::start(
            f.runtime.store(),
            f.runtime.files(),
            Arc::new(move |context| {
                ensure!(context.engine == "blender", "Wrong engine selected");
                Ok(Tools {
                    engine: executable.clone().into(),
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
                start.elapsed() < Duration::from_secs(240),
                "Preview timeout"
            );
            std::thread::sleep(Duration::from_millis(200));
        };
        service.shutdown()?;
        assert_eq!(result, latest(&f.runtime, &target)?);
        assert_eq!(result["projectConfig"], "blender-frozen-v1");
        if mode == "external" {
            assert_eq!(result["run"]["status"], "failed", "{result}");
            assert!(
                result["run"]["error"]
                    .as_str()
                    .unwrap()
                    .contains("BLENDER_PREVIEW_DEPENDENCY_NOT_FROZEN"),
                "{result}"
            );
            assert!(result["run"]["evidence"].as_array().unwrap().is_empty());
            continue;
        }
        assert_eq!(result["run"]["status"], "completed", "{result}");
        assert!(result["integrityError"].is_null(), "{result}");
        let run: Run = repository::get(
            &f.runtime.store().lock().unwrap(),
            "validationRun",
            receipt["runId"].as_str().unwrap(),
        )?;
        let path = validation::evidence::media_path(&f.runtime.files(), &run, &run.evidence[0].id)?;
        let image = image::open(&path)?.to_rgb8();
        assert_eq!(image.dimensions(), (1280, 720));
        let green = image
            .pixels()
            .filter(|p| {
                u16::from(p[1]) > u16::from(p[0]) + 30 && u16::from(p[1]) > u16::from(p[2]) + 30
            })
            .count();
        assert!(
            green > 1000,
            "Expected visible frozen green geometry: {green}"
        );
        if mode == "automatic" {
            for (x, y, pixel) in image.enumerate_pixels() {
                if x < 10 || y < 10 || x >= image.width() - 10 || y >= image.height() - 10 {
                    assert!(
                        pixel.0.iter().all(|channel| *channel < 20),
                        "Automatic camera cropped geometry at {x},{y}"
                    );
                }
            }
        }
        assert_eq!(
            run.evidence[0].state["cameraSource"],
            if mode == "camera" {
                "frozen"
            } else {
                "automatic-bounds-v1"
            }
        );
        if let Ok(output) = std::env::var("BEAVER_PREVIEW_PROOF") {
            fs::create_dir_all(&output)?;
            fs::copy(
                &path,
                Path::new(&output).join(format!("blender-{mode}.png")),
            )?;
            fs::write(
                Path::new(&output).join(format!("blender-{mode}.json")),
                serde_json::to_vec_pretty(&result)?,
            )?;
        }
        fs::write(path, b"corrupted")?;
        assert!(latest(&f.runtime, &target)?["integrityError"].is_string());
    }
    Ok(())
}
