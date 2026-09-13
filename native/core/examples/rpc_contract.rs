use anyhow::{ensure, Result};
use beaver_core::rpc::{Event, Rpc};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    time::Duration,
};
use tokio::{process::Command, time::timeout};

fn fixture() -> Result<()> {
    let mut output = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let msg: Value = serde_json::from_str(&line?)?;
        let id = msg["id"].clone();
        let response = match msg["method"].as_str() {
            Some("initialize") => json!({"id":id,"result":{"fixture":true}}),
            Some("initialized") => continue,
            Some("wait") => continue,
            Some("exit") => break,
            Some("oversize") => {
                output.write_all(&vec![b'x'; 8 * 1024 * 1024 + 1])?;
                output.flush()?;
                continue;
            }
            Some("fail") => json!({"id":id,"error":{"code":-1,"message":"fixture error"}}),
            Some("events") => {
                writeln!(output, "not-json")?;
                writeln!(
                    output,
                    "{}",
                    json!({"method":"turn/completed","params":{"turn":{"id":"t"}}})
                )?;
                writeln!(
                    output,
                    "{}",
                    json!({"id":"question-1","method":"item/tool/requestUserInput","params":{"questions":[]}})
                )?;
                json!({"id":id,"result":true})
            }
            Some("echo") => json!({"id":id,"result":msg["params"]}),
            None => {
                writeln!(
                    output,
                    "{}",
                    json!({"method":"fixture/replied","params":{"id":id,"result":msg["result"],"error":msg["error"]}})
                )?;
                output.flush()?;
                continue;
            }
            _ => json!({"id":id,"result":null}),
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--fixture") {
        return fixture();
    }
    let output = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("output directory required"))?,
    );
    std::fs::create_dir_all(&output)?;
    let mut command = Command::new(std::env::current_exe()?);
    command.arg("--fixture");
    let (rpc, mut events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
    let checks = async {
        ensure!(rpc.initialize().await.map_err(anyhow::Error::msg)?["fixture"] == true);
        let (a, b) = tokio::join!(
            rpc.request("echo", json!({"text":"中文"})),
            rpc.request("echo", json!({"number":2}))
        );
        ensure!(a.map_err(anyhow::Error::msg)?["text"] == "中文");
        ensure!(b.map_err(anyhow::Error::msg)?["number"] == 2);
        ensure!(rpc.request("fail", json!({})).await.unwrap_err() == "fixture error");
        ensure!(rpc
            .request_timeout("wait", json!({}), Duration::from_millis(30))
            .await
            .is_err());
        rpc.request("events", json!({}))
            .await
            .map_err(anyhow::Error::msg)?;
        let mut notification = false;
        let mut question = false;
        for _ in 0..3 {
            match timeout(Duration::from_secs(2), events.recv()).await?? {
                Event::Notification { method, .. } => notification = method == "turn/completed",
                Event::ServerRequest { id, method, .. } => {
                    ensure!(method == "item/tool/requestUserInput");
                    rpc.respond(id, json!({"answer":"fixture"}))
                        .await
                        .map_err(anyhow::Error::msg)?;
                    question = true;
                }
                Event::Log(_) => {}
                _ => anyhow::bail!("unexpected event"),
            }
        }
        ensure!(notification && question);
        match timeout(Duration::from_secs(2), events.recv()).await?? {
            Event::Notification { method, params } => {
                ensure!(
                    method == "fixture/replied"
                        && params["id"] == "question-1"
                        && params["result"]["answer"] == "fixture"
                );
            }
            _ => anyhow::bail!("interactive response missing"),
        }
        let (pending, stopped) = tokio::join!(rpc.request("wait", json!({})), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            rpc.close().await
        });
        ensure!(pending.is_err());
        stopped.map_err(anyhow::Error::msg)?;
        ensure!(rpc.request("echo", json!({})).await.is_err());
        Ok::<_, anyhow::Error>(())
    }
    .await;
    rpc.close().await.map_err(anyhow::Error::msg)?;
    checks?;
    for method in ["exit", "oversize"] {
        let mut command = Command::new(std::env::current_exe()?);
        command.arg("--fixture");
        let (rpc, _events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
        let response = rpc
            .request_timeout(method, json!({}), Duration::from_secs(5))
            .await;
        rpc.close().await.map_err(anyhow::Error::msg)?;
        ensure!(response.is_err(), "unexpected response to {method}");
        ensure!(
            !response.unwrap_err().contains("请求超时"),
            "transport did not fail promptly"
        );
    }
    let proof = json!({"passed":true,"checks":["native subprocess stdio initialize", "concurrent requests and Unicode", "RPC error and timeout", "malformed output isolated", "notification and interactive reply", "close rejects pending and future requests", "repeat close safe", "unexpected subprocess exit rejects pending", "oversized message fails promptly"], "codexExecutionComplete":false});
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!("{proof}");
    Ok(())
}
