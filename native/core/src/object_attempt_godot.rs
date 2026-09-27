//! A bounded host-owned import after the model writer has stopped. No session
//! can be reattached or replayed from a persisted attempt.
use crate::{
    execution_settings::ExecutionSettings, executor::Control, object_attempt::State,
    rpc::process_tree::ProcessTree,
};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::{process::Command, sync::mpsc};

pub struct Import {
    pub executable: PathBuf,
    pub timeout: Duration,
}

pub(crate) fn configured(settings: &ExecutionSettings) -> Result<Option<Import>> {
    if settings.mcp["godot"] != true {
        return Ok(None);
    }
    Ok(Some(Import {
        executable: PathBuf::from(settings.tools["godot"].as_str().unwrap_or("")),
        timeout: Duration::from_secs(120),
    }))
}

pub(crate) async fn run(
    import: Import,
    cwd: &Path,
    home: &Path,
    cancelled: &AtomicBool,
    controls: &mut mpsc::Receiver<Control>,
) -> Result<(State, Option<String>), String> {
    if cancelled.load(Ordering::SeqCst) {
        return Ok(interrupted());
    }
    // Asset-only workspaces have no Godot project to import. Never search parent
    // directories: those may contain the original project or another run.
    if !cwd.join("project.godot").is_file() {
        return Ok((State::AwaitingGate, None));
    }
    let executable = match crate::tools::find("godot", &import.executable.to_string_lossy()) {
        Ok(path) => path,
        Err(_) => {
            return Ok((
                State::Failed,
                Some("OBJECT_ATTEMPT_GODOT_UNAVAILABLE".into()),
            ))
        }
    };
    let mut command = Command::new(executable);
    command
        .args(["--headless", "--editor", "--import", "--quit", "--path"])
        .arg(cwd)
        .current_dir(cwd);
    fs::create_dir_all(home).map_err(|error| error.to_string())?;
    execute(command, home, import.timeout, cancelled, controls).await
}

pub(crate) async fn execute(
    mut command: Command,
    home: &Path,
    timeout: Duration,
    cancelled: &AtomicBool,
    controls: &mut mpsc::Receiver<Control>,
) -> Result<(State, Option<String>), String> {
    if cancelled.load(Ordering::SeqCst) {
        return Ok(interrupted());
    }
    let log_path = home.join("godot-import.log");
    let log = fs::File::create(&log_path).map_err(|error| error.to_string())?;
    command
        .stdin(Stdio::null())
        .stderr(log.try_clone().map_err(|error| error.to_string())?)
        .stdout(log)
        .kill_on_drop(true);
    let tree = ProcessTree::new()?;
    tree.configure(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| "OBJECT_ATTEMPT_GODOT_SPAWN_FAILED")?;
    tree.attach(&child)?;
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    let result = loop {
        tokio::select! {
            biased;
            control = controls.recv() => match control {
                None | Some(Control::Interrupt) => break interrupted(),
                Some(Control::Steer { reply, .. }) => {
                    let _ = reply.send(Err("OBJECT_ATTEMPT_DEFINITION_FROZEN".into()));
                }
            },
            _ = &mut deadline => break failed("OBJECT_ATTEMPT_GODOT_TIMEOUT", &log_path),
            status = child.wait() => break match status {
                Ok(status) if status.success() => (State::AwaitingGate, None),
                _ => failed("OBJECT_ATTEMPT_GODOT_IMPORT_FAILED", &log_path),
            },
        }
    };
    // Root exit is insufficient: importer/editor plugins may have descendants.
    // Any failure here leaves Running/no checkpoint, preserving recovery ownership.
    tree.close().await?;
    child
        .wait()
        .await
        .map_err(|_| "OBJECT_ATTEMPT_GODOT_WAIT_FAILED")?;
    if cancelled.load(Ordering::SeqCst) {
        Ok(interrupted())
    } else {
        Ok(result)
    }
}

fn interrupted() -> (State, Option<String>) {
    (
        State::Interrupted,
        Some("OBJECT_ATTEMPT_INTERRUPTED".into()),
    )
}

fn failed(code: &str, log: &Path) -> (State, Option<String>) {
    (
        State::Failed,
        Some(format!("{code}; log: {}", log.display())),
    )
}
