use super::{engine_path, package::PACKAGE_ROOT, verify_package, write_new, Input, WORKFLOW};
use crate::{
    files::{file_hash, safe_path},
    process,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path, sync::atomic::AtomicBool, time::Duration};

pub(super) fn engine_errors(text: &str) -> bool {
    text.lines().any(|line| {
        line.contains("SCRIPT ERROR:")
            || line.contains("SHADER ERROR:")
            || line.starts_with("ERROR:")
    })
}

// Do not infer structural success from exit zero or an actor's initialization.
// The official report and exit status must agree, including the failure schema.
pub(super) fn checker_status(report: &Value, exit: i32) -> &'static str {
    if report["schema_version"] != 1
        || !report["compliance_errors"].is_array()
        || !report["visual_suggestions"].is_array()
        || !report["measurements"].is_object()
        || !report["not_evaluated"].is_array()
    {
        return "error";
    }
    match (
        exit,
        report["status"].as_str(),
        report["compliance_errors"].as_array().map(Vec::is_empty),
    ) {
        (0, Some("pass"), Some(true)) => "pass",
        (1, Some("fail"), Some(false)) => "fail",
        _ => "error",
    }
}

fn adapter(
    root: &Path,
    engine: &Path,
    script: &str,
    definition: &str,
    folder: &str,
    mode: &str,
    options: &str,
    cancelled: &AtomicBool,
) -> Result<(Value, bool)> {
    let uri = format!("res://{script}");
    let mut args = vec![
        "--path",
        ".",
        "--script",
        &uri,
        "--resolution",
        "768x768",
        "--rendering-method",
        "forward_plus",
    ];
    if mode == "metadata" {
        args.push("--headless");
    }
    args.extend(["--", definition, mode, folder, options]);
    let output = process::run_cancellable(
        engine,
        &args,
        Some(root),
        Duration::from_secs(180),
        cancelled,
    )?;
    write_new(
        root,
        &format!("{folder}/{mode}.log"),
        output.text.as_bytes(),
    )?;
    let report: Value = output
        .text
        .lines()
        .find_map(|line| line.strip_prefix("BEAVER_WORKFLOW_RESULT="))
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| {
            json!({"status":"error",
            "errors":["Engine did not produce a capture/metadata result"]})
        });
    let expected = if mode == "metadata" {
        "metadata"
    } else {
        "rendered"
    };
    let valid = output.code == 0
        && !engine_errors(&output.text)
        && report["status"] == expected
        && report["errors"].as_array().is_some_and(Vec::is_empty);
    Ok((report, valid))
}

fn model_hash(root: &Path, metadata: &Value) -> Option<String> {
    metadata["modelSource"]
        .as_str()
        .and_then(|p| p.strip_prefix("res://"))
        .and_then(|p| safe_path(root, p).ok())
        .and_then(|p| file_hash(&p).ok().flatten())
}

pub(super) fn run(
    root: &Path,
    input: Input,
    config: Value,
    cancelled: &AtomicBool,
) -> Result<Value> {
    let definition = input
        .definition
        .as_deref()
        .context("definition is required")?;
    let definition_hash = file_hash(&safe_path(root, definition)?)?;
    let engine = engine_path(config["godot"].as_str().context("Missing project engine")?)?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let folder = format!("artifacts/npr/{run_id}");
    fs::create_dir_all(safe_path(root, &folder)?)?;
    let script = format!(".beaver-context/workflows/{run_id}/capture.gd");
    write_new(
        root,
        &script,
        include_bytes!("../../../resources/workflows/npr_validate.gd"),
    )?;
    crate::game_export::import_project(
        &engine,
        root,
        &safe_path(root, &format!("{folder}/import.log"))?,
        cancelled,
    )?;
    let options = json!({"camera":input.camera,"grayscale":input.grayscale}).to_string();
    let (metadata, metadata_ok) = adapter(
        root, &engine, &script, definition, &folder, "metadata", &options, cancelled,
    )?;
    let before_model = model_hash(root, &metadata);
    let checker = format!("res://{PACKAGE_ROOT}/.ci_script/model/check_model.gd");
    let output_folder = format!("res://{folder}/model_check");
    // Always pass --definition: omission runs the upstream Silver Wolf default.
    let definition_uri = format!("res://{definition}");
    let output = process::run_cancellable(
        &engine,
        &[
            "--headless",
            "--path",
            ".",
            "--script",
            &checker,
            "--",
            "--definition",
            &definition_uri,
            "--output",
            &output_folder,
        ],
        Some(root),
        Duration::from_secs(180),
        cancelled,
    )?;
    write_new(
        root,
        &format!("{folder}/engine.log"),
        output.text.as_bytes(),
    )?;
    let official_path = format!("{folder}/model_check/report.json");
    let official: Value = fs::read(safe_path(root, &official_path)?)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    let mut status = checker_status(&official, output.code);
    if official["definition"] != definition_uri {
        status = "error";
    }
    let mut errors = Vec::new();
    if status == "error" {
        errors.push(json!("Official model checker failed to produce a valid report consistent with exit 0/1 (exit 2 means report I/O failure)"));
    }
    let has_engine_errors = engine_errors(&output.text);
    if has_engine_errors {
        errors.push(json!(
            "Official checker logged script, shader or engine errors"
        ));
        status = "error";
    }
    let package_unchanged = verify_package(root).is_ok();
    let checked_sources_match = package_unchanged
        && definition_hash.is_some()
        && file_hash(&safe_path(root, definition)?)? == definition_hash
        && before_model.is_some()
        && model_hash(root, &metadata) == before_model
        && before_model.as_deref() == metadata["modelSha256"].as_str();
    let mut capture = json!({"screenshots":[],"grayscaleScreenshots":[]});
    let mut render_ok = input.action != "preview";
    if status == "pass" && metadata_ok && checked_sources_match && input.action == "preview" {
        let (result, valid) = adapter(
            root, &engine, &script, definition, &folder, "preview", &options, cancelled,
        )?;
        capture = result;
        render_ok = valid
            && capture["screenshots"]
                .as_array()
                .is_some_and(|v| !v.is_empty());
        if !render_ok {
            errors.push(json!(
                "Preview capture failed; inspect preview.log and capture.errors"
            ));
        }
    }
    let package_unchanged = package_unchanged && verify_package(root).is_ok();
    if !package_unchanged {
        errors.push(json!(
            "Pinned NPR package changed during validation/capture"
        ));
    }
    let source_stable = package_unchanged
        && checked_sources_match
        && file_hash(&safe_path(root, definition)?)? == definition_hash
        && before_model.is_some()
        && model_hash(root, &metadata) == before_model
        && before_model.as_deref() == metadata["modelSha256"].as_str()
        && (input.action != "preview"
            || !render_ok
            || capture["modelSha256"] == metadata["modelSha256"]);
    if status == "pass" && (!metadata_ok || !source_stable) {
        errors.push(json!("Definition/model source identity could not be verified or changed during validation/capture"));
    }
    let ok = status == "pass" && errors.is_empty() && render_ok && source_stable;
    let envelope_path = format!("{folder}/report.json");
    let mut report = json!({"schemaVersion":1,"ok":ok,"status":if ok {"pass"} else if status == "fail" {"fail"} else {"error"},
        "runId":run_id,"workflow":WORKFLOW,"action":input.action,"definition":definition,
        "definitionSha256":definition_hash,"model":metadata["model"],"modelSource":metadata["modelSource"],
        "modelSha256":metadata["modelSha256"],"definitionAndModelHashesVerified":source_stable,
        "frameworkPackageHashesVerified":package_unchanged,"assetDependencyHashesVerified":false,
        "engine":metadata["engine"],"engineErrors":has_engine_errors,"errors":errors,
        "modelCheck":official,"modelCheckExitCode":output.code,"modelCheckReport":official_path,
        "compliance_errors":[],"visual_suggestions":[],"measurements":{},"not_evaluated":[],
        "capture":capture,"screenshots":capture["screenshots"],"grayscaleScreenshots":capture["grayscaleScreenshots"],
        "visualAcceptance":"not_evaluated","metadataLog":format!("{folder}/metadata.log"),
        "previewLog":if input.action == "preview" {Some(format!("{folder}/preview.log"))} else {None},"log":format!("{folder}/engine.log"),"report":envelope_path});
    for name in [
        "compliance_errors",
        "visual_suggestions",
        "measurements",
        "not_evaluated",
    ] {
        if !official[name].is_null() {
            report[name] = official[name].clone();
        }
    }
    if !capture["camera"].is_null() {
        report["camera"] = capture["camera"].clone();
    }
    write_new(root, &envelope_path, &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}
