use anyhow::{ensure, Context, Result};
use beaver_core::rpc::Rpc;
use serde_json::json;
use std::path::PathBuf;
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let executable = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("Codex executable required")?,
    );
    ensure!(executable.is_file(), "Codex executable missing");
    let output = PathBuf::from(std::env::args_os().nth(2).context("output root required")?);
    let home = output.join(format!("home-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&home)?;
    let home = std::fs::canonicalize(home)?;
    let mut command = Command::new(&executable);
    command
        .args(["app-server", "--listen", "stdio://"])
        .env("CODEX_HOME", &home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .current_dir(&home);
    let (rpc, _events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
    let initialized = rpc.initialize().await;
    let stopped = rpc.close().await;
    let result = initialized.map_err(anyhow::Error::msg)?;
    stopped.map_err(anyhow::Error::msg)?;
    ensure!(result.is_object(), "initialize returned non-object");
    let proof = json!({"passed":true,"codexExecutable":executable,"isolatedHome":home,"checks":["real Codex app-server spawned directly from Rust", "initialize and initialized over stdio", "owned server stopped and waited"],"modelRequestSent":false,"taskExecutionVerified":false});
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!("{proof}");
    Ok(())
}
