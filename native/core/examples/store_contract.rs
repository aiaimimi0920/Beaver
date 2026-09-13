use anyhow::{bail, Context, Result};
use beaver_core::store::Store;
use serde_json::{json, Value};
use std::path::PathBuf;

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("fixture directory required")?,
    );
    if !root.join("migration-fixture.json").is_file() {
        bail!("only an explicitly marked generated fixture may be modified");
    }
    let mut store = Store::open(&root)?;
    let project = store
        .get::<Value>("project", "fixture")?
        .context("missing TypeScript fixture")?;
    if project["name"] != "迁移验证" || project["future"]["preserve"] != true {
        bail!("legacy fields changed");
    }
    if store.get::<String>("secret", "fixture")?.as_deref() != Some("NOT-A-REAL-SECRET") {
        bail!("opaque secret preservation failed");
    }
    if store.recover_tasks()? != 1 {
        bail!("unexpected recovery count");
    }
    store.put(
        "nativeProof",
        "fixture",
        &json!({"from":"rust","unicode":"原样保留"}),
    )?;
    store.event("fixture", "2026-09-07T00:00:00.000Z", "native", "Rust 事件")?;
    println!("NATIVE_STORE_CONTRACT_OK");
    Ok(())
}
