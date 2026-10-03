use super::*;
use std::os::unix::fs::PermissionsExt;

// Contract tests use an explicit fake executable, never claim Godot rendering.
fn fixture(exit: i32, status: &str) -> Result<(tempfile::TempDir, Vec<u8>)> {
    let tmp = tempfile::tempdir()?;
    materialize_package(tmp.path())?;
    let engine = tmp.path().join("test-engine");
    fs::write(
        &engine,
        r##"#!/bin/sh
if [ "$1" = "--help" ]; then echo '--import'; exit 0; fi
printf 'CALL\n' >> calls.log
printf '<%s>\n' "$@" >> calls.log
case "$*" in
  *--import*) exit 0 ;;
  *check_model.gd*)
    previous=''
    for argument in "$@"; do
      if [ "$previous" = '--output' ]; then output="${argument#res://}"; fi
      previous="$argument"
    done
    code=$(cat checker-exit)
    if [ "$code" != '2' ]; then mkdir -p "$output"; cp official-fixture.json "$output/report.json"; fi
    if [ -f mutate-model ]; then echo mutation >> model.tscn; fi
    exit "$code" ;;
  *metadata*)
    hash=$(sha256sum model.tscn | cut -d ' ' -f 1)
    printf 'BEAVER_WORKFLOW_RESULT={"status":"metadata","errors":[],"model":"res://model.tscn","modelSource":"res://model.tscn","modelSha256":"%s","engine":"mock"}\n' "$hash"
    exit 0 ;;
  *preview*) echo preview-ran > preview-ran; exit 2 ;;
esac
exit 3
"##,
    )?;
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755))?;
    let mut runtime = current_runtime();
    runtime["godot"] = json!(engine);
    fs::write(tmp.path().join(RUNTIME_FILE), runtime.to_string())?;
    fs::write(tmp.path().join("character.tres"), "test definition")?;
    fs::write(tmp.path().join("model.tscn"), "test model")?;
    fs::write(tmp.path().join("checker-exit"), exit.to_string())?;
    let official = json!({"schema_version":1,"case_id":"model.contract","definition":"res://character.tres",
        "status":status,"compliance_errors":if status == "fail" {json!([{"category":"geometry","message":"Missing UV1"}])} else {json!([])},
        "visual_suggestions":[],"measurements":{"Body":{"surfaces":1}},"not_evaluated":["visual appearance"]});
    let bytes = serde_json::to_vec_pretty(&official)?;
    fs::write(tmp.path().join("official-fixture.json"), &bytes)?;
    Ok((tmp, bytes))
}

fn invoke(root: &Path, action: &str) -> Result<Value> {
    run(
        root,
        json!({"workflow":WORKFLOW,"action":action,"definition":"character.tres"}),
        &AtomicBool::new(false),
    )
}

#[test]
fn official_checker_receives_explicit_definition_and_raw_report_is_preserved() -> Result<()> {
    let (tmp, bytes) = fixture(0, "pass")?;
    let report = invoke(tmp.path(), "validate")?;
    assert_eq!(report["ok"], true);
    assert_eq!(report["status"], "pass");
    assert_eq!(report["visualAcceptance"], "not_evaluated");
    assert_eq!(report["not_evaluated"], json!(["visual appearance"]));
    assert_eq!(report["definitionAndModelHashesVerified"], true);
    assert_eq!(report["frameworkPackageHashesVerified"], true);
    assert_eq!(report["assetDependencyHashesVerified"], false);
    assert!(report.get("sourceHashesVerified").is_none());
    assert!(report.get("initialized").is_none());
    let raw = report["modelCheckReport"].as_str().unwrap();
    assert!(raw.ends_with("/model_check/report.json"));
    assert_eq!(fs::read(tmp.path().join(raw))?, bytes);
    assert_ne!(raw, report["report"].as_str().unwrap());
    let calls = fs::read_to_string(tmp.path().join("calls.log"))?;
    assert!(calls.contains("<--definition>\n<res://character.tres>\n<--output>"));
    assert!(calls.contains("npr_character_frame/.ci_script/model/check_model.gd"));
    Ok(())
}

#[test]
fn compliance_failure_and_io_failure_do_not_render() -> Result<()> {
    for (exit, status, expected) in [
        (1, "fail", "fail"),
        (2, "pass", "error"),
        (0, "fail", "error"),
    ] {
        let (tmp, _) = fixture(exit, status)?;
        let report = invoke(tmp.path(), "preview")?;
        assert_eq!(report["ok"], false);
        assert_eq!(report["status"], expected);
        assert_eq!(report["modelCheckExitCode"], exit);
        assert!(!tmp.path().join("preview-ran").exists());
    }
    Ok(())
}

#[test]
fn changed_model_or_wrong_definition_cannot_pass() -> Result<()> {
    let (tmp, _) = fixture(0, "pass")?;
    fs::write(tmp.path().join("mutate-model"), "")?;
    let report = invoke(tmp.path(), "preview")?;
    assert_eq!(report["ok"], false);
    assert_eq!(report["definitionAndModelHashesVerified"], false);
    assert!(!tmp.path().join("preview-ran").exists());
    let (tmp, _) = fixture(0, "pass")?;
    let fixture_path = tmp.path().join("official-fixture.json");
    let mut official: Value = serde_json::from_slice(&fs::read(&fixture_path)?)?;
    official["definition"] = json!("res://samples/silver_wolf/character_definition.tres");
    fs::write(fixture_path, official.to_string())?;
    assert_eq!(invoke(tmp.path(), "validate")?["ok"], false);
    Ok(())
}
