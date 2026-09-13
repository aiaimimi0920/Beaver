use anyhow::{Context, Result};
use beaver_core::{tool_install, tools};
use serde_json::json;
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("isolated output directory required")?,
    );
    // This test owns only a newly created directory; never reuse a user profile.
    std::fs::create_dir(&root)?;
    let node = tools::find("node", "")?;
    let cancelled = AtomicBool::new(false);
    let status = tools::detect_one(
        "node",
        node.to_str().context("Node path encoding")?,
        &[],
        &cancelled,
    )?;
    anyhow::ensure!(status["available"] == true, "Node identity check failed");
    tool_install::install("codex", &root, node.to_str().unwrap(), &cancelled)?;
    let binary = tools::native_codex(&root.join("tools")).context("managed Codex not found")?;
    let result = tools::detect_one(
        "codex",
        binary.to_str().context("Codex path encoding")?,
        &[],
        &cancelled,
    )?;
    anyhow::ensure!(
        result["available"] == true,
        "installed Codex identity check failed"
    );
    anyhow::ensure!(
        std::fs::canonicalize(&binary)?.starts_with(std::fs::canonicalize(&root)?),
        "installation escaped owned directory"
    );
    let proof = json!({"checks":["real npm installation into a fresh isolated directory",
        "installed native Codex binary stays inside owned directory", "actual installed executable passes version identity probe"],
        "node":status,"codex":result,"binaryBytes":std::fs::metadata(&binary)?.len(),
        "desktopIpcVerified":false,"zeroEnvironmentVerified":false});
    std::fs::write(root.join("proof.json"), serde_json::to_vec_pretty(&proof)?)?;
    println!("{}", root.join("proof.json").display());
    Ok(())
}
