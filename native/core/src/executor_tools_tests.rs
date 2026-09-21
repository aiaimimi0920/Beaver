use super::*;
use crate::{
    asset_delivery_files as artifacts, asset_preview::Client, asset_task,
    project_storage::ProjectStore, rpc::Event,
};
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
    let files = Arc::new(Files::new(temp.path().into()));
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
        files: files.clone(),
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
            &files,
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

#[tokio::test]
async fn task_callback_rpc_returns_durable_result_and_recovers_from_conflicts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let files = Arc::new(Files::new(temp.path().into()));
    let db = Store::open(temp.path())?;
    db.put(
        "task",
        "task",
        &json!({"id":"task","projectId":"project","status":"running",
        "threadId":"thread","turnId":"turn"}),
    )?;
    let store = Arc::new(Mutex::new(db));
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "executor_tools::tests::capture_rpc",
            "--nocapture",
        ])
        .env("BEAVER_TEST_RPC_CAPTURE", "1");
    let (rpc, mut events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
    let request = json!({"operation":"report","requestId":"draft","expectedRevision":0,
        "report":{"kind":"result","summary":"Draft saved","inputs":{},"outputs":{"file":"draft.md"}}});
    let mut conflict = request.clone();
    conflict["report"]["summary"] = json!("Other draft");
    for (id, arguments, success) in [
        (1, request.clone(), true),
        (2, conflict, false),
        (3, request, true),
        (4, json!({"operation":"state"}), true),
    ] {
        assert!(dispatch(
            &store,
            &files,
            "task",
            None,
            &rpc,
            json!(id),
            "item/tool/call",
            &json!({"tool":"beaver_task","threadId":"thread","turnId":"turn","arguments":arguments})
        )
        .await
        .map_err(anyhow::Error::msg)?
        .is_none());
        let reply = captured(&mut events).await?;
        assert_eq!(reply["id"], id);
        assert!(reply.get("error").is_none());
        assert_eq!(reply["result"]["success"], success);
        if id == 4 {
            let state: Value =
                serde_json::from_str(reply["result"]["contentItems"][0]["text"].as_str().unwrap())?;
            assert_eq!(state["revision"], 1);
            assert_eq!(state["receipts"].as_array().unwrap().len(), 1);
        }
    }
    rpc.close().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

#[tokio::test]
async fn delivery_submission_returns_frozen_receipt_and_stops_dynamic_dispatch() -> Result<()> {
    for with_asset in [false, true] {
        project_delivery_dispatch(with_asset).await?;
    }
    Ok(())
}

async fn project_delivery_dispatch(with_asset: bool) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("project");
    std::fs::create_dir(&project)?;
    std::fs::write(project.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(&project, "project")?.into_runtime();
    let files = runtime.files();
    let store = runtime.store();
    let workspace = files.workspace("task")?;
    std::fs::create_dir(&workspace)?;
    std::fs::write(workspace.join("design.md"), "frozen design")?;
    let task = json!({"id":"task","projectId":"project","status":"running",
        "threadId":"thread","turnId":"turn","assetTask":true,
        "workspace":files.workspace_location("task")?});
    let db = store.lock().unwrap();
    db.put("task", "task", &task)?;
    asset_task::enable(&db, &task)?;
    drop(db);
    let asset = asset_agent::Context {
        store: store.clone(),
        files: files.clone(),
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
        (
            1,
            json!({"operation":"deliveryPlan","requestId":"plan","expectedRevision":0,"assetRevision":0,
            "template":{"id":"two-stage","version":1,"stages":[{"id":"design","name":"Design"},{"id":"model","name":"Model"}]}}),
        ),
        (
            2,
            json!({"operation":"submitDelivery","requestId":"candidate","expectedRevision":1,"assetRevision":1,
            "stageId":"design","inputCandidates":[],"paths":["design.md"],"summary":"Saved design"}),
        ),
    ] {
        let outcome = dispatch(&store, &files, "task", with_asset.then_some(&asset), &rpc, json!(id), "item/tool/call",
            &json!({"tool":"beaver_task","threadId":"thread","turnId":"turn","arguments":arguments}))
            .await.map_err(anyhow::Error::msg)?;
        assert_eq!(
            outcome,
            if id == 2 {
                Some(Outcome::AwaitingInput)
            } else {
                None
            }
        );
        let reply = captured(&mut events).await?;
        assert_eq!(reply["result"]["success"], true, "{reply}");
        if id == 2 {
            let receipt: Value =
                serde_json::from_str(reply["result"]["contentItems"][0]["text"].as_str().unwrap())?;
            assert_eq!(receipt["paused"], true);
            let task: Value = store.lock().unwrap().get("task", "task")?.unwrap();
            assert_eq!(task["waitingDelivery"], receipt["candidateId"]);
            let candidate = artifacts::get(
                &store.lock().unwrap(),
                "task",
                receipt["candidateId"].as_str().unwrap(),
            )?;
            let hash = &candidate.files["design.md"];
            assert_eq!(
                files.blob(hash)?,
                std::fs::canonicalize(project.join(".beaver/content/blobs"))?.join(hash)
            );
            assert_eq!(
                artifacts::read(&files, &candidate, "design.md")?,
                b"frozen design"
            );
        }
    }
    rpc.close().await.map_err(anyhow::Error::msg)?;
    assert!(!project.join(".beaver/blobs").exists());
    assert!(!project.join(".beaver/control/blobs").exists());
    assert!(!temp.path().join("blobs").exists());
    Ok(())
}

#[tokio::test]
async fn workflow_dispatch_without_blender_enforces_model_authority_and_identity() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let files = Arc::new(Files::new(temp.path().into()));
    let db = Store::open(temp.path())?;
    db.put(
        "task",
        "task",
        &json!({"id":"task","status":"running","threadId":"thread","turnId":"turn"}),
    )?;
    let store = Arc::new(Mutex::new(db));
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "executor_tools::tests::capture_rpc",
            "--nocapture",
        ])
        .env("BEAVER_TEST_RPC_CAPTURE", "1");
    let (rpc, mut events) = Rpc::spawn(command).map_err(anyhow::Error::msg)?;
    for (id, turn, arguments, success) in [
        (
            1,
            "turn",
            json!({"operation":"configure","configuration":{"revision":0,"plugins":[],"rules":[],"semanticRequired":false}}),
            false,
        ),
        (2, "old-turn", json!({"operation":"state"}), false),
        (3, "turn", json!({"operation":"state"}), true),
    ] {
        assert!(dispatch(&store, &files, "task", None, &rpc, json!(id), "item/tool/call",
            &json!({"tool":"beaver_workflow","threadId":"thread","turnId":turn,"arguments":arguments}))
            .await.map_err(anyhow::Error::msg)?.is_none());
        let reply = captured(&mut events).await?;
        assert!(reply.get("error").is_none());
        assert_eq!(reply["result"]["success"], success, "{reply}");
        let content = reply["result"]["contentItems"][0]["text"].as_str().unwrap();
        if id == 1 {
            assert!(content.contains("Only the owner"), "{content}");
        } else if id == 3 {
            let state: Value = serde_json::from_str(content)?;
            assert_eq!(state["configuration"]["revision"], 0);
            assert_eq!(state["operations"], json!([]));
        }
    }
    rpc.close().await.map_err(anyhow::Error::msg)?;
    Ok(())
}
