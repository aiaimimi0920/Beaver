use super::{runner, sandbox, test_support::Fixture};
use anyhow::{bail, Context, Result};
use std::{fs, io::Read, path::Path, sync::atomic::AtomicBool};

#[test]
fn gut_overlay_is_sandbox_only_and_preserves_other_upstream_files() -> Result<()> {
    let f = Fixture::new()?;
    let original = f.files.capture(&f.project)?;
    let run = f.run(None)?;
    let sandbox = sandbox::Sandbox::prepare(&f.files, &run)?;
    sandbox.install_gut()?;
    sandbox.install_gut()?;
    let bytes = include_bytes!("../../../../resources/validation/gut-9.4.0.zip");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut loader_found = false;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let installed = fs::read(sandbox.project.join(&name))?;
        let mut upstream = Vec::new();
        entry.read_to_end(&mut upstream)?;
        if name == "addons/gut/gut_loader.gd" {
            assert_ne!(installed, upstream);
            assert_eq!(
                installed,
                include_bytes!("../../../../resources/validation/gut_loader.gd")
            );
            loader_found = true;
        } else {
            assert_eq!(installed, upstream, "Unexpected upstream change: {name}");
        }
    }
    assert!(loader_found);
    assert_eq!(f.files.capture(&f.project)?, original);
    assert!(!f.project.join("addons/gut").exists());
    Ok(())
}

#[test]
fn successful_exit_does_not_hide_engine_errors() {
    for text in [
        "SCRIPT ERROR: Trying to assign value of type 'Nil' to a variable of type 'bool'.",
        "13 passed\nERROR: fixture failure",
    ] {
        assert!(sandbox::check_output(&crate::process::Output {
            code: 0,
            text: text.into(),
        })
        .is_err());
    }
}

#[test]
#[ignore = "requires BEAVER_TEST_GODOT pointing to a real Godot editor"]
fn gut_overlay_real_engine_restores_legacy_and_directory_warning_policies() -> Result<()> {
    let engine = std::env::var("BEAVER_TEST_GODOT").context("Set BEAVER_TEST_GODOT")?;
    let f = Fixture::new()?;
    fs::write(
        f.project.join("project.godot"),
        "config_version=5\n[debug]\ngdscript/warnings/directory_rules={\"res://addons\":1,\"res://addons/custom\":1,\"res://scripts\":0}\n",
    )?;
    fs::write(
        f.project.join("tests/test_game.gd"),
        include_str!("../../../../resources/validation/tests/test_gut_loader.gd"),
    )?;
    // The editor requires a complete preset, unlike the shared metadata-only fixture.
    fs::write(
        f.project.join("export_presets.cfg"),
        "[preset.0]\nname=\"Windows Desktop\"\nplatform=\"Windows Desktop\"\nexport_filter=\"all_resources\"\ninclude_filter=\"\"\nexclude_filter=\"\"\n[preset.0.options]\nbinary_format/embed_pck=false\n",
    )?;
    let original = f.files.capture(&f.project)?;
    let mut run = f.run(None)?;
    runner::execute(
        &f.files,
        Path::new(&engine),
        None,
        &mut run,
        &AtomicBool::new(false),
        |_| {},
    );
    println!("Engine: {}", run.engine_version);
    if !super::code::green(&run) {
        let logs = super::repository::run_dir(&f.files, &run.id)?;
        for name in ["import.log", "gut.log"] {
            if let Ok(log) = fs::read_to_string(logs.join(name)) {
                let lines: Vec<_> = log.lines().collect();
                for (index, line) in lines.iter().enumerate() {
                    if line.contains("ERROR") {
                        eprintln!(
                            "{name}:{}\n{}",
                            index + 1,
                            lines[index..(index + 6).min(lines.len())].join("\n")
                        );
                    }
                }
            }
        }
        eprintln!("Validation logs: {}", logs.display());
        let retained = f._temp.keep();
        bail!(
            "Validation failed: {:?}; fixture kept at {}",
            run.error,
            retained.display()
        );
    }
    let report = run.code.as_ref().context("GUT report missing")?;
    assert_eq!(report.passed, 4);
    assert!(report.cases.iter().all(|case| case.assertions > 0));
    assert_eq!(f.files.capture(&f.project)?, original);
    Ok(())
}
