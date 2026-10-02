use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::LazyLock};

static NPR_HASHES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../resources/packages/npr-characters/provenance.json"
    ))
    .expect("bundled NPR provenance must be valid");
    serde_json::from_value(manifest["files"].clone())
        .expect("bundled NPR file hashes must be valid")
});

pub fn immutable(path: &str, bytes: &[u8]) -> bool {
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
