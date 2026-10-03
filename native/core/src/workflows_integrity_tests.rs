use super::*;

fn ready_project() -> Result<tempfile::TempDir> {
    let tmp = tempfile::tempdir()?;
    materialize_package(tmp.path())?;
    fs::write(tmp.path().join(RUNTIME_FILE), current_runtime().to_string())?;
    verify_package(tmp.path())?;
    Ok(tmp)
}

#[test]
fn checker_and_shader_changes_reject_workflow_despite_forged_project_provenance() -> Result<()> {
    for relative in [
        ".ci_script/model/check_model.gd",
        "shaders/body/body_npr.gdshader",
    ] {
        let tmp = ready_project()?;
        let target = tmp.path().join(package::PACKAGE_ROOT).join(relative);
        let mut edited = fs::read(&target)?;
        edited.extend_from_slice(b"\n// local change\n");
        fs::write(&target, &edited)?;
        // A project's editable provenance cannot bless the modified dependency.
        let provenance_path = tmp
            .path()
            .join(package::PACKAGE_ROOT)
            .join("beaver-provenance.json");
        let mut forged: Value = serde_json::from_slice(&fs::read(&provenance_path)?)?;
        forged["files"][relative] = json!(crate::files::file_hash(&target)?);
        fs::write(&provenance_path, forged.to_string())?;
        assert!(verify_package(tmp.path())
            .unwrap_err()
            .to_string()
            .contains(relative));
        assert_eq!(list(tmp.path())?["workflows"][0]["enabled"], false);
        assert_eq!(list(tmp.path())?["workflows"][0]["migrationRequired"], true);
        assert!(run(
            tmp.path(),
            json!({"workflow":WORKFLOW,"action":"inspect"}),
            &AtomicBool::new(false)
        )
        .is_err());
        assert_eq!(fs::read(&target)?, edited);
    }
    Ok(())
}

#[test]
fn ready_manifest_cannot_hide_deleted_dependencies_or_trigger_silent_repair() -> Result<()> {
    for relative in [
        ".ci_script/model/check_model.gd",
        "shaders/body/body_npr.gdshader",
    ] {
        let tmp = ready_project()?;
        let target = tmp.path().join(package::PACKAGE_ROOT).join(relative);
        fs::remove_file(&target)?;
        let engine = tmp.path().join("test-engine.exe");
        fs::write(&engine, "not executed")?;
        let mut runtime = current_runtime();
        runtime["godot"] = json!(engine);
        fs::write(tmp.path().join(RUNTIME_FILE), runtime.to_string())?;
        assert!(install(
            tmp.path(),
            engine.to_str().unwrap(),
            &AtomicBool::new(false)
        )
        .is_err());
        assert!(!target.exists());
        assert_eq!(super::super::runtime(tmp.path())?, Some(runtime));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn identical_dependency_bytes_behind_file_or_directory_links_are_rejected() -> Result<()> {
    use std::os::unix::fs::symlink;
    let tmp = ready_project()?;
    let checker = tmp
        .path()
        .join(package::PACKAGE_ROOT)
        .join(".ci_script/model/check_model.gd");
    let replacement = tmp.path().join("same-checker.gd");
    fs::rename(&checker, &replacement)?;
    symlink(&replacement, &checker)?;
    assert!(verify_package(tmp.path()).is_err());
    let tmp = ready_project()?;
    let shaders = tmp.path().join(package::PACKAGE_ROOT).join("shaders");
    let replacement = tmp.path().join("same-shaders");
    fs::rename(&shaders, &replacement)?;
    symlink(&replacement, &shaders)?;
    assert!(verify_package(tmp.path()).is_err());
    Ok(())
}
