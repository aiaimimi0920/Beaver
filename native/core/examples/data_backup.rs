use anyhow::{bail, Context, Result};
use beaver_core::data_backup;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args
        .first()
        .context("usage: data_backup create|verify|restore SOURCE [NEW_DESTINATION]")?;
    let source = Path::new(args.get(1).context("source required")?);
    let manifest = match (mode.as_str(), args.len()) {
        ("verify", 2) => data_backup::verify(source)?,
        ("create", 3) => data_backup::create(source, Path::new(&args[2]))?,
        ("restore", 3) => data_backup::restore(source, Path::new(&args[2]))?,
        _ => bail!("usage: data_backup create|verify|restore SOURCE [NEW_DESTINATION]"),
    };
    println!(
        "{}",
        serde_json::json!({"format":manifest.format,"scope":manifest.scope,"entries":manifest.entries.len(),"bytes":manifest.entries.iter().map(|entry| entry.bytes).sum::<u64>()})
    );
    Ok(())
}
