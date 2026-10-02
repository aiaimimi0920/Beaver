#![cfg(target_os = "linux")]

use anyhow::{Context, Result};
use beaver_headless::Host;
use serde_json::{json, Value};
use std::{
    ffi::{c_int, c_ulong},
    os::fd::AsRawFd,
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

// Linux pipe controls let the test prove backpressure without consuming output.
// Values are from linux/fcntl.h and asm-generic/ioctls.h; no production FFI is added.
unsafe extern "C" {
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn shrink_pipe(fd: c_int) -> Result<c_int> {
    let capacity = unsafe { fcntl(fd, 1031, 4096 as c_int) };
    anyhow::ensure!(
        capacity >= 4096,
        "Could not set test pipe size: {}",
        std::io::Error::last_os_error()
    );
    Ok(capacity)
}

fn unread_bytes(fd: c_int) -> Result<c_int> {
    let mut bytes: c_int = 0;
    let result = unsafe { ioctl(fd, 0x541B as c_ulong, &mut bytes as *mut c_int) };
    anyhow::ensure!(
        result == 0,
        "Could not inspect test pipe: {}",
        std::io::Error::last_os_error()
    );
    Ok(bytes)
}

async fn stop_with_full_stdout(pipeline_overflow: bool) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let mut child = Command::new(env!("CARGO_BIN_EXE_beaver-headless"))
        .arg("--data-dir")
        .arg(&data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().context("Missing stdin")?;
    let stdout = child.stdout.take().context("Missing stdout")?;
    let fd = stdout.as_raw_fd();
    let capacity = shrink_pipe(fd)?;
    let mut output = BufReader::new(stdout).lines();
    let create = json!({"id":1,"method":"project.create","input":{
        "parent":temp.path(),"name":"BlockedOutput","template":"blank"
    }});
    input.write_all(format!("{create}\n").as_bytes()).await?;
    let line = tokio::time::timeout(Duration::from_secs(10), output.next_line())
        .await??
        .context("Missing project response")?;
    let created: Value = serde_json::from_str(&line)?;
    let project_id = created["result"]["id"]
        .as_str()
        .context("Project creation failed")?;
    let prompt = "Check the project without making changes. ".repeat(500);
    assert!(prompt.len() > capacity as usize * 2);
    let task = json!({"id":2,"method":"task.create","input":{
        "projectId":project_id,"prompt":prompt,"decompose":false
    }});
    tokio::time::timeout(
        Duration::from_secs(10),
        input.write_all(format!("{task}\n").as_bytes()),
    )
    .await??;
    // No further output is read. A full pipe proves the response writer is blocked,
    // rather than merely sending SIGTERM while the host is idle or still starting.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if unread_bytes(fd)? == capacity {
                return Ok::<(), anyhow::Error>(());
            }
            anyhow::ensure!(
                child.try_wait()?.is_none(),
                "Host exited before stdout filled"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    let keep_input_open = if pipeline_overflow {
        // Two further complete frames reproduce the old queue-capacity deadlock:
        // the first occupied the queue, the second hid EOF in the reader buffer.
        for name in ["UnadmittedA", "UnadmittedB"] {
            let next = json!({"id":name,"method":"project.create","input":{
                "parent":temp.path(),"name":name,"template":"blank"
            }});
            input.write_all(format!("{next}\n").as_bytes()).await?;
        }
        drop(input);
        None
    } else {
        assert!(Command::new("kill")
            .arg("-TERM")
            .arg(child.id().context("Missing PID")?.to_string())
            .status()
            .await?
            .success());
        // Keep stdin open and stdout unread until the signal-driven exit.
        Some(input)
    };
    let status = tokio::time::timeout(Duration::from_secs(10), child.wait()).await??;
    drop(keep_input_open);
    if pipeline_overflow {
        assert!(
            !status.success(),
            "Pipeline overflow must be an explicit transport failure"
        );
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .context("Missing stderr")?
            .read_to_string(&mut stderr)
            .await?;
        assert!(stderr.contains("request pipeline exceeded"), "{stderr}");
        assert!(!temp.path().join("UnadmittedA").exists());
        assert!(!temp.path().join("UnadmittedB").exists());
    } else {
        assert!(status.success());
    }
    drop(output);
    let host = Host::open(&data)?;
    let state = host.call("state", json!({})).await?;
    let tasks = state["tasks"].as_array().context("Missing tasks")?;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["prompt"], prompt);
    assert_eq!(tasks[0]["projectId"], project_id);
    assert!(matches!(
        tasks[0]["status"].as_str(),
        Some("failed" | "interrupted")
    ));
    assert!(tasks[0]["threadId"].is_null());
    host.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn sigterm_drains_committed_mutation_with_full_unread_stdout_pipe() -> Result<()> {
    stop_with_full_stdout(false).await
}

#[tokio::test]
async fn eof_with_full_stdout_and_two_unadmitted_frames_cannot_deadlock() -> Result<()> {
    stop_with_full_stdout(true).await
}
