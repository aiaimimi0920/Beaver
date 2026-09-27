use serde_json::{json, Value};
use std::{
    collections::HashMap,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdin, Command},
    sync::{broadcast, oneshot, Mutex as AsyncMutex},
    time::timeout,
};

type Reply = Result<Value, String>;

#[path = "rpc_process_tree.rs"]
pub(crate) mod process_tree;
use process_tree::ProcessTree;

#[derive(Clone, Debug)]
pub enum Event {
    Notification {
        method: String,
        params: Value,
    },
    ServerRequest {
        id: Value,
        method: String,
        params: Value,
    },
    Log(String),
    Exit,
}

struct Inner {
    input: AsyncMutex<ChildStdin>,
    child: AsyncMutex<Child>,
    process_tree: Option<ProcessTree>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Reply>>>,
    sequence: AtomicU64,
    closed: AtomicBool,
    events: broadcast::Sender<Event>,
}

impl Inner {
    fn fail(&self, reason: &str) {
        self.closed.store(true, Ordering::SeqCst);
        for (_, reply) in self.pending.lock().unwrap().drain() {
            let _ = reply.send(Err(reason.into()));
        }
    }
    async fn send(&self, value: Value) -> Result<(), String> {
        let mut input = self.input.lock().await;
        if self.closed.load(Ordering::SeqCst) {
            return Err("Codex 连接已关闭".into());
        }
        let mut bytes = serde_json::to_vec(&value).map_err(|_| "RPC 编码失败")?;
        bytes.push(b'\n');
        struct WriteGuard<'a>(&'a Inner, bool);
        impl Drop for WriteGuard<'_> {
            fn drop(&mut self) {
                if !self.1 {
                    self.0.fail("Codex 消息写入中断，连接不可继续使用");
                }
            }
        }
        let mut guard = WriteGuard(self, false);
        input
            .write_all(&bytes)
            .await
            .map_err(|_| "Codex 输入管道已关闭".to_string())?;
        guard.1 = true;
        Ok(())
    }
    fn receive(&self, bytes: &[u8]) {
        let Ok(Value::Object(message)) = serde_json::from_slice::<Value>(bytes) else {
            let _ = self
                .events
                .send(Event::Log("忽略无法解析的 Codex 输出".into()));
            return;
        };
        if let Some(method) = message.get("method").and_then(Value::as_str) {
            let params = message
                .get("params")
                .filter(|v| v.is_object())
                .cloned()
                .unwrap_or(json!({}));
            let event = match message.get("id").filter(|v| v.is_string() || v.is_number()) {
                Some(id) => Event::ServerRequest {
                    id: id.clone(),
                    method: method.into(),
                    params,
                },
                None => Event::Notification {
                    method: method.into(),
                    params,
                },
            };
            let _ = self.events.send(event);
        } else if let Some(id) = message.get("id").and_then(Value::as_u64) {
            if let Some(reply) = self.pending.lock().unwrap().remove(&id) {
                let value = if let Some(error) = message.get("error").filter(|v| !v.is_null()) {
                    Err(error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Codex RPC error")
                        .into())
                } else {
                    Ok(message.get("result").cloned().unwrap_or(Value::Null))
                };
                let _ = reply.send(value);
            }
        }
    }
}

pub struct Rpc {
    inner: Arc<Inner>,
}

impl Rpc {
    /// `command` is the resolved executable with explicit environment and cwd.
    /// Callers must continuously consume events, including interactive requests.
    pub fn spawn(command: Command) -> Result<(Self, broadcast::Receiver<Event>), String> {
        Self::spawn_inner(command, None)
    }

    pub(crate) fn spawn_owned(
        command: Command,
    ) -> Result<(Self, broadcast::Receiver<Event>), String> {
        Self::spawn_inner(command, Some(ProcessTree::new()?))
    }

    fn spawn_inner(
        mut command: Command,
        process_tree: Option<ProcessTree>,
    ) -> Result<(Self, broadcast::Receiver<Event>), String> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        #[cfg(unix)]
        command.process_group(0);
        if let Some(tree) = &process_tree {
            tree.configure(&mut command);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("无法启动 Codex：{e}"))?;
        if let Some(tree) = &process_tree {
            tree.attach(&child)?;
        }
        let stdout = child.stdout.take().ok_or("缺少 Codex 输出管道")?;
        let stderr = child.stderr.take().ok_or("缺少 Codex 日志管道")?;
        let input = child.stdin.take().ok_or("缺少 Codex 输入管道")?;
        let (events, receiver) = broadcast::channel(512);
        let inner = Arc::new(Inner {
            input: AsyncMutex::new(input),
            child: AsyncMutex::new(child),
            process_tree,
            pending: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            events,
        });
        let weak = Arc::downgrade(&inner);
        tokio::spawn(async move {
            let reason = read_messages(stdout, &weak)
                .await
                .err()
                .unwrap_or("Codex 执行进程已退出".into());
            if let Some(inner) = weak.upgrade() {
                inner.fail(&reason);
                let _ = inner.events.send(Event::Exit);
            }
        });
        let weak = Arc::downgrade(&inner);
        tokio::spawn(async move {
            let mut stderr = stderr;
            let mut buffer = [0u8; 8000];
            while let Ok(length) = stderr.read(&mut buffer).await {
                if length == 0 {
                    break;
                }
                let Some(inner) = weak.upgrade() else {
                    break;
                };
                let _ = inner.events.send(Event::Log(
                    String::from_utf8_lossy(&buffer[..length]).into(),
                ));
            }
        });
        Ok((Self { inner }, receiver))
    }

    pub async fn initialize(&self) -> Reply {
        let result = self.request("initialize", json!({"clientInfo":{"name":"beaver_studio","title":"Beaver","version":"0.1.19"},"capabilities":{"experimentalApi":true}})).await?;
        self.inner
            .send(json!({"method":"initialized","params":{}}))
            .await?;
        Ok(result)
    }

    pub async fn request(&self, method: &str, params: Value) -> Reply {
        self.request_timeout(method, params, Duration::from_secs(45))
            .await
    }

    pub async fn request_timeout(&self, method: &str, params: Value, limit: Duration) -> Reply {
        if self.inner.closed.load(Ordering::SeqCst) {
            return Err("Codex 连接已关闭".into());
        }
        let id = self.inner.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = oneshot::channel();
        self.inner.pending.lock().unwrap().insert(id, sender);
        // Guard also removes pending entries if the caller cancels the future.
        struct Pending<'a>(&'a Inner, u64);
        impl Drop for Pending<'_> {
            fn drop(&mut self) {
                self.0.pending.lock().unwrap().remove(&self.1);
            }
        }
        let _pending = Pending(&self.inner, id);
        let work = async {
            self.inner
                .send(json!({"id":id,"method":method,"params":params}))
                .await?;
            receiver.await.map_err(|_| "Codex 连接已关闭".to_string())?
        };
        timeout(limit, work)
            .await
            .map_err(|_| format!("Codex {method} 请求超时"))?
    }

    pub async fn respond(&self, id: Value, result: Value) -> Result<(), String> {
        if !id.is_string() && !id.is_number() {
            return Err("RPC 请求标识无效".into());
        }
        self.inner.send(json!({"id":id,"result":result})).await
    }

    pub async fn reject(&self, id: Value, message: &str) -> Result<(), String> {
        if !id.is_string() && !id.is_number() {
            return Err("RPC 请求标识无效".into());
        }
        self.inner
            .send(json!({"id":id,"error":{"code":-32000,"message":message}}))
            .await
    }

    pub(crate) async fn wait_for_exit(&self) -> Result<(), String> {
        // Descendants may retain pipe handles after the root process exits.
        // The caller drops this cancellation-safe wait before closing the tree.
        self.inner
            .child
            .lock()
            .await
            .wait()
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub async fn close(&self) -> Result<(), String> {
        self.inner.fail("Codex 连接已关闭");
        let mut child = self.inner.child.lock().await;
        if let Some(tree) = &self.inner.process_tree {
            tree.close().await?;
            timeout(Duration::from_secs(5), child.wait())
                .await
                .map_err(|_| "OBJECT_ATTEMPT_ROOT_CLOSE_TIMEOUT")?
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Ok(());
        }
        if let Some(pid) = child.id() {
            #[cfg(windows)]
            {
                let executable = std::env::var_os("SystemRoot")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| "C:/Windows".into())
                    .join("System32/taskkill.exe");
                let mut killer = Command::new(executable);
                killer
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true);
                let _ = timeout(Duration::from_secs(5), killer.status()).await;
            }
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        let _ = child.start_kill();
        timeout(Duration::from_secs(5), child.wait())
            .await
            .map_err(|_| "无法停止本次 Codex 进程".to_string())?
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

async fn read_messages(
    mut stdout: impl AsyncRead + Unpin,
    weak: &std::sync::Weak<Inner>,
) -> Result<(), String> {
    let mut pending = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let length = stdout
            .read(&mut buffer)
            .await
            .map_err(|_| "Codex 输出管道读取失败")?;
        if length == 0 {
            if !pending.is_empty() {
                if let Some(inner) = weak.upgrade() {
                    inner.receive(&pending);
                }
            }
            return Ok(());
        }
        for part in buffer[..length].split_inclusive(|b| *b == b'\n') {
            if pending.len() + part.len() > 8 * 1024 * 1024 {
                return Err("Codex 单条消息超过 8 MiB".into());
            }
            pending.extend_from_slice(part);
            if part.last() == Some(&b'\n') {
                let Some(inner) = weak.upgrade() else {
                    return Ok(());
                };
                inner.receive(&pending);
                pending.clear();
            }
        }
    }
}
