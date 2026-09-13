use anyhow::{bail, Context, Result};
use beaver_core::{files::Files, journal::Journal, store::Store};
use serde_json::{json, Value};
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("fixture directory required")?,
    );
    if !root.join("migration-fixture.json").is_file() {
        bail!("generated fixture required");
    }
    let mut store = Store::open(&root)?;
    let files = Files::new(root.clone());
    Journal::new(&mut store, &files).recover()?;
    let task = store
        .get::<Value>("task", "file-task")?
        .context("missing legacy task")?;
    if task["status"] != "completed" || task["direction"] != "story" {
        bail!("legacy task recovery failed");
    }
    let snapshot = files.capture(&root.join("project"))?;
    store.put("nativeProof", "files", &json!({"snapshot":snapshot}))?;
    println!("NATIVE_FILES_CONTRACT_OK");
    Ok(())
}
