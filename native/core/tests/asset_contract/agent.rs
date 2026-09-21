use super::fixture::{stage, Fixture};
use anyhow::Result;
use base64::Engine;
use beaver_core::{asset_agent, asset_preview::Client, asset_task};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

fn context(fixture: Fixture, port: u16) -> Result<(asset_agent::Context, tempfile::TempDir)> {
    let checkpoints = fixture.root.join("checkpoints");
    fs::create_dir_all(&checkpoints)?;
    let client = Client::new(
        port,
        "fixture-token".into(),
        fixture.id,
        "project".into(),
        "session".into(),
        checkpoints,
    )?;
    Ok((
        asset_agent::Context {
            store: Arc::new(Mutex::new(fixture.store)),
            files: Arc::new(fixture.files),
            client,
        },
        fixture._temp,
    ))
}

fn params(arguments: Value) -> Value {
    json!({"threadId":"thread","turnId":"turn","arguments":arguments})
}

#[tokio::test]
async fn poll_carries_actual_png_and_acknowledgement_waits_for_transport_delivery() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("image", "now")?;
    fixture.submit(input)?;
    let (context, _temp) = context(fixture, 0)?;
    let reply = context.call(&params(json!({"operation":"poll"}))).await?;
    assert_eq!(reply.delivered.as_deref(), Some("image"));
    let data = reply.value["contentItems"][1]["imageUrl"]
        .as_str()
        .unwrap()
        .strip_prefix("data:image/png;base64,")
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
    assert_eq!(image::load_from_memory(&bytes)?.width(), 64);
    let acknowledge = params(
        json!({"operation":"acknowledge","feedbackId":"image","frameId":"frame",
        "imageObservation":"White fixture image","impact":"No geometry in the fixture","affectedStages":[]}),
    );
    assert!(context.call(&acknowledge).await.is_err());
    context.delivered("image")?;
    context.call(&acknowledge).await?;
    let db = context.store.lock().unwrap();
    let state = asset_task::get(&db, &context.client.task_id)?;
    assert_eq!(state.feedback[0].status, "acknowledged");
    assert_eq!(state.feedback[0].history[1]["transport"], "inputImage");
    Ok(())
}

#[tokio::test]
async fn stages_validate_before_io_and_commit_only_after_a_real_checkpoint() -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let (context, _temp) = context(Fixture::new()?, listener.local_addr()?.port())?;
    let before = params(json!({"operation":"stages","revision":0,"stages":[]}));
    assert!(context.call(&before).await.is_err());
    let server = checkpoint_server(listener, context.clone(), false);
    context
        .call(&params(
            json!({"operation":"stages","revision":0,"stages":[stage("body","running",&[])]}),
        ))
        .await?;
    server.join().unwrap()?;
    let db = context.store.lock().unwrap();
    let state = asset_task::get(&db, &context.client.task_id)?;
    assert_eq!(state.revision, 1);
    assert_eq!(state.stages[0].id, "body");
    assert!(state
        .checkpoint
        .as_deref()
        .is_some_and(|p| std::path::Path::new(p).is_file()));
    Ok(())
}

#[tokio::test]
async fn a_turn_change_during_checkpoint_cannot_commit_stale_stages() -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let (context, _temp) = context(Fixture::new()?, listener.local_addr()?.port())?;
    let server = checkpoint_server(listener, context.clone(), true);
    let result = context
        .call(&params(
            json!({"operation":"stages","revision":0,"stages":[stage("body","running",&[])]}),
        ))
        .await;
    server.join().unwrap()?;
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Stale asset tool invocation"));
    let db = context.store.lock().unwrap();
    let state = asset_task::get(&db, &context.client.task_id)?;
    assert_eq!(state.revision, 0);
    assert!(state.stages.is_empty());
    Ok(())
}

#[tokio::test]
async fn missing_identity_and_other_tasks_cannot_read_the_asset_tool() -> Result<()> {
    let (context, _temp) = context(Fixture::new()?, 0)?;
    for invalid in [
        json!({"arguments":{"operation":"state"}}),
        json!({"threadId":"other","turnId":"turn","arguments":{"operation":"state"}}),
        json!({"threadId":"thread","turnId":"old","arguments":{"operation":"state"}}),
    ] {
        assert!(context.call(&invalid).await.is_err());
    }
    let mut old_session = context.clone();
    old_session.client.session_id = "old-session".into();
    assert!(old_session
        .call(&params(json!({"operation":"state"})))
        .await
        .is_err());
    Ok(())
}

pub(super) fn checkpoint_server(
    listener: TcpListener,
    context: asset_agent::Context,
    change_turn: bool,
) -> thread::JoinHandle<Result<()>> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte)?;
            headers.push(byte[0]);
            anyhow::ensure!(headers.len() < 16384);
        }
        let headers = String::from_utf8(headers)?.to_ascii_lowercase();
        anyhow::ensure!(headers.starts_with("post /checkpoint "));
        anyhow::ensure!(headers.contains("authorization: bearer fixture-token"));
        anyhow::ensure!(headers.contains(&format!("x-beaver-task: {}", context.client.task_id)));
        anyhow::ensure!(headers.contains("x-beaver-project: project"));
        anyhow::ensure!(headers.contains("x-beaver-session: session"));
        let size: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .unwrap()
            .parse()?;
        let mut body = vec![0; size];
        stream.read_exact(&mut body)?;
        let path = context.client.checkpoints.join("checkpoint.blend");
        fs::write(&path, "BLENDER fixture scene")?;
        if change_turn {
            let db = context.store.lock().unwrap();
            let mut task: Value = db.get("task", &context.client.task_id)?.unwrap();
            task["turnId"] = json!("new-turn");
            db.put("task", &context.client.task_id, &task)?;
        }
        let response = json!({"path":path}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response)?;
        Ok(())
    })
}
