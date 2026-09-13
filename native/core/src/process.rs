use anyhow::{bail, Result};
use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

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

pub struct Output {
    pub code: i32,
    pub text: String,
}

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
    if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    let mut output = tempfile::tempfile()?;
    let mut command = Command::new(executable);
    command
        .args(args)
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
    let mut child = OwnedChild(command.spawn()?);
    let deadline = Instant::now() + timeout;
    let limit = 8 * 1024 * 1024;
    let status = loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            bail!("工具操作已取消");
        }
        if output.metadata()?.len() > limit {
            bail!("工具输出超过限制");
        }
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            bail!("工具执行超时，已中止本次进程");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit as usize {
        bail!("工具输出超过限制");
    }
    Ok(Output {
        code: status.code().unwrap_or(-1),
        text: String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_owned(),
    })
}
