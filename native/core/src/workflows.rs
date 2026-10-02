use crate::files::safe_path;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

#[path = "workflows_check.rs"]
mod check;
#[path = "workflows_integrity.rs"]
mod integrity;
#[path = "workflows_package.rs"]
mod package;
#[cfg(test)]
#[path = "workflows_tests.rs"]
mod tests;

pub use integrity::verify_package;
pub use package::{configured_engine, engine_path, install, runtime};
pub const RUNTIME_FILE: &str = "beaver.runtime.json";
pub const WORKFLOW: &str = "npr-character";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NprOptions {
    pub godot: String,
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

pub fn list(root: &Path) -> Result<Value> {
    let config = runtime(root)?;
    let enabled = config
        .as_ref()
        .is_some_and(|v| package::require_current(root, v).is_ok());
    Ok(
        json!({"workflows":[{"id":WORKFLOW,"version":"1.2.0","enabled":enabled,
        "migrationRequired":config.is_some() && !enabled,"contractVersion":"1.1.0",
        "pluginVersion":"1.3.0","sourceCommit":package::SOURCE_COMMIT,"sourcePath":package::PACKAGE_ROOT,"name":"NPR 人物制作与接入",
        "actions":["inspect","validate","preview"],
        "description":"Inspect the pinned NPR Character Frame contract and authoring prompts. Validate runs its official model checker. Preview checks compliance before rendering matched camera views and optional grayscale; structural pass does not establish visual acceptance."}]}),
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
    package::require_current(root, &config)?;
    if input.action == "inspect" {
        return inspect(root, config);
    }
    if !["validate", "preview"].contains(&input.action.as_str()) {
        bail!("Unknown workflow action");
    }
    let definition = input
        .definition
        .as_deref()
        .context("definition is required")?;
    if !definition.ends_with(".tres") || !safe_path(root, definition)?.is_file() {
        bail!("Expected an existing project-relative definition .tres");
    }
    check::run(root, input, config, cancelled)
}

fn inspect(root: &Path, config: Value) -> Result<Value> {
    let base = package::PACKAGE_ROOT;
    let guide_path = format!("{base}/docs/model_authoring/README.md");
    let guide = fs::read_to_string(safe_path(root, &guide_path)?)?;
    let contract: Value = serde_json::from_slice(&fs::read(safe_path(
        root,
        &format!("{base}/asset_contract.json"),
    )?)?)?;
    let mut prompts = serde_json::Map::new();
    for name in ["geometry", "textures", "feature_data"] {
        let path = format!("{base}/docs/model_authoring/prompts/{name}.md");
        prompts.insert(
            name.to_owned(),
            json!({"path":path,"content":fs::read_to_string(safe_path(root, &path)?)?}),
        );
    }
    let mut documents = Vec::new();
    for entry in fs::read_dir(safe_path(root, &format!("{base}/docs/model_authoring"))?)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|v| v == "md") {
            documents.push(format!(
                "{base}/docs/model_authoring/{}",
                entry.file_name().to_string_lossy()
            ));
        }
    }
    documents.sort();
    Ok(
        json!({"status":"instructions","workflow":WORKFLOW,"runtime":config,"guide":guide,
        "guidePath":guide_path,"contract":contract,"authoringDocs":documents,"prompts":prompts,
        "previewOptions":{"camera":camera_schema(),"grayscale":"Optional boolean: save grayscale views alongside color views"},
        "next":"Read docs/model_authoring and all three prompts. Author .blend, GLB, textures and a definition in this workspace via Blender MCP. Explicitly pass the actual project-relative definition to validate and preview. Preserve official model_check/report.json separately from the Beaver envelope. Check compliance_errors, visual_suggestions, measurements and not_evaluated. A structural pass is not full-feature or visual acceptance. Reuse report.camera and verify source hashes for matched before/after views; do not use the diagnostic mannequin as the requested character."}),
    )
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
