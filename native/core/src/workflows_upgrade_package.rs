//! Compiled trust roots for explicitly supported NPR package transitions.
use super::super::{package, RUNTIME_FILE};
use crate::files::{file_hash_limited, safe_path};
use anyhow::{ensure, Context, Result};
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) const OLD_COMMIT: &str = "05f3dceee965c9a5ec78ee8d159e962c133cfc57";
pub(super) const EARLIER_COMMIT: &str = "48839ce137ce2749210a613c565b602c5a6bbc4a";
const EARLIER_MANIFEST: &[u8] =
    include_bytes!("../../../resources/workflows/npr-upgrade-from-48839ce1.json");
const OLD_MANIFEST: &[u8] =
    include_bytes!("../../../resources/workflows/npr-upgrade-from-05f3dcee.json");

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn identity(root: &Path) -> Result<(Value, String)> {
    let path = safe_path(root, RUNTIME_FILE)?;
    ensure!(
        fs::metadata(&path)?.len() <= 65536,
        "Runtime manifest too large"
    );
    let bytes = fs::read(path)?;
    Ok((serde_json::from_slice(&bytes)?, hash(&bytes)))
}

pub(super) fn old_manifest(commit: &str) -> Result<&'static [u8]> {
    match commit {
        OLD_COMMIT => Ok(OLD_MANIFEST),
        EARLIER_COMMIT => Ok(EARLIER_MANIFEST),
        _ => {
            anyhow::bail!("Unsupported NPR upgrade source; a compiled trusted manifest is required")
        }
    }
}

pub(super) fn old_hashes(commit: &str) -> Result<BTreeMap<String, String>> {
    let bytes = old_manifest(commit)?;
    let manifest: Value = serde_json::from_slice(bytes)?;
    ensure!(
        manifest["sourceCommit"] == commit && manifest["sourcePath"] == package::PACKAGE_ROOT,
        "Invalid compiled upgrade source"
    );
    let files: BTreeMap<String, String> = serde_json::from_value(manifest["files"].clone())?;
    let mut hashes: BTreeMap<_, _> = files
        .into_iter()
        .map(|(name, hash)| (format!("{}/{name}", package::PACKAGE_ROOT), hash))
        .collect();
    hashes.insert(
        format!("{}/beaver-provenance.json", package::PACKAGE_ROOT),
        hash(bytes),
    );
    Ok(hashes)
}

pub(super) fn bundled() -> Result<BTreeMap<String, Vec<u8>>> {
    let bundle: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../dist-native/npr-package.json"))?;
    bundle
        .into_iter()
        .map(|(name, encoded)| {
            let name = if name == "provenance.json" {
                format!("{}/beaver-provenance.json", package::PACKAGE_ROOT)
            } else {
                name
            };
            ensure!(
                name.starts_with(&format!("{}/", package::PACKAGE_ROOT)),
                "Invalid bundled path"
            );
            Ok((
                name,
                base64::engine::general_purpose::STANDARD.decode(encoded)?,
            ))
        })
        .collect()
}

pub(super) struct Plan {
    pub source_commit: String,
    pub runtime: Value,
    pub runtime_hash: String,
    pub before: BTreeMap<String, String>,
    pub after: BTreeMap<String, Vec<u8>>,
    pub changed: Vec<String>,
}

pub(super) fn plan(root: &Path) -> Result<Plan> {
    let (config, runtime_hash) = identity(root)?;
    ensure!(
        config["schemaVersion"] == 1 && config["packages"]["npr-characters"]["status"] == "ready",
        "NPR runtime is not ready"
    );
    let old = &config["packages"]["npr-characters"];
    let source_commit = old["sourceCommit"]
        .as_str()
        .context("Missing old source commit")?
        .to_owned();
    old_manifest(&source_commit)?;
    ensure!(
        old["sourcePath"] == package::PACKAGE_ROOT
            && old["contractVersion"] == "1.1.0"
            && old["pluginVersion"] == "1.3.0",
        "Unsupported NPR upgrade source; only compiled previous pins are supported"
    );
    ensure!(
        !safe_path(root, "addons/npr_characters")?.exists(),
        "Legacy package must not coexist"
    );
    let before = old_hashes(&source_commit)?;
    for (relative, expected) in &before {
        let path = safe_path(root, relative)?;
        ensure!(
            fs::symlink_metadata(&path)?.is_file(),
            "Not a regular dependency: {relative}"
        );
        ensure!(
            file_hash_limited(&path, Some(16 * 1024 * 1024))?.as_ref() == Some(expected),
            "Customized or missing NPR dependency; upgrade refused: {relative}"
        );
    }
    let after = bundled()?;
    let mut changed = Vec::new();
    for (relative, bytes) in &after {
        if let Some(expected) = before.get(relative) {
            if *expected != hash(bytes) {
                changed.push(relative.clone());
            }
        } else {
            ensure!(
                !safe_path(root, relative)?.exists(),
                "New NPR path conflicts with an existing file: {relative}"
            );
            changed.push(relative.clone());
        }
    }
    changed.extend(
        before
            .keys()
            .filter(|name| !after.contains_key(*name))
            .cloned(),
    );
    changed.sort();
    let mut updated = config;
    updated["packages"]["npr-characters"]["sourceCommit"] = json!(package::SOURCE_COMMIT);
    Ok(Plan {
        source_commit,
        runtime: updated,
        runtime_hash,
        before,
        after,
        changed,
    })
}
