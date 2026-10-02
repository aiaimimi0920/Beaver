//! Owned-process fixtures only. These do not claim Blender or character acceptance.
#![cfg(unix)]
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};

type Output = Lines<BufReader<ChildStdout>>;
async fn request(
    input: &mut ChildStdin,
    output: &mut Output,
    method: &str,
    arguments: Value,
) -> Result<Value> {
    let bytes = format!(
        "{}\n",
        json!({"id":method,"method":method,"input":arguments})
    );
    input.write_all(bytes.as_bytes()).await?;
    let line = tokio::time::timeout(Duration::from_secs(15), output.next_line())
        .await??
        .context("Missing JSONL response")?;
    let response: Value = serde_json::from_str(&line)?;
    anyhow::ensure!(response["error"].is_null(), "{response}");
    Ok(response["result"].clone())
}

async fn running_job(root: &Path) -> Result<(Child, ChildStdin, Output, Value)> {
    let executable = root.join("owned-process-fixture");
    fs::write(&executable, "#!/bin/sh\nsleep 120 &\nwait\n")?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_beaver-headless"))
        .arg("--data-dir")
        .arg(root.join("host"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().context("Missing stdin")?;
    let mut output = BufReader::new(child.stdout.take().context("Missing stdout")?).lines();
    let mut settings = request(&mut input, &mut output, "settings.get", json!({})).await?;
    settings["tools"]["blender"] = json!(executable);
    request(
        &mut input,
        &mut output,
        "settings.save",
        json!({"settings":settings}),
    )
    .await?;
    let project = request(
        &mut input,
        &mut output,
        "project.create",
        json!({
            "parent":root,"name":"StopFixture","template":"blank"
        }),
    )
    .await?;
    let task = request(&mut input, &mut output, "task.create", json!({
        "projectId":project["id"],"prompt":"Exercise owned job cancellation","decompose":false,"executionMode":"external-agent"
    })).await?;
    let context = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let pending = request(&mut input, &mut output, "external.pending", json!({})).await?;
            if let Some(context) = pending
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["taskId"] == task["id"])
            {
                return Ok::<_, anyhow::Error>(context.clone());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    let receipt = request(&mut input, &mut output, "external.tool", json!({
        "taskId":task["id"],"runId":context["runId"],"revision":0,"requestId":"start-fixture",
        "arguments":{"tool":"blender.start","arguments":{"code":"# process fixture, never real Blender",
            "outputs":[{"path":"fixture.txt","expectedSha256":null}],"timeoutSeconds":120}}
    })).await?;
    anyhow::ensure!(receipt["status"] == "succeeded", "{receipt}");
    assert_eq!(receipt["result"]["status"], "running");
    Ok((child, input, output, receipt["result"].clone()))
}

async fn verify_stopped(mut child: Child, job: Value, root: &Path) -> Result<()> {
    assert!(tokio::time::timeout(Duration::from_secs(15), child.wait())
        .await??
        .success());
    let directory = Path::new(
        job["process"]["jobDirectory"]
            .as_str()
            .context("Missing job directory")?,
    );
    let receipt: Value = serde_json::from_slice(&fs::read(directory.join("job.json"))?)?;
    assert_eq!(receipt["status"], "cancelled");
    assert_eq!(receipt["result"]["ownedTreeCleanup"], "completed");
    assert!(!Command::new("kill")
        .arg("-0")
        .arg(job["process"]["pid"].as_u64().unwrap().to_string())
        .stderr(Stdio::null())
        .status()
        .await?
        .success());
    let host = beaver_headless::Host::open(&root.join("host"))?;
    assert_eq!(
        host.call("state", json!({})).await?["tasks"][0]["status"],
        "interrupted"
    );
    host.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eof_cancels_owned_external_job_before_releasing_instance_lock() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (child, input, _output, job) = running_job(temp.path()).await?;
    drop(input);
    verify_stopped(child, job, temp.path()).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_cancels_owned_external_job_while_stdin_remains_open() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (child, input, _output, job) = running_job(temp.path()).await?;
    assert!(Command::new("kill")
        .arg("-TERM")
        .arg(child.id().context("Missing host PID")?.to_string())
        .status()
        .await?
        .success());
    let result = verify_stopped(child, job, temp.path()).await;
    drop(input);
    result
}
