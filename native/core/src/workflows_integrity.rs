use super::package::{PACKAGE_ROOT, SOURCE_COMMIT};
use crate::files::{file_hash_limited, safe_path};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, sync::LazyLock};

// Only the manifest compiled into Beaver is authoritative. The project's copy
// exists for human provenance inspection and can never authorize changed code.
static PINNED_HASHES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let provenance: Value = serde_json::from_str(include_str!(
        "../../../resources/packages/npr-characters/provenance.json"
    ))
    .expect("bundled NPR provenance must be valid");
    assert_eq!(provenance["sourcePath"], PACKAGE_ROOT);
    assert_eq!(provenance["sourceCommit"], SOURCE_COMMIT);
    serde_json::from_value(provenance["files"].clone())
        .expect("bundled NPR provenance hashes must be valid")
});

/// Check every pinned dependency without trusting the project's manifest or
/// rewriting user changes. safe_path rejects file and ancestor links/junctions.
pub fn verify_package(root: &Path) -> Result<()> {
    for (name, expected) in PINNED_HASHES.iter() {
        let relative = format!("{PACKAGE_ROOT}/{name}");
        let path = safe_path(root, &relative)
            .with_context(|| format!("Unsafe NPR package dependency: {relative}"))?;
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("Missing NPR package dependency: {relative}"))?;
        ensure!(
            metadata.is_file(),
            "NPR package dependency is not a regular file: {relative}"
        );
        let actual = file_hash_limited(&path, Some(16 * 1024 * 1024))?;
        ensure!(actual.as_deref() == Some(expected.as_str()),
            "NPR package dependency was modified: {relative}; restore the pinned package or explicitly migrate");
    }
    Ok(())
}
