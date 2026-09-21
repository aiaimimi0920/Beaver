use anyhow::{Context, Result};
use beaver_core::{files::Files, reveal, store::Store};
use serde_json::json;
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("output directory required")?,
    );
    std::fs::create_dir_all(&root)?;
    let root = std::fs::canonicalize(root)?;
    let data = tempfile::tempdir()?;
    let store = Store::open(data.path())?;
    let id = uuid::Uuid::new_v4().to_string();
    let name = "reveal-fixture.txt";
    std::fs::write(root.join(name), b"file-manager selection fixture")?;
    store.put("project", &id, &json!({"path":root}))?;
    let target = reveal::resolve(
        &store,
        &Files::new(data.path().to_owned()),
        "asset.reveal",
        &json!({"id":id,"path":name}),
    )?;
    reveal::open(&target)?;
    println!("{}", reveal::windows_path(&target.path)?);
    Ok(())
}
