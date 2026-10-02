//! Process ownership only; environment isolation is not an OS sandbox.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
#[cfg(windows)]
#[path = "external_tools_windows.rs"]
mod windows;

pub(super) const LOG_LIMIT: u64 = 8 * 1024 * 1024;
pub(super) struct Owned {
    child: Child,
    reaped: bool,
    #[cfg(windows)]
    tree: windows::Tree,
}

impl Owned {
    pub(super) fn spawn(
        executable: &Path,
        arguments: &[String],
        cwd: &Path,
        directory: &Path,
    ) -> Result<Self> {
        let home = directory.join("home");
        let temporary = directory.join("tmp");
        fs::create_dir(&home)?;
        fs::create_dir(&temporary)?;
        let log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("process.log"))?;
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .current_dir(cwd)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("TMPDIR", &temporary)
            .env("TMP", &temporary)
            .env("TEMP", &temporary)
            .env("BLENDER_USER_CONFIG", home.join("blender-config"))
            .env("BLENDER_USER_SCRIPTS", home.join("blender-scripts"))
            .env("PYTHONNOUSERSITE", "1")
            .env("LANG", "C.UTF-8")
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        let tree = {
            // Windows needs its installation root for system DLL loading.
            if let Some(root) = std::env::var_os("SystemRoot") {
                command.env("SystemRoot", root);
            }
            let tree = windows::Tree::new()?;
            tree.configure(&mut command);
            tree
        };
        let child = command
            .spawn()
            .context("Could not start Beaver-owned Blender")?;
        let mut owned = Self {
            child,
            reaped: false,
            #[cfg(windows)]
            tree,
        };
        #[cfg(windows)]
        if let Err(error) = owned.tree.attach(&owned.child) {
            let _ = owned.child.kill();
            let _ = owned.child.wait();
            owned.reaped = true;
            return Err(error);
        }
        Ok(owned)
    }

    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    fn exited(&mut self) -> Result<bool> {
        #[cfg(unix)]
        unsafe {
            // Observe without reaping: the PID/PGID remains ours until group cleanup.
            let mut information: libc::siginfo_t = std::mem::zeroed();
            let result = libc::waitid(
                libc::P_PID,
                self.id(),
                &mut information,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            );
            if result != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(information.si_pid() != 0)
        }
        #[cfg(windows)]
        {
            self.tree.exited(&self.child)
        }
        #[cfg(not(any(unix, windows)))]
        {
            anyhow::bail!("Managed Blender process ownership unsupported on this platform")
        }
    }

    fn close(&mut self) -> Result<ExitStatus> {
        ensure!(!self.reaped, "Owned Blender was already reaped");
        #[cfg(unix)]
        unsafe {
            let result = libc::kill(-(self.id() as i32), libc::SIGKILL);
            if result != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(windows)]
        self.tree.close()?;
        #[cfg(target_os = "linux")]
        wait_for_group(self.id())?;
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }

    pub(super) fn wait(
        &mut self,
        directory: &Path,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Value> {
        let start = Instant::now();
        let reason = loop {
            if cancelled.load(Ordering::SeqCst) {
                break "cancelled";
            }
            if fs::metadata(directory.join("process.log"))?.len() > LOG_LIMIT {
                break "log_limit";
            }
            if self.exited()? {
                break "exited";
            }
            if start.elapsed() >= timeout {
                break "timed_out";
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let status = self.close()?;
        Ok(
            json!({"termination":reason,"exitCode":status.code(),"success":reason == "exited" && status.success(),"durationMs":start.elapsed().as_millis() as u64,"ownedTreeCleanup":"completed","ownershipBoundary":if cfg!(windows) { "windows_job_object" } else { "unix_process_group" }}),
        )
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.close();
        }
    }
}

#[cfg(target_os = "linux")]
fn wait_for_group(group: u32) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut live = false;
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
                continue;
            }
            let stat = match fs::read_to_string(entry.path().join("stat")) {
                Ok(stat) => stat,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            // comm may contain spaces and parentheses; the last ')' ends it.
            let Some((_, fields)) = stat.rsplit_once(')') else {
                continue;
            };
            let fields: Vec<_> = fields.split_whitespace().collect();
            if fields.get(2).and_then(|value| value.parse::<u32>().ok()) == Some(group)
                && !fields
                    .first()
                    .is_some_and(|state| ["Z", "X"].contains(state))
            {
                live = true;
                break;
            }
        }
        if !live {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Owned Blender process group cleanup timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
