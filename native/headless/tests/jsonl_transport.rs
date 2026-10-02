use anyhow::{Context, Result};
use beaver_headless::{protocol, Host};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_and_oversized_lines_do_not_desynchronize_or_execute() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::open(temp.path())?;
    let (_signal, stop) = tokio::sync::watch::channel(false);
    let mut oversized = vec![b'x'; protocol::MAX_REQUEST_BYTES + 20];
    oversized.push(b'\n');
    let mut inputs = vec![oversized, b"{broken}\n".to_vec()];
    for request in [
        json!({"id":1,"method":"state","input":{"unknown":true}}),
        json!({"id":2,"method":"objectTask.run"}),
        json!({"id":3,"method":"state"}),
        json!({"id":4,"method":"shutdown","input":{"typo":true}}),
        json!({"id":5,"method":"shutdown"}),
    ] {
        inputs.push(format!("{request}\n").into_bytes());
    }
    // Keep stdin open until the explicit shutdown response: EOF is cancellation,
    // not a promise to execute additional buffered, unadmitted commands.
    let (client, server) = tokio::io::duplex(2 * protocol::MAX_REQUEST_BYTES);
    let (reader, writer) = tokio::io::split(server);
    let serving = tokio::spawn(protocol::serve(host.clone(), reader, writer, stop));
    let (mut input, output) = tokio::io::split(client);
    let mut output = BufReader::new(output).lines();
    let mut responses = Vec::new();
    for frame in inputs {
        input.write_all(&frame).await?;
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), output.next_line())
            .await??
            .context("Missing sequential response")?;
        responses.push(serde_json::from_str::<Value>(&line)?);
    }
    serving.await??;
    assert!(output.next_line().await?.is_none());
    assert_eq!(responses.len(), 7);
    assert!(responses[..4]
        .iter()
        .all(|response| response["error"].is_object()));
    assert_eq!(responses[4]["id"], 3);
    assert_eq!(responses[4]["result"]["projects"], json!([]));
    assert!(responses[5]["error"].is_object());
    assert_eq!(responses[6]["result"]["stopped"], true);
    assert!(host.call("state", json!({})).await.is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eof_after_accepted_mutation_preserves_project_and_closes_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Host::open(&data)?;
    let (_signal, stop) = tokio::sync::watch::channel(false);
    let request = json!({"id":"create","method":"project.create","input":{
        "parent":temp.path(), "name":"Preserved", "template":"blank"
    }})
    .to_string();
    let mut output = Vec::new();
    protocol::serve(host.clone(), request.as_bytes(), &mut output, stop).await?;
    let response: Value = serde_json::from_slice(&output)?;
    assert_eq!(response["id"], "create");
    assert!(temp.path().join("Preserved/project.godot").is_file());
    assert!(host.call("state", json!({})).await.is_err());
    drop(host);
    let reopened = Host::open(&data)?;
    assert_eq!(
        reopened.call("state", json!({})).await?["projects"][0]["id"],
        response["result"]["id"]
    );
    reopened.shutdown().await?;
    Ok(())
}
