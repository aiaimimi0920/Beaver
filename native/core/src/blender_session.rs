use crate::{
    call_log,
    files::{safe_path, Files},
    process::OwnedChild,
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub struct Request {
    executable: PathBuf,
    workspace: PathBuf,
    directory: PathBuf,
    checkpoints: PathBuf,
    reservation: TcpListener,
    pub port: u16,
}

#[path = "blender_addon.rs"]
mod addon;

impl Request {
    /// Reserve a distinct port while Codex configuration is prepared. No Blender starts here.
    pub fn prepare(home: &Path, workspace: &Path, executable: &Path) -> Result<Self> {
        if !executable.is_file() {
            bail!("Configure an installed Blender editor for NPR production");
        }
        let reservation = TcpListener::bind("127.0.0.1:0")?;
        let port = reservation.local_addr()?.port();
        let directory = safe_path(home, &format!("blender/{}", uuid::Uuid::new_v4()))?;
        fs::create_dir_all(&directory)?;
        Ok(Self {
            executable: executable.to_owned(),
            workspace: workspace.to_owned(),
            directory,
            checkpoints: safe_path(home, "asset-checkpoints")?,
            reservation,
            port,
        })
    }

    fn start(
        self,
        cancelled: &AtomicBool,
        observer: &Value,
        restore: Option<&str>,
    ) -> Result<(OwnedChild, u16)> {
        if cancelled.load(Ordering::SeqCst) {
            bail!("Blender preparation interrupted");
        }
        let ready = self.directory.join("ready.json");
        let request = self.directory.join("request.json");
        fs::write(
            &request,
            serde_json::to_vec(
                &json!({"addon":addon::installed()?,"port":self.port,"ready":ready,"observer":observer,"restore":restore}),
            )?,
        )?;
        let script = self.directory.join("start.py");
        fs::write(
            &script,
            include_bytes!("../../../resources/workflows/blender_session.py"),
        )?;
        for (name, bytes) in [
            (
                "asset_observer.py",
                include_bytes!("../../../resources/workflows/asset_observer.py").as_slice(),
            ),
            (
                "asset_observer_view.py",
                include_bytes!("../../../resources/workflows/asset_observer_view.py").as_slice(),
            ),
            (
                "asset_observer_transport.py",
                include_bytes!("../../../resources/workflows/asset_observer_transport.py")
                    .as_slice(),
            ),
        ] {
            fs::write(self.directory.join(name), bytes)?;
        }
        let output = fs::File::create(self.directory.join("blender.log"))?;
        for folder in ["config", "scripts", "temp"] {
            fs::create_dir_all(self.directory.join(folder))?;
        }
        let mut command = Command::new(&self.executable);
        command
            .args([
                "--factory-startup",
                "--disable-autoexec",
                "--python-exit-code",
                "1",
                "--python",
            ])
            .arg(&script)
            .current_dir(&self.workspace)
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .env_clear()
            .envs(std::env::vars_os().filter(|(key, _)| {
                let key = key.to_string_lossy().to_ascii_uppercase();
                !["BEAVER_", "CODEX_", "OPENAI_", "BLENDERMCP_", "BLENDER_"]
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
            }))
            .env("BLENDER_USER_CONFIG", self.directory.join("config"))
            .env("BLENDER_USER_SCRIPTS", self.directory.join("scripts"))
            .env("TEMP", self.directory.join("temp"))
            .env("TMP", self.directory.join("temp"))
            .env("BEAVER_BLENDER_REQUEST", request);
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
        drop(self.reservation);
        let mut child = OwnedChild(command.spawn()?);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if cancelled.load(Ordering::SeqCst) {
                bail!("Blender preparation interrupted");
            }
            if let Some(status) = child.0.try_wait()? {
                bail!(
                    "Blender exited during preparation ({status}); see {}",
                    self.directory.join("blender.log").display()
                );
            }
            if ready.is_file() {
                let status: Value = serde_json::from_slice(&fs::read(&ready)?)?;
                if status["pid"] != child.0.id() || status["port"] != self.port {
                    bail!("Blender session identity mismatch");
                }
                if ping(self.port).is_ok() {
                    let observer_port = status["observerPort"]
                        .as_u64()
                        .filter(|port| *port > 0 && *port <= 65535)
                        .context("Missing observer port")?;
                    return Ok((child, observer_port as u16));
                }
            }
            if Instant::now() >= deadline {
                bail!(
                    "Blender MCP did not become ready; see {}",
                    self.directory.join("blender.log").display()
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn start_logged(
        self,
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        task: &Value,
        cancelled: &AtomicBool,
    ) -> Result<Session> {
        let started = Instant::now();
        let task_id = task["id"].as_str().context("Missing task id")?.to_owned();
        let project = task["projectId"].as_str().map(str::to_owned);
        let port = self.port;
        let session_id = uuid::Uuid::new_v4().to_string();
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let checkpoints = self.checkpoints.clone();
        files.check_checkpoint_directory(&task_id, &checkpoints)?;
        let observer = json!({"taskId":task_id,"projectId":project,"sessionId":session_id,"token":token,"checkpoints":checkpoints});
        let restore = {
            let db = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            crate::asset_task::enable(&db, task)?
                .checkpoint
                .map(|path| files.resolve_checkpoint(&path))
                .transpose()?
        };
        let log = self.directory.join("blender.log");
        let id = {
            let db = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            call_log::begin(
                &db,
                "workflow",
                "blender.session.start",
                Some(&task_id),
                project.as_deref(),
                &json!({"port":port}),
            )?
        };
        let restore_path = restore.as_ref().map(|path| path.to_string_lossy());
        let result = self.start(cancelled, &observer, restore_path.as_deref());
        let output = match &result {
            Ok((child, _)) => {
                json!({"pid":child.0.id(),"port":port,"log":log,"sessionId":session_id,"restore":restore})
            }
            Err(error) => json!({"error":error.to_string()}),
        };
        if let Ok(db) = store.lock() {
            let _ = call_log::finish(
                &db,
                &id,
                if result.is_ok() {
                    "succeeded"
                } else if cancelled.load(Ordering::SeqCst) {
                    "interrupted"
                } else {
                    "failed"
                },
                started.elapsed().as_millis() as u64,
                &output,
            );
            let _ = db.event(
                &task_id,
                &chrono::Utc::now().to_rfc3339(),
                "workflow",
                &json!({"method":"blender.session.start","result":output}).to_string(),
            );
        }
        let (child, observer_port) = result?;
        let client = crate::asset_preview::Client::new(
            observer_port,
            token,
            task_id.clone(),
            project.clone().context("Missing project")?,
            session_id.clone(),
            checkpoints,
        )?;
        {
            let db = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            let mut state = crate::asset_task::get(&db, &task_id)?;
            state.session_id = Some(session_id);
            crate::asset_task::save(&db, &state)?;
        }
        Ok(Session {
            child: Some(child),
            pub_port: port,
            client,
            store,
            files,
            task: task_id,
            project,
        })
    }
}

fn ping(port: u16) -> Result<()> {
    let mut stream =
        TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(1))?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    stream.write_all(b"{\"type\":\"ping\",\"params\":{}}")?;
    let mut bytes = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 || bytes.len() + count > 4096 {
            bail!("Invalid Blender readiness reply");
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Ok(reply) = serde_json::from_slice::<Value>(&bytes) {
            if reply["status"] == "success" && reply["result"]["pong"] == true {
                return Ok(());
            }
            bail!("Blender readiness check failed");
        }
    }
}

pub struct Session {
    child: Option<OwnedChild>,
    store: Arc<Mutex<Store>>,
    pub(crate) files: Arc<Files>,
    task: String,
    project: Option<String>,
    pub pub_port: u16,
    pub client: crate::asset_preview::Client,
}

impl Session {
    pub(crate) fn store(&self) -> Arc<Mutex<Store>> {
        self.store.clone()
    }

    pub fn alive(&mut self) -> bool {
        self.child.as_mut().is_some_and(|child| {
            child
                .0
                .try_wait()
                .ok()
                .is_some_and(|status| status.is_none())
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let start = Instant::now();
        let id = self.store.lock().ok().and_then(|db| {
            call_log::begin(
                &db,
                "workflow",
                "blender.session.stop",
                Some(&self.task),
                self.project.as_deref(),
                &Value::Null,
            )
            .ok()
        });
        let stopped = self.child.take().is_none_or(|mut child| {
            crate::process::terminate(&mut child.0);
            child.0.try_wait().ok().flatten().is_some()
        });
        if let (Some(id), Ok(db)) = (id, self.store.lock()) {
            let _ = call_log::finish(
                &db,
                &id,
                if stopped { "succeeded" } else { "failed" },
                start.elapsed().as_millis() as u64,
                &if stopped {
                    Value::Null
                } else {
                    json!({"error":"Could not confirm owned Blender process exited"})
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservations_are_distinct_and_released_without_starting_blender() -> Result<()> {
        let root = tempfile::tempdir()?;
        let exe = std::env::current_exe()?;
        let first = Request::prepare(root.path(), root.path(), &exe)?;
        let second = Request::prepare(root.path(), root.path(), &exe)?;
        assert_ne!(first.port, second.port);
        let port = first.port;
        assert!(TcpListener::bind(("127.0.0.1", port)).is_err());
        drop(first);
        let _released = TcpListener::bind(("127.0.0.1", port))?;
        Ok(())
    }
}
