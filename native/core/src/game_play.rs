use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    sync::oneshot,
    task::JoinHandle,
};

#[derive(Default)]
struct State {
    closing: bool,
    sequence: u64,
    jobs: BTreeMap<u64, Job>,
}
struct Job {
    stop: oneshot::Sender<()>,
    task: JoinHandle<Result<()>>,
}
#[derive(Clone, Default)]
pub struct Players(Arc<Mutex<State>>);

#[cfg(windows)]
struct ChildGroup {
    _handle: std::os::windows::io::OwnedHandle,
}
#[cfg(not(windows))]
struct ChildGroup(u32);

impl ChildGroup {
    fn attach(child: &Child) -> Result<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
            use windows_sys::Win32::System::JobObjects::*;
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if raw.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                SetInformationJobObject(
                    handle.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&info) as u32,
                )
            };
            if configured == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let process = child.raw_handle().context("游戏进程句柄不可用")?;
            if unsafe { AssignProcessToJobObject(handle.as_raw_handle(), process) } == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self { _handle: handle })
        }
        #[cfg(not(windows))]
        {
            Ok(Self(child.id().context("游戏进程标识不可用")?))
        }
    }
}

#[cfg(unix)]
impl Drop for ChildGroup {
    fn drop(&mut self) {
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}

async fn drain(mut reader: impl AsyncRead + Unpin, log: Arc<Mutex<Vec<u8>>>) {
    let mut bytes = [0; 4096];
    while let Ok(length) = reader.read(&mut bytes).await {
        if length == 0 {
            break;
        }
        let mut log = log.lock().unwrap();
        log.extend_from_slice(&bytes[..length]);
        let discard = log.len().saturating_sub(8000);
        log.drain(..discard);
    }
}

async fn terminate(child: &mut Child) -> Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    if let Some(pid) = child.id() {
        #[cfg(windows)]
        {
            let executable = std::path::PathBuf::from(
                std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
            )
            .join("System32/taskkill.exe");
            let mut killer = Command::new(executable);
            killer
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x08000000)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            let _ = tokio::time::timeout(Duration::from_secs(5), killer.status()).await;
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    let _ = child.start_kill();
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .context("游戏进程退出超时")??;
    Ok(())
}

async fn supervise(
    mut child: Child,
    group: ChildGroup,
    mut stop: oneshot::Receiver<()>,
    ready: oneshot::Sender<Result<(), String>>,
) -> Result<()> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let stdout = tokio::spawn(drain(
        child.stdout.take().context("游戏输出管道不可用")?,
        log.clone(),
    ));
    let stderr = tokio::spawn(drain(
        child.stderr.take().context("游戏错误管道不可用")?,
        log.clone(),
    ));
    let mut startup = Some(ready);
    let startup_timer = tokio::time::sleep(Duration::from_millis(900));
    tokio::pin!(startup_timer);
    let result = loop {
        tokio::select! {
            biased;
            _ = &mut stop => break terminate(&mut child).await,
            exit = child.wait() => {
                break exit.map(|_| ()).map_err(anyhow::Error::from);
            }
            _ = &mut startup_timer, if startup.is_some() => {
                if startup.take().unwrap().send(Ok(())).is_err() {
                    break terminate(&mut child).await;
                }
            }
        }
    };
    // Closing the owned Windows job also cleans descendants whose parent already exited.
    drop(group);
    // A plugin retaining an inherited pipe must not prevent application shutdown.
    let mut stdout = stdout;
    let mut stderr = stderr;
    if tokio::time::timeout(Duration::from_secs(1), &mut stdout)
        .await
        .is_err()
    {
        stdout.abort();
    }
    if tokio::time::timeout(Duration::from_secs(1), &mut stderr)
        .await
        .is_err()
    {
        stderr.abort();
    }
    if let Some(ready) = startup {
        let tail = String::from_utf8_lossy(&log.lock().unwrap()).into_owned();
        let _ = ready.send(Err(format!("游戏启动后退出或已取消，请检查项目。\n{tail}")));
    }
    result
}

impl Players {
    pub async fn play(&self, executable: &Path, project: &Path) -> Result<()> {
        let (ready, started) = oneshot::channel();
        {
            let mut state = self
                .0
                .lock()
                .map_err(|_| anyhow::anyhow!("游戏运行锁不可用"))?;
            if state.closing {
                bail!("应用正在退出");
            }
            let mut command = Command::new(executable);
            command
                .arg("--path")
                .arg(project)
                .current_dir(project)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            #[cfg(windows)]
            command.creation_flags(0x08000000);
            #[cfg(unix)]
            command.process_group(0);
            let mut child = command.spawn().context("无法启动游戏")?;
            let group = match ChildGroup::attach(&child) {
                Ok(group) => group,
                Err(error) => {
                    let _ = child.start_kill();
                    return Err(error.context("无法建立游戏进程清理边界"));
                }
            };
            state.sequence += 1;
            let id = state.sequence;
            let (stop, stopped) = oneshot::channel();
            let task = tokio::spawn(supervise(child, group, stopped, ready));
            state.jobs.retain(|_, job| !job.task.is_finished());
            state.jobs.insert(id, Job { stop, task });
        }
        started
            .await
            .context("游戏启动检查中断")?
            .map_err(anyhow::Error::msg)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let jobs = {
            let mut state = self
                .0
                .lock()
                .map_err(|_| anyhow::anyhow!("游戏运行锁不可用"))?;
            state.closing = true;
            std::mem::take(&mut state.jobs)
        };
        let mut tasks = Vec::new();
        for (_, job) in jobs {
            let _ = job.stop.send(());
            tasks.push(job.task);
        }
        let mut error = None;
        for task in tasks {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(value)) => error = Some(value),
                Err(value) => error = Some(value.into()),
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn output_keeps_only_bounded_tail() {
        let mut bytes = vec![b'x'; 100_000];
        bytes.extend_from_slice(b"last startup diagnostic");
        let log = Arc::new(Mutex::new(Vec::new()));
        drain(bytes.as_slice(), log.clone()).await;
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 8000);
        assert!(log.ends_with(b"last startup diagnostic"));
    }

    #[tokio::test]
    async fn closed_player_rejects_launch_before_touching_executable() -> Result<()> {
        let players = Players::default();
        players.shutdown().await?;
        players.shutdown().await?;
        let error = players
            .play(Path::new("missing"), Path::new("missing"))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("应用正在退出"));
        Ok(())
    }
}
