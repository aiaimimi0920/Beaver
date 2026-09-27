use super::*;
use crate::{object_task_planning_rpc_fixture as fake, object_task_title_service::Service};
use anyhow::Result;
use std::path::Path;

fn request() -> Request {
    Request {
        project_id: "p".into(),
        prompt: "Add dash".into(),
        acceptance: "Buffered input".into(),
    }
}

fn launch(root: &Path, mode: &str) -> Result<Prepared> {
    let launch = fake::launch(root, mode, Value::Null)?;
    Ok(Prepared {
        command: launch.command,
        cwd: launch.cwd,
        model: launch.model,
        secrets: launch.secrets,
        scratch: launch.scratch.unwrap(),
    })
}

#[tokio::test]
async fn structured_title_is_isolated_and_rejects_wrong_identity_or_tools() -> Result<()> {
    for mode in ["title", "title-correction"] {
        let output = tempfile::tempdir()?;
        let launch = launch(output.path(), mode)?;
        let home = launch.cwd.clone();
        let (_sender, stop) = watch::channel(false);
        assert_eq!(
            run(launch, request(), stop, Duration::from_secs(10))
                .await
                .unwrap(),
            "Add buffered dash"
        );
        fake::assert_closed(output.path())?;
        assert!(!home.exists());
        let messages = fake::transcript(output.path())?;
        let start = messages
            .iter()
            .find(|m| m["method"] == "thread/start")
            .unwrap();
        assert_eq!(start["params"]["sandbox"], "read-only");
        assert_eq!(start["params"]["ephemeral"], true);
        assert_eq!(start["params"]["dynamicTools"].as_array().unwrap().len(), 1);
        let turn = messages
            .iter()
            .find(|m| m["method"] == "turn/start")
            .unwrap();
        let input: Value =
            serde_json::from_str(turn["params"]["input"][0]["text"].as_str().unwrap())?;
        assert_eq!(
            input,
            json!({"prompt":"Add dash","acceptance":"Buffered input"})
        );
        if mode == "title-correction" {
            for id in 100..104 {
                let reply = messages.iter().find(|m| m["id"] == id).unwrap();
                assert_eq!(reply["result"]["success"], false);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn errors_and_timeouts_close_process_and_redact_provider_secrets() -> Result<()> {
    for (mode, expected) in [
        ("empty", "NO_RESULT"),
        ("error", "REDACTED_SECRET"),
        ("hang", "TIMEOUT"),
        ("initialize-hang", "TIMEOUT"),
    ] {
        let output = tempfile::tempdir()?;
        let (_sender, stop) = watch::channel(false);
        let result = run(
            launch(output.path(), mode)?,
            request(),
            stop,
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
        assert!(result.contains(expected), "{result}");
        assert!(!result.contains("test-provider-secret"));
        fake::assert_closed(output.path())?;
    }
    Ok(())
}

#[tokio::test]
async fn dropped_callers_remain_bounded_and_shutdown_joins_children() -> Result<()> {
    let service = Service::default();
    let mut outputs = Vec::new();
    for _ in 0..2 {
        let output = tempfile::tempdir()?;
        let root = output.path().to_owned();
        let client = service.clone();
        let caller = tokio::spawn(async move {
            client
                .suggest(request(), move || launch(&root, "initialize-hang"))
                .await
        });
        fake::wait_until(|| output.path().join("lease").exists()).await;
        caller.abort();
        let _ = caller.await;
        outputs.push(output);
    }
    assert_eq!(
        service
            .suggest(request(), || panic!("busy must not prepare"))
            .await
            .unwrap_err(),
        "OBJECT_TASK_TITLE_BUSY"
    );
    service.shutdown().await.unwrap();
    for output in outputs {
        fake::assert_closed(output.path())?;
    }
    assert_eq!(
        service
            .suggest(request(), || panic!("shutdown must not prepare"))
            .await
            .unwrap_err(),
        "OBJECT_TASK_TITLE_STOPPED"
    );
    Ok(())
}

#[tokio::test]
async fn invalid_input_never_launches_a_process() {
    let service = Service::default();
    for bad in [
        Request {
            prompt: "".into(),
            acceptance: "".into(),
            ..request()
        },
        Request {
            prompt: "x".repeat(20_001),
            ..request()
        },
    ] {
        assert_eq!(
            service
                .suggest(bad, || panic!("invalid input must not prepare"))
                .await
                .unwrap_err(),
            "OBJECT_TASK_TITLE_INVALID_INPUT"
        );
    }
    service.shutdown().await.unwrap();
}
