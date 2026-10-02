use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn media_mode_uses_core_stdio_without_starting_business_host() -> Result<()> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_beaver-headless"))
        .arg("--media-mcp")
        .env_remove("BEAVER_PROJECT_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().context("Missing stdin")?;
    let mut output = BufReader::new(child.stdout.take().context("Missing stdout")?).lines();
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .await?;
    let line = tokio::time::timeout(Duration::from_secs(10), output.next_line())
        .await??
        .context("Missing Core MCP response")?;
    let response: Value = serde_json::from_str(&line)?;
    drop(input);
    assert!(tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await??
        .success());
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["serverInfo"]["name"], "beaver-media");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn sigterm_exits_with_stdin_open_and_releases_instance_lock() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_beaver-headless"))
        .arg("--data-dir")
        .arg(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().context("Missing stdin")?;
    let mut output = BufReader::new(child.stdout.take().context("Missing stdout")?).lines();
    input
        .write_all(b"{\"id\":1,\"method\":\"state\"}\n")
        .await?;
    let ready = tokio::time::timeout(Duration::from_secs(10), output.next_line())
        .await??
        .context("Missing response")?;
    assert!(serde_json::from_str::<Value>(&ready)?["result"].is_object());
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().context("Missing PID")?.to_string())
        .status()
        .await?;
    assert!(status.success());
    // Keep stdin open: graceful termination must not need another byte or EOF.
    assert!(tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await??
        .success());
    drop(input);
    let host = beaver_headless::Host::open(temp.path())?;
    assert_eq!(host.call("state", json!({})).await?["projects"], json!([]));
    host.shutdown().await?;
    Ok(())
}
