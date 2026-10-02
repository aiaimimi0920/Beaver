use anyhow::{anyhow, bail, Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;

pub(crate) fn terminate(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let killer = std::path::PathBuf::from(
            std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
        )
        .join("System32/taskkill.exe");
        if let Ok(mut killer) = Command::new(killer)
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            let deadline = Instant::now() + Duration::from_secs(3);
            while killer.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = killer.kill();
            let _ = killer.wait();
        }
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) struct OwnedChild(pub Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        terminate(&mut self.0);
    }
}

// The captured runner consumes ownership before explicit cleanup, so a cleanup
// error cannot cause Drop to signal the same PID again.
struct CapturedChild(Option<Child>);
impl Drop for CapturedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            terminate(child);
        }
    }
}

pub struct Output {
    pub code: i32,
    pub text: String,
}

#[derive(Debug)]
pub(crate) struct ProcessLogError {
    reason: String,
    log: PathBuf,
}
impl std::fmt::Display for ProcessLogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}; process log: {}",
            self.reason,
            self.log.display()
        )
    }
}
impl std::error::Error for ProcessLogError {}

/// Blocking worker only. Disk-backed bounded output avoids pipe backpressure deadlocks.
pub fn run(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
) -> Result<Output> {
    run_cancellable(
        executable,
        args,
        cwd,
        timeout,
        &std::sync::atomic::AtomicBool::new(false),
    )
}

pub fn run_cancellable(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Output> {
    run_cancellable_env(
        executable,
        args,
        cwd,
        timeout,
        cancelled,
        &Default::default(),
    )
}

pub fn run_cancellable_env(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    cancelled: &std::sync::atomic::AtomicBool,
    environment: &std::collections::BTreeMap<String, String>,
) -> Result<Output> {
    if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    run_captured(
        executable,
        args,
        cwd,
        timeout,
        cancelled,
        environment,
        tempfile::tempfile()?,
        None,
    )
}

/// The caller owns this evidence path; never accepts a project-supplied log path.
/// Raw output stays in a bounded host-published snapshot, not the error/repair prompt.
/// The child never holds the final evidence handle, even via an inherited stream.
pub(crate) fn run_cancellable_env_logged(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    cancelled: &std::sync::atomic::AtomicBool,
    environment: &std::collections::BTreeMap<String, String>,
    log: &Path,
) -> Result<Output> {
    (|| {
        let evidence = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(log)
            .context("Cannot create process log")?;
        run_captured(
            executable,
            args,
            cwd,
            timeout,
            cancelled,
            environment,
            tempfile::tempfile()?,
            Some(evidence),
        )
    })()
    .map_err(|error: anyhow::Error| {
        ProcessLogError {
            reason: format!("{error:#}"),
            log: log.into(),
        }
        .into()
    })
}

fn run_captured(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
    cancelled: &std::sync::atomic::AtomicBool,
    environment: &std::collections::BTreeMap<String, String>,
    mut output: File,
    evidence: Option<File>,
) -> Result<Output> {
    if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output.try_clone()?);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut guard = CapturedChild(Some(command.spawn()?));
    let mut result = collect(guard.0.as_mut().unwrap(), &mut output, timeout, cancelled);
    let mut child = guard.0.take().unwrap();
    if result.is_err() {
        // Attempt stop/reap before publishing evidence. The guard is disarmed
        // even if cleanup fails; retain that uncertainty and never return success.
        terminate(&mut child);
        if let Err(cleanup) = child.wait() {
            result = Err(anyhow!(
                "{}; cannot confirm process exit: {cleanup}",
                result.err().unwrap()
            ));
        }
    }
    if let Some(mut evidence) = evidence {
        if let Err(retention) = retain_tail(&mut output, &mut evidence) {
            return Err(match result {
                Err(error) => anyhow!("{error}; cannot publish process log: {retention}"),
                Ok(_) => retention,
            });
        }
    }
    result
}

fn collect(
    child: &mut Child,
    output: &mut File,
    timeout: Duration,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Output> {
    let deadline = Instant::now() + timeout;
    let status = loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            bail!("工具操作已取消");
        }
        if output.metadata()?.len() > OUTPUT_LIMIT {
            bail!("工具输出超过限制");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            bail!("工具执行超时，已中止本次进程");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut bytes = Vec::new();
    copy_range(output, 0, OUTPUT_LIMIT + 1, &mut bytes)?;
    if bytes.len() > OUTPUT_LIMIT as usize {
        bail!("工具输出超过限制");
    }
    Ok(Output {
        code: status.code().unwrap_or(-1),
        text: String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_owned(),
    })
}

fn retain_tail(output: &mut File, evidence: &mut File) -> Result<()> {
    let length = output.metadata()?.len();
    let retained = length.min(OUTPUT_LIMIT);
    // Polling may overshoot. Limit this raw-byte snapshot even if an inherited
    // capture FD outlives its root; it can never grow the host's final evidence.
    copy_range(output, length - retained, retained, evidence)?;
    Ok(())
}

fn copy_range(output: &File, start: u64, limit: u64, target: &mut impl Write) -> Result<()> {
    let mut copied = 0;
    let mut buffer = [0; 8192];
    while copied < limit {
        let size = (limit - copied).min(buffer.len() as u64) as usize;
        // Never follow growth beyond this sampled range. Unix read_at leaves the
        // shared capture cursor alone. Windows seek_read can move that cursor if
        // a descendant survives; only the final snapshot's byte bound is assured.
        #[cfg(unix)]
        let read = {
            use std::os::unix::fs::FileExt;
            output.read_at(&mut buffer[..size], start + copied)?
        };
        #[cfg(windows)]
        let read = {
            use std::os::windows::fs::FileExt;
            output.seek_read(&mut buffer[..size], start + copied)?
        };
        if read == 0 {
            break;
        }
        target.write_all(&buffer[..read])?;
        copied += read as u64;
    }
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "process_tests.rs"]
mod tests;
