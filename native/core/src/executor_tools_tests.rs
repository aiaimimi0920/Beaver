use super::*;
use crate::{asset_preview::Client, asset_task, rpc::Event};
use anyhow::Result;
use std::time::Duration;
use tokio::{process::Command, sync::broadcast, time::timeout};

#[test]
fn capture_rpc() -> Result<()> {
    if std::env::var_os("BEAVER_TEST_RPC_CAPTURE").is_none() {
        return Ok(());
    }
    for line in std::io::stdin().lines() {
        let message: Value = serde_json::from_str(&line?)?;
        println!("{}", json!({"method":"captured","params":message}));
    }
    Ok(())
}

async fn captured(events: &mut broadcast::Receiver<Event>) -> Result<Value> {
    Ok(timeout(Duration::from_secs(10), async {
        loop {
            if let Event::Notification { method, params } = events.recv().await? {
                if method == "captured" {
                    return Ok::<_, anyhow::Error>(params);
                }
            }
        }
    })
    .await??)
}

#[tokio::test]
async fn asset_validation_error_reaches_model_and_next_call_still_works() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let db = Store::open(temp.path())?;
    let task = json!({"id":"task","projectId":"project","status":"running",
        "threadId":"thread","turnId":"turn","assetTask":true});
    db.put("task", "task", &task)?;
    let mut state = asset_task::enable(&db, &task)?;
    state.session_id = Some("session".into());
    asset_task::save(&db, &state)?;
    let store = Arc::new(Mutex::new(db));
    let asset = asset_agent::Context {
        store: store.clone(),
        root: temp.path().into(),
        client: Client::new(
            0,
            "unused".into(),
            "task".into(),
            "project".into(),
            "session".into(),
            temp.path().join("checkpoints"),
        )?,
    };
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "executor_tools::tests::capture_rpc",
            "--nocapture",
        ])
        .env("BEAVER_TEST_RPC_CAPTURE", "1");
    let (rpc, mut events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
    for (id, arguments) in [
        (1, json!({"operation":"stages","stages":[]})),
        (2, json!({"operation":"state"})),
    ] {
        dispatch(
            &store,
            "task",
            Some(&asset),
            &rpc,
            json!(id),
            "item/tool/call",
            &json!({"tool":"beaver_asset_task","threadId":"thread",
                "turnId":"turn","arguments":arguments}),
        )
        .await
        .map_err(anyhow::Error::msg)?;
        let reply = captured(&mut events).await?;
        assert_eq!(reply["id"], id);
        assert!(
            reply.get("error").is_none(),
            "tool errors must reach model content"
        );
        assert_eq!(reply["result"]["success"], id == 2);
        let content = reply["result"]["contentItems"][0]["text"].as_str().unwrap();
        if id == 1 {
            assert!(content.contains("revision"));
        } else {
            let state: Value = serde_json::from_str(content)?;
            assert_eq!(state["revision"], 0);
            assert_eq!(state["stages"], json!([]));
        }
    }
    assert!(!temp.path().join("checkpoints").exists());
    rpc.close().await.map_err(anyhow::Error::msg)?;
    Ok(())
}
