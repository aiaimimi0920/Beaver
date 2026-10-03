use anyhow::{ensure, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    sync::LazyLock,
};

const GUT: &[u8] = include_bytes!("../../../../resources/validation/gut-9.4.0.zip");
const GUT_SHA256: &str = "c54428c250f55a2945282ce462f1aecffac0133a7af2c459ac2ed286ddd0b83e";

// Derive identities only from the compiled-in, verified upstream archive.
// A broken bundle fails closed; project files cannot add trusted identities.
static GUT_HASHES: LazyLock<BTreeMap<String, String>> =
    LazyLock::new(|| gut_hashes(GUT).unwrap_or_default());

fn gut_hashes(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    ensure!(
        format!("{:x}", Sha256::digest(bytes)) == GUT_SHA256,
        "Bundled GUT checksum mismatch"
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut hashes = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        // The pinned ZIP uses Windows separators; project keys stay canonical.
        let path = entry.name().replace('\\', "/");
        ensure!(
            path.starts_with("addons/gut/")
                && !path.contains([':', '\0'])
                && !path.split('/').any(|part| matches!(part, "" | "." | ".."))
                && entry.size() <= 20 * 1024 * 1024,
            "Invalid bundled GUT archive member"
        );
        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        let hash = format!("{:x}", Sha256::digest(&content));
        ensure!(hashes.insert(path, hash).is_none(), "Duplicate GUT member");
    }
    Ok(hashes)
}

static NPR_HASHES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../resources/packages/npr-characters/provenance.json"
    ))
    .expect("bundled NPR provenance must be valid");
    serde_json::from_value(manifest["files"].clone())
        .expect("bundled NPR file hashes must be valid")
});

pub fn immutable(path: &str, bytes: &[u8]) -> bool {
    if path.starts_with("addons/gut/") {
        return GUT_HASHES
            .get(path)
            .is_some_and(|expected| *expected == format!("{:x}", Sha256::digest(bytes)));
    }
    let relative = path
        .strip_prefix("resources/packages/npr-characters/")
        .unwrap_or(path);
    let Some(name) = relative.strip_prefix("addons/npr_character_frame/") else {
        return false;
    };
    NPR_HASHES
        .get(name)
        .is_some_and(|expected| *expected == format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
#[path = "vendor_tests.rs"]
mod tests;
