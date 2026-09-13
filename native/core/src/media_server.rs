use crate::media::{self, Media};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    task::JoinSet,
};

pub const MODE_ARGUMENT: &str = "--media-mcp";

/// Headless entry point; deliberately independent of desktop state and WebView.
pub fn run_stdio() -> Result<()> {
    let media = std::env::var_os("BEAVER_PROJECT_ROOT")
        .ok_or_else(|| "Project root missing".to_owned())
        .and_then(|root| {
            let configuration =
                std::env::var("BEAVER_MEDIA_PROVIDERS").unwrap_or_else(|_| "{}".into());
            Media::new(root.into(), &configuration)
                .map_err(|_| "Invalid media provider configuration".to_owned())
        });
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(tokio::io::stdin(), tokio::io::stdout(), media))
}

async fn respond(media: Arc<Result<Media, String>>, message: Value) -> Option<Value> {
    let id = message.get("id")?;
    if !id.is_string() && !id.is_number() {
        return Some(
            json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Invalid request ID"}}),
        );
    }
    let result = match message["method"].as_str().unwrap_or("") {
        "initialize" => {
            json!({"protocolVersion":message["params"]["protocolVersion"].as_str().unwrap_or("2024-11-05"),"capabilities":{"tools":{}},"serverInfo":{"name":"beaver-media","version":"0.1.0"}})
        }
        "ping" => json!({}),
        "tools/list" => {
            let mut tools = media::tools().as_array().cloned().unwrap_or_default();
            tools.extend(crate::workflows::tools());
            tools.push(crate::code_structure::tool::definition());
            json!({"tools":tools})
        }
        "tools/call" => {
            let call = match media.as_ref() {
                Ok(media) => media
                    .call(
                        message["params"]["name"].as_str().unwrap_or(""),
                        message["params"]["arguments"].clone(),
                    )
                    .await
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.clone()),
            };
            match call {
                Ok(text) => {
                    let failed = serde_json::from_str::<Value>(&text)
                        .ok()
                        .is_some_and(|v| v["ok"] == false);
                    json!({"isError":failed,"content":[{"type":"text","text":text}]})
                }
                Err(text) => json!({"isError":true,"content":[{"type":"text","text":text}]}),
            }
        }
        _ => {
            return Some(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported MCP method"}}),
            )
        }
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

/// Bounded stdio transport. EOF cancels outstanding requests; stdout is JSON only.
pub async fn serve<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    mut reader: R,
    mut writer: W,
    media: Result<Media, String>,
) -> Result<()> {
    let media = Arc::new(media);
    let mut tasks = JoinSet::new();
    let mut pending = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        while tasks.len() < 4 {
            let Some(end) = pending.iter().position(|b| *b == b'\n') else {
                break;
            };
            let line: Vec<u8> = pending.drain(..=end).collect();
            let media = media.clone();
            tasks.spawn(async move {
                match serde_json::from_slice(&line) {
                    Ok(message) => respond(media,message).await,
                    Err(_) => Some(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Invalid JSON"}})),
                }
            });
        }
        tokio::select! {
            read = reader.read(&mut chunk) => {
                let size = read?;
                if size == 0 { tasks.abort_all(); while tasks.join_next().await.is_some() {} return Ok(()); }
                pending.extend_from_slice(&chunk[..size]);
                if pending.len() > 256 * 1024 { anyhow::bail!("MCP request exceeds size limit"); }
            },
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(result) = result {
                    if let Some(response) = result? {
                        let mut bytes = serde_json::to_vec(&response)?;
                        bytes.push(b'\n');
                        writer.write_all(&bytes).await?;
                        writer.flush().await?;
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};

    #[tokio::test]
    async fn malformed_input_and_invalid_configuration_do_not_break_protocol() -> Result<()> {
        let (client, server) = tokio::io::duplex(4096);
        let (reader, writer) = tokio::io::split(server);
        let job = tokio::spawn(serve(
            reader,
            writer,
            Err("Invalid media provider configuration".into()),
        ));
        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);
        writer.write_all(b"not json\n").await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["error"]["code"], -32700);
        writer.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{}}\n").await?;
        line.clear();
        reader.read_line(&mut line).await?;
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["id"], "request-1");
        assert_eq!(response["result"]["isError"], true);
        writer.shutdown().await?;
        tokio::time::timeout(std::time::Duration::from_secs(2), job).await???;
        Ok(())
    }

    #[tokio::test]
    async fn oversized_unterminated_message_is_bounded() -> Result<()> {
        let input = vec![b'x'; 256 * 1024 + 1];
        let result = serve(
            input.as_slice(),
            tokio::io::sink(),
            Err("not configured".into()),
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("size limit"));
        Ok(())
    }
}
