use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(test)]
#[path = "business_mcp_tests.rs"]
mod tests;

struct Bridge {
    client: reqwest::Client,
    base: String,
    token: String,
    initialized: AtomicBool,
}

pub fn run() -> Result<()> {
    let token = crate::business_api::token()?.context("BEAVER_API_TOKEN is required")?;
    let bridge = Arc::new(Bridge {
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(1200))
            .build()?,
        base: format!("http://127.0.0.1:{}", crate::business_api::port()?),
        token,
        initialized: AtomicBool::new(false),
    });
    tauri::async_runtime::block_on(serve(bridge, tokio::io::stdin(), tokio::io::stdout()))
}

impl Bridge {
    async fn capabilities(&self) -> Result<Value> {
        Ok(self
            .client
            .get(format!("{}/v1/capabilities", self.base))
            .bearer_auth(&self.token)
            .send()
            .await
            .context("Cannot connect to Beaver API")?
            .error_for_status()
            .context("Beaver API authentication or availability failure")?
            .json()
            .await?)
    }

    async fn respond(&self, message: Value) -> Option<Value> {
        let id = message.get("id")?.clone();
        let error = |code, text: &str| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":text}});
        if message["jsonrpc"] != "2.0" || !(id.is_string() || id.is_i64() || id.is_u64()) {
            return Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Invalid request"}}),
            );
        }
        let method = message["method"].as_str().unwrap_or("");
        let result = if method == "initialize" {
            let requested = message["params"]["protocolVersion"].as_str().unwrap_or("");
            let protocol = match requested {
                "2024-11-05" | "2025-03-26" | "2025-06-18" | "2025-11-25" => requested,
                _ => "2025-11-25",
            };
            if self.capabilities().await.is_err() {
                return Some(error(
                    -32603,
                    "Beaver API unavailable; start the desktop with matching API credentials",
                ));
            }
            self.initialized.store(true, Ordering::SeqCst);
            json!({"protocolVersion":protocol,"capabilities":{"tools":{}},"serverInfo":{"name":"beaver-business","version":env!("CARGO_PKG_VERSION")},"instructions":"Business operations affect the same live projects as the desktop. Poll state and task.events for task progress. Do not automatically retry mutations after timeouts."})
        } else if method == "ping" {
            json!({})
        } else if !self.initialized.load(Ordering::SeqCst) {
            return Some(error(-32600, "Initialize first"));
        } else if method == "tools/list" {
            match self.capabilities().await {
                Ok(value) => json!({"tools":value["tools"]}),
                Err(_) => return Some(error(-32603, "Cannot read Beaver capabilities")),
            }
        } else if method == "tools/call" {
            let result = self.client.post(format!("{}/v1/call", self.base)).bearer_auth(&self.token)
                .header("x-beaver-client","mcp")
                .json(&json!({"method":message["params"]["name"],"input":message["params"].get("arguments").cloned().unwrap_or_else(||json!({}))}))
                .send().await;
            let value = match result {
                Ok(response) => response.json::<Value>().await.unwrap_or_else(|_| json!({"ok":false,"error":{"code":"INVALID_RESPONSE","message":"Invalid API response; operation outcome unknown"}})),
                Err(_) => json!({"ok":false,"error":{"code":"TRANSPORT_ERROR","message":"API connection failed or timed out; operation outcome unknown. Inspect state before retrying."}}),
            };
            json!({"isError":value["ok"] != true,"structuredContent":value,"content":[{"type":"text","text":serde_json::to_string(&value).unwrap()}]})
        } else {
            return Some(error(-32601, "Unsupported MCP method"));
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}

async fn serve(
    bridge: Arc<Bridge>,
    mut reader: impl tokio::io::AsyncRead + Unpin,
    mut writer: impl tokio::io::AsyncWrite + Unpin,
) -> Result<()> {
    let mut pending = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut tasks = tokio::task::JoinSet::new();
    loop {
        while tasks.len() < 16 {
            let Some(end) = pending.iter().position(|b| *b == b'\n') else {
                break;
            };
            let line: Vec<u8> = pending.drain(..=end).collect();
            let bridge = bridge.clone();
            tasks.spawn(async move {
                match serde_json::from_slice(&line) {
                    Ok(message) => bridge.respond(message).await,
                    Err(_) => Some(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Invalid JSON"}})),
                }
            });
        }
        tokio::select! {
            read = reader.read(&mut chunk), if pending.len() <= 2 * 1024 * 1024 => {
                let size = read?;
                if size == 0 { tasks.abort_all(); return Ok(()); }
                pending.extend_from_slice(&chunk[..size]);
                anyhow::ensure!(pending.len() <= 2 * 1024 * 1024, "MCP input exceeds 2 MiB");
            }
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Some(response)) = result.transpose()? {
                    let mut bytes = serde_json::to_vec(&response)?;
                    bytes.push(b'\n');
                    writer.write_all(&bytes).await?;
                    writer.flush().await?;
                }
            }
        }
    }
}
