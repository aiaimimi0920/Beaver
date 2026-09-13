use crate::{files::safe_path, process};
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub const RUNTIME_FILE: &str = "beaver.runtime.json";
pub const WORKFLOW: &str = "npr-character";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NprOptions {
    pub godot: String,
}

pub fn engine_path(configured: &str) -> Result<PathBuf> {
    let path = Path::new(configured);
    let path = if path.is_dir() {
        path.join("godot.windows.editor.x86_64.exe")
    } else {
        path.to_path_buf()
    };
    if !path.is_absolute() || !path.is_file() {
        bail!("请选择定制 Godot 编辑器或其 export 目录");
    }
    crate::tools::find("godot", path.to_str().context("Invalid engine path")?)
}

pub fn runtime(root: &Path) -> Result<Option<Value>> {
    let path = safe_path(root, RUNTIME_FILE)?;
    if !path.exists() {
        return Ok(None);
    }
    if fs::metadata(&path)?.len() > 65536 {
        bail!("Runtime manifest too large");
    }
    let value: Value = serde_json::from_slice(&fs::read(path)?)?;
    if value["schemaVersion"] != 1 || value["packages"]["npr-characters"]["status"] != "ready" {
        bail!("NPR runtime is not ready");
    }
    Ok(Some(value))
}

pub fn configured_engine(project: &Value, fallback: &str) -> Result<String> {
    let root = Path::new(project["path"].as_str().context("Invalid project path")?);
    Ok(runtime(root)?
        .and_then(|v| v["godot"].as_str().map(str::to_owned))
        .unwrap_or_else(|| fallback.to_owned()))
}

fn write_new(root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    let path = safe_path(root, relative)?;
    fs::create_dir_all(path.parent().context("Invalid resource path")?)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(safe_path(root, relative)?)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn install(root: &Path, configured: &str, cancelled: &AtomicBool) -> Result<Value> {
    let engine = engine_path(configured)?;
    let package: Value =
        serde_json::from_str(include_str!("../../../dist-native/npr-package.json"))?;
    if let Some(existing) = runtime(root)? {
        if engine_path(existing["godot"].as_str().context("Missing bound engine")?)? == engine {
            return Ok(existing);
        }
        bail!("NPR already uses another engine; explicit migration is required");
    }
    let mut entries = Vec::new();
    for (relative, encoded) in package.as_object().context("Invalid bundled NPR package")? {
        let relative = if relative == "provenance.json" {
            "addons/npr_characters/beaver-provenance.json"
        } else {
            relative
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.as_str().context("Invalid package entry")?)?;
        let path = safe_path(root, relative)?;
        if path.exists() && fs::read(&path)? != bytes {
            bail!("NPR package has customized/conflicting file: {relative}");
        }
        entries.push((relative, bytes, path.exists()));
    }
    // Recovery may reuse identical files from a partial install, never overwrite edits.
    for (relative, bytes, exists) in entries {
        if !exists {
            write_new(root, relative, &bytes)?;
        }
    }
    let context = format!(".beaver-context/workflows/{}", uuid::Uuid::new_v4());
    let script = format!("{context}/setup.gd");
    write_new(
        root,
        &script,
        include_bytes!("../../../resources/workflows/npr_setup.gd"),
    )?;
    let output = process::run_cancellable(
        &engine,
        &[
            "--headless",
            "--path",
            ".",
            "--script",
            &format!("res://{script}"),
        ],
        Some(root),
        Duration::from_secs(120),
        cancelled,
    )?;
    write_new(
        root,
        &format!("{context}/install.log"),
        output.text.as_bytes(),
    )?;
    if output.code != 0 || !output.text.contains("BEAVER_NPR_INSTALL_OK") {
        bail!(
            "NPR install/engine compatibility failed: {}",
            output.text.chars().take(3000).collect::<String>()
        );
    }
    let version = process::run_cancellable(
        &engine,
        &["--version"],
        Some(root),
        Duration::from_secs(20),
        cancelled,
    )?;
    if version.code != 0 {
        bail!("NPR engine version probe failed");
    }
    let runtime = json!({"schemaVersion":1,"godot":engine,"engineVersion":version.text.trim(),"packages":{"npr-characters":{"version":"1.0.0","status":"ready","installedAt":chrono::Utc::now().to_rfc3339(),"contract":"addons/npr_characters/asset_contract.json","guide":"addons/npr_characters/AI_CONTEXT.md","validation":"installation and reflected extensions; character rendering requires workflow validation"}}});
    write_new(root, RUNTIME_FILE, &serde_json::to_vec_pretty(&runtime)?)?;
    Ok(runtime)
}

pub fn list(root: &Path) -> Result<Value> {
    let enabled = runtime(root)?.is_some();
    Ok(
        json!({"workflows":[{"id":WORKFLOW,"version":"1.1.0","enabled":enabled,"name":"NPR 人物制作与接入","actions":["inspect","validate","preview"],"description":"Codex authors the character using Blender MCP and the bundled contract. Preview supports an explicit camera for matched close-ups, optional grayscale images and source hashes. Reuse the returned camera for before/after comparison."}]}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    workflow: String,
    action: String,
    definition: Option<String>,
    camera: Option<PreviewCamera>,
    #[serde(default)]
    grayscale: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PreviewCamera {
    target: [f64; 3],
    distance: f64,
    fov: f64,
    angles: Vec<f64>,
}

impl Input {
    fn validate_preview(&self) -> Result<()> {
        if self.action != "preview" && (self.camera.is_some() || self.grayscale) {
            bail!("camera and grayscale are only supported for preview");
        }
        if let Some(camera) = &self.camera {
            anyhow::ensure!(camera.target.iter().all(|v| v.is_finite() && v.abs() <= 100000.0)
                && camera.distance.is_finite() && (0.01..=100000.0).contains(&camera.distance)
                && camera.fov.is_finite() && (5.0..=120.0).contains(&camera.fov)
                && (1..=4).contains(&camera.angles.len())
                && camera.angles.iter().all(|v| v.is_finite() && (-360.0..=360.0).contains(v)),
                "Invalid preview camera: supply target xyz, positive distance, fov 5..120 and 1..4 angles in degrees");
        }
        Ok(())
    }
}

pub fn camera_schema() -> Value {
    json!({"type":"object","description":"Fixed world-space camera for matched before/after and head close-ups. Angles are degrees about target, with the same studio light and a fixed 0.1 * distance elevation. Reuse the camera returned by an earlier preview.","properties":{"target":{"type":"array","items":{"type":"number"},"minItems":3,"maxItems":3},"distance":{"type":"number","minimum":0.01,"maximum":100000},"fov":{"type":"number","minimum":5,"maximum":120},"angles":{"type":"array","items":{"type":"number","minimum":-360,"maximum":360},"minItems":1,"maxItems":4}},"required":["target","distance","fov","angles"],"additionalProperties":false})
}

pub fn run(root: &Path, input: Value, cancelled: &AtomicBool) -> Result<Value> {
    let input: Input = serde_json::from_value(input).context("Invalid workflow arguments")?;
    input.validate_preview()?;
    if input.workflow != WORKFLOW {
        bail!("Unknown workflow");
    }
    let config = runtime(root)?.context("Create a project with NPR enabled first")?;
    if input.action == "inspect" {
        let guide = fs::read_to_string(safe_path(root, "addons/npr_characters/AI_CONTEXT.md")?)?;
        let contract: Value = serde_json::from_slice(&fs::read(safe_path(
            root,
            "addons/npr_characters/asset_contract.json",
        )?)?)?;
        return Ok(
            json!({"status":"instructions","workflow":WORKFLOW,"runtime":config,"guide":guide,"contract":contract,"previewOptions":{"camera":camera_schema(),"grayscale":"Optional boolean: save grayscale views alongside color views"},"next":"Author .blend, GLB, textures and definition in this workspace via Blender MCP, then call validate and preview with the actual definition path. For matched close-ups pass camera={target:[x,y,z],distance:positive,fov:degrees,angles:[0,45,90]} and optionally grayscale=true; reuse report.camera before/after. Verify definition/model hashes identify the requested asset. The bundled mannequin is diagnostic, not the requested character."}),
        );
    }
    if !["validate", "preview"].contains(&input.action.as_str()) {
        bail!("Unknown workflow action");
    }
    let definition = input.definition.context("definition is required")?;
    if !definition.ends_with(".tres") || !safe_path(root, &definition)?.is_file() {
        bail!("Expected an existing project-relative definition .tres");
    }
    let definition_hash = crate::files::file_hash(&safe_path(root, &definition)?)?;
    let engine = engine_path(config["godot"].as_str().context("Missing project engine")?)?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let folder = format!("artifacts/npr/{run_id}");
    fs::create_dir_all(safe_path(root, &folder)?)?;
    let script = format!(".beaver-context/workflows/{run_id}/validate.gd");
    write_new(
        root,
        &script,
        include_bytes!("../../../resources/workflows/npr_validate.gd"),
    )?;
    let import_log = safe_path(root, &format!("{folder}/import.log"))?;
    crate::game_export::import_project(&engine, root, &import_log, cancelled)?;
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
    if input.action == "validate" {
        args.push("--headless");
    }
    let preview_options = json!({"camera":input.camera,"grayscale":input.grayscale}).to_string();
    args.extend(["--", &definition, &input.action, &folder, &preview_options]);
    let output = process::run_cancellable(
        &engine,
        &args,
        Some(root),
        Duration::from_secs(180),
        cancelled,
    )?;
    write_new(
        root,
        &format!("{folder}/engine.log"),
        output.text.as_bytes(),
    )?;
    let mut report: Value = output
        .text
        .lines()
        .find_map(|line| line.strip_prefix("BEAVER_WORKFLOW_RESULT="))
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(
            || json!({"initialized":false,"errors":["Engine did not produce a validation result"]}),
        );
    let engine_errors = output.text.lines().any(|line| {
        line.contains("SCRIPT ERROR:")
            || line.contains("SHADER ERROR:")
            || line.starts_with("ERROR:")
    });
    let definition_stable =
        crate::files::file_hash(&safe_path(root, &definition)?)? == definition_hash;
    let model_hash = report["modelSource"]
        .as_str()
        .and_then(|p| p.strip_prefix("res://"))
        .and_then(|p| safe_path(root, p).ok())
        .and_then(|p| crate::files::file_hash(&p).ok().flatten());
    let model_stable = model_hash
        .as_deref()
        .is_some_and(|hash| report["modelSha256"].as_str() == Some(hash));
    if report["initialized"] == true && (!definition_stable || !model_stable) {
        if let Some(errors) = report["errors"].as_array_mut() {
            errors.push(json!("Definition or model changed during capture, or its source hash could not be verified"));
        }
    }
    let ok = output.code == 0
        && report["initialized"] == true
        && report["errors"].as_array().is_some_and(Vec::is_empty)
        && !engine_errors
        && definition_stable
        && model_stable;
    report["ok"] = json!(ok);
    report["runId"] = json!(run_id);
    report["workflow"] = json!(WORKFLOW);
    report["action"] = json!(input.action);
    report["engineErrors"] = json!(engine_errors);
    report["definitionSha256"] = json!(definition_hash);
    report["log"] = json!(format!("{folder}/engine.log"));
    report["report"] = json!(format!("{folder}/report.json"));
    write_new(
        root,
        report["report"].as_str().unwrap(),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"beaver_workflow_list","description":"Discover Beaver standard workflows enabled for this task workspace.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({"name":"beaver_workflow_run","description":"Use a standard Beaver workflow in the current task copy. Inspect provides authoring contract; validate checks actual NPR assets; preview renders matched views with optional fixed camera and grayscale. Reuse report.camera for before/after; verify source hashes. Results with ok=false need repair, not success claims.","inputSchema":{"type":"object","properties":{"workflow":{"type":"string","enum":[WORKFLOW]},"action":{"type":"string","enum":["inspect","validate","preview"]},"definition":{"type":"string","description":"Existing project-relative NPRCharacterDefinition .tres path for validate/preview"},"camera":camera_schema(),"grayscale":{"type":"boolean"}},"required":["workflow","action"],"additionalProperties":false}}),
    ]
}

pub async fn call(root: PathBuf, name: &str, input: Value) -> Result<String> {
    if name == "beaver_workflow_list" {
        return Ok(list(&root)?.to_string());
    }
    struct Cancel(Arc<AtomicBool>);
    impl Drop for Cancel {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let _guard = Cancel(cancelled.clone());
    Ok(
        tokio::task::spawn_blocking(move || run(&root, input, &cancelled))
            .await??
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn workflows_require_enabled_package_and_contained_definition() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        assert_eq!(list(tmp.path())?["workflows"][0]["enabled"], false);
        assert!(run(
            tmp.path(),
            json!({"workflow":WORKFLOW,"action":"inspect"}),
            &AtomicBool::new(false)
        )
        .is_err());
        fs::write(tmp.path().join(RUNTIME_FILE),json!({"schemaVersion":1,"packages":{"npr-characters":{"status":"ready"}},"godot":"not-an-engine"}).to_string())?;
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
}
