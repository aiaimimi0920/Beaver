use super::*;
use base64::Engine;

fn materialize_package(root: &Path) -> Result<()> {
    let bundle: Value =
        serde_json::from_str(include_str!("../../../dist-native/npr-package.json"))?;
    for (relative, encoded) in bundle.as_object().context("Invalid test bundle")? {
        let relative = if relative == "provenance.json" {
            format!("{}/beaver-provenance.json", package::PACKAGE_ROOT)
        } else {
            relative.clone()
        };
        let bytes = base64::engine::general_purpose::STANDARD.decode(encoded.as_str().unwrap())?;
        write_new(root, &relative, &bytes)?;
    }
    Ok(())
}

fn current_runtime() -> Value {
    json!({"schemaVersion":1,"godot":"not-an-engine","packages":{"npr-characters":{
        "status":"ready","sourcePath":package::PACKAGE_ROOT,"sourceCommit":package::SOURCE_COMMIT,
        "contractVersion":"1.1.0","pluginVersion":"1.3.0"}}})
}

#[test]
fn preview_camera_is_bounded_and_never_silently_ignored() -> Result<()> {
    let input = json!({"workflow":WORKFLOW,"action":"preview","definition":"character.tres","camera":{"target":[0.0,1.6,0.0],"distance":0.8,"fov":35.0,"angles":[0.0,45.0,90.0]},"grayscale":true});
    serde_json::from_value::<Input>(input.clone())?.validate_preview()?;
    for patch in [
        json!({"distance":0}),
        json!({"angles":[]}),
        json!({"angles":[0,1,2,3,4]}),
        json!({"fov":180}),
    ] {
        let mut invalid = input.clone();
        for (key, value) in patch.as_object().unwrap() {
            invalid["camera"][key] = value.clone();
        }
        assert!(serde_json::from_value::<Input>(invalid)?
            .validate_preview()
            .is_err());
    }
    let mut invalid = input;
    invalid["action"] = json!("validate");
    assert!(serde_json::from_value::<Input>(invalid)?
        .validate_preview()
        .is_err());
    Ok(())
}

#[test]
fn workflows_require_current_package_and_contained_definition() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    assert_eq!(list(tmp.path())?["workflows"][0]["enabled"], false);
    assert!(run(
        tmp.path(),
        json!({"workflow":WORKFLOW,"action":"inspect"}),
        &AtomicBool::new(false)
    )
    .is_err());
    fs::write(tmp.path().join(RUNTIME_FILE), current_runtime().to_string())?;
    assert_eq!(list(tmp.path())?["workflows"][0]["enabled"], false);
    assert!(package::require_current(tmp.path(), &current_runtime()).is_err());
    materialize_package(tmp.path())?;
    assert_eq!(list(tmp.path())?["workflows"][0]["enabled"], true);
    for definition in ["../escape.tres", "C:/escape.tres", "missing.tres"] {
        assert!(run(
            tmp.path(),
            json!({"workflow":WORKFLOW,"action":"validate","definition":definition}),
            &AtomicBool::new(false)
        )
        .is_err());
    }
    assert!(run(
        tmp.path(),
        json!({"workflow":WORKFLOW,"action":"unknown"}),
        &AtomicBool::new(false)
    )
    .is_err());
    Ok(())
}

#[test]
fn legacy_manifest_keeps_engine_but_requires_explicit_migration() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let legacy = json!({"schemaVersion":1,"godot":"legacy-engine","packages":{"npr-characters":{"status":"ready","version":"1.0.0"}}});
    fs::write(tmp.path().join(RUNTIME_FILE), legacy.to_string())?;
    assert_eq!(
        configured_engine(&json!({"path":tmp.path()}), "fallback")?,
        "legacy-engine"
    );
    assert_eq!(list(tmp.path())?["workflows"][0]["migrationRequired"], true);
    assert!(run(
        tmp.path(),
        json!({"workflow":WORKFLOW,"action":"inspect"}),
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("explicit migration"));
    assert_eq!(runtime(tmp.path())?, Some(legacy));
    fs::write(tmp.path().join(RUNTIME_FILE), current_runtime().to_string())?;
    fs::create_dir_all(tmp.path().join("addons/npr_characters"))?;
    assert!(package::require_current(tmp.path(), &current_runtime()).is_err());
    Ok(())
}

#[test]
fn official_result_requires_report_schema_and_exit_status_agreement() {
    let pass = json!({"schema_version":1,"status":"pass","compliance_errors":[],"visual_suggestions":[],"measurements":{},"not_evaluated":["visual appearance"]});
    assert_eq!(check::checker_status(&pass, 0), "pass");
    for exit in [1, 2, -1] {
        assert_eq!(check::checker_status(&pass, exit), "error");
    }
    assert_eq!(
        check::checker_status(&json!({"initialized":true,"errors":[]}), 0),
        "error"
    );
    let mut fail = pass.clone();
    fail["status"] = json!("fail");
    fail["compliance_errors"] = json!([{"category":"geometry","message":"Missing UV1"}]);
    assert_eq!(check::checker_status(&fail, 1), "fail");
    assert_eq!(check::checker_status(&fail, 0), "error");
    fail.as_object_mut().unwrap().remove("not_evaluated");
    assert_eq!(check::checker_status(&fail, 1), "error");
    assert!(check::engine_errors("SCRIPT ERROR: cannot parse"));
    assert!(check::engine_errors("SHADER ERROR: post_light unavailable"));
}

#[test]
fn inspect_returns_new_authoring_docs_and_all_three_prompts() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    fs::write(tmp.path().join(RUNTIME_FILE), current_runtime().to_string())?;
    let docs = format!("{}/docs/model_authoring", package::PACKAGE_ROOT);
    materialize_package(tmp.path())?;
    let result = run(
        tmp.path(),
        json!({"workflow":WORKFLOW,"action":"inspect"}),
        &AtomicBool::new(false),
    )?;
    assert_eq!(
        result["guide"],
        fs::read_to_string(tmp.path().join(format!("{docs}/README.md")))?
    );
    assert_eq!(result["prompts"].as_object().unwrap().len(), 3);
    assert_eq!(
        result["prompts"]["feature_data"]["content"],
        fs::read_to_string(tmp.path().join(format!("{docs}/prompts/feature_data.md")))?
    );
    assert!(result["authoringDocs"]
        .as_array()
        .unwrap()
        .contains(&json!(format!("{docs}/README.md"))));
    Ok(())
}

#[cfg(unix)]
#[path = "workflows_process_tests.rs"]
mod process_tests;

#[path = "workflows_integrity_tests.rs"]
mod integrity_tests;
