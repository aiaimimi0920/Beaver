use super::{verify_package, write_new, RUNTIME_FILE};
use crate::{files::safe_path, process};
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub(super) const PACKAGE_ROOT: &str = "addons/npr_character_frame";
pub(super) const SOURCE_COMMIT: &str = "a08b47a6cc229b6978afda26d74d13af690a67ba";

pub fn engine_path(configured: &str) -> Result<PathBuf> {
    let path = Path::new(configured);
    let path = if path.is_dir() {
        #[cfg(windows)]
        let name = "godot.windows.editor.x86_64.exe";
        #[cfg(not(windows))]
        let name = "godot.linuxbsd.editor.x86_64";
        path.join(name)
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
    // Old projects still keep their bound engine for ordinary project operations.
    // The new workflow explicitly rejects their package instead of migrating it.
    Ok(Some(value))
}

pub(super) fn require_current(root: &Path, value: &Value) -> Result<()> {
    let package = &value["packages"]["npr-characters"];
    if package["sourcePath"] != PACKAGE_ROOT
        || package["sourceCommit"] != SOURCE_COMMIT
        || package["contractVersion"] != "1.1.0"
        || package["pluginVersion"] != "1.3.0"
        || safe_path(root, "addons/npr_characters")?.exists()
    {
        bail!("NPR package requires explicit migration; legacy npr_characters and npr_character_frame must not coexist");
    }
    verify_package(root)
}

pub fn configured_engine(project: &Value, fallback: &str) -> Result<String> {
    let root = Path::new(project["path"].as_str().context("Invalid project path")?);
    Ok(runtime(root)?
        .and_then(|v| v["godot"].as_str().map(str::to_owned))
        .unwrap_or_else(|| fallback.to_owned()))
}

pub fn install(root: &Path, configured: &str, cancelled: &AtomicBool) -> Result<Value> {
    let engine = engine_path(configured)?;
    if let Some(existing) = runtime(root)? {
        require_current(root, &existing)?;
        if engine_path(existing["godot"].as_str().context("Missing bound engine")?)? == engine {
            return Ok(existing);
        }
        bail!("NPR already uses another engine; explicit migration is required");
    }
    if safe_path(root, "addons/npr_characters")?.exists() {
        bail!("Legacy npr_characters package exists; explicit migration is required before installation");
    }
    let package: Value =
        serde_json::from_str(include_str!("../../../dist-native/npr-package.json"))?;
    let mut entries = Vec::new();
    for (relative, encoded) in package.as_object().context("Invalid bundled NPR package")? {
        let relative = if relative == "provenance.json" {
            format!("{PACKAGE_ROOT}/beaver-provenance.json")
        } else {
            relative.clone()
        };
        if !relative.starts_with(&format!("{PACKAGE_ROOT}/")) {
            bail!("Unexpected bundled NPR path: {relative}");
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.as_str().context("Invalid package entry")?)?;
        let path = safe_path(root, &relative)?;
        if path.exists() && fs::read(&path)? != bytes {
            bail!("NPR package has customized/conflicting file: {relative}");
        }
        entries.push((relative, bytes, path.exists()));
    }
    for (relative, bytes, exists) in entries {
        if !exists {
            write_new(root, &relative, &bytes)?;
        }
    }
    // Verify all compiled-in dependency hashes before launching any package code.
    verify_package(root)?;
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
    let runtime = json!({"schemaVersion":1,"godot":engine,"engineVersion":version.text.trim(),
        "packages":{"npr-characters":{"contract":"addons/npr_character_frame/asset_contract.json",
            "contractVersion":"1.1.0","pluginVersion":"1.3.0","sourceCommit":SOURCE_COMMIT,
            "sourcePath":PACKAGE_ROOT,"editorPluginAutoEnabled":false,
            "sampleShowcase":"not distributed; upstream editor demo menu is not enabled", "status":"ready","installedAt":chrono::Utc::now().to_rfc3339(),
            "guide":"addons/npr_character_frame/docs/model_authoring/README.md",
            "provenance":"addons/npr_character_frame/beaver-provenance.json",
            "validation":"installation and reflected extensions only; structural compliance and visual appearance require separate evidence"}}});
    write_new(root, RUNTIME_FILE, &serde_json::to_vec_pretty(&runtime)?)?;
    Ok(runtime)
}
