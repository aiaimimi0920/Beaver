use super::{attempt_fixture::*, object_attempt_rpc_fixture as rpc, queue_fixture};
use crate::{
    executor::Control, object_attempt::State, object_attempt_launch::Factory,
    object_attempt_worker, object_run_preparation,
};
use anyhow::Result;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

fn factory(root: PathBuf, mode: &str, timeout: Duration) -> Factory {
    let mode = mode.to_owned();
    Arc::new(move |attempt, runtime| {
        let cwd = runtime
            .files()
            .resolve_workspace(
                &attempt.preparation.run.id,
                std::path::Path::new(&attempt.preparation.workspace),
            )
            .map_err(|error| error.to_string())?;
        let mut launch = rpc::launch(&root, &cwd, &mode).map_err(|error| error.to_string())?;
        launch.timeout = timeout;
        Ok(launch)
    })
}

#[tokio::test]
async fn early_completion_waits_for_entire_writer_tree_and_freezes_prompt() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head", "next"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let token = claim.record().claim_token.clone();
    let root = fixture.temp.path().join("rpc");
    let (_controls, receiver) = mpsc::channel(16);
    object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        factory(root.clone(), "early", Duration::from_secs(10)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {}),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root)?;
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::AwaitingGate);
    let trace = crate::object_attempt_trace::read(
        &fixture.runtime,
        &crate::object_attempt_trace::Request {
            project_id: "project-1".into(),
            run_id: attempt.preparation.run.id.clone(),
            attempt_id: attempt.id.clone(),
        },
    )?;
    assert_eq!(trace.entries.last().unwrap().operation, "turn");
    assert_eq!(trace.entries.len(), 3);
    assert_eq!(trace.entries[0].operation, "commandExecution");
    assert_eq!(trace.entries[0].phase, "started");
    assert_eq!(trace.entries[1].phase, "completed");
    assert!(!serde_json::to_string(&trace)?.contains("test-provider-secret"));
    assert_eq!(trace.entries.last().unwrap().status, "completed");
    let output = attempt.output.as_ref().unwrap();
    assert!(output.contains_key("result.txt") && output.contains_key("child-output.txt"));
    assert_eq!(task_record(&fixture, "fine-b")?.status, "planned");
    assert_eq!(task_record(&fixture, "next")?.status, "planned");
    let messages = rpc::transcript(&root)?;
    assert_eq!(
        messages
            .iter()
            .filter(|message| message["method"] == "turn/start")
            .count(),
        1
    );
    let prompt = messages
        .iter()
        .find(|message| message["method"] == "turn/start")
        .unwrap()["params"]["input"][0]["text"]
        .as_str()
        .unwrap();
    let context: Value = serde_json::from_str(prompt)?;
    assert_eq!(context["fine"], serde_json::to_value(&attempt.fine)?);
    assert_eq!(context["input"], serde_json::to_value(&attempt.input)?);
    assert!(!prompt.contains(&token));
    assert_eq!(fixture.count("task")?, 0);
    assert!(!fixture.temp.path().join("result.txt").exists());
    Ok(())
}

#[tokio::test]
async fn object_attempt_callback_roundtrip_freezes_result_and_waits_for_gate() -> Result<()> {
    let fixture = fixture()?;
    let version =
        super::run_fixture::capture(&fixture, "hero", "callback", "frozen input", vec![])?;
    super::run_fixture::accept(&fixture, "hero", &version)?;
    mutate(
        &fixture,
        "object_task",
        "head",
        "/identity/baseline",
        serde_json::json!({"basePolicy":"latestAccepted"}),
    )?;
    queue_fixture::enqueue(&fixture, &["head", "next"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let root = fixture.temp.path().join("rpc");
    let (_controls, receiver) = mpsc::channel(16);
    object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        factory(root.clone(), "callback", Duration::from_secs(10)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {}),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root)?;
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::AwaitingGate, "{:?}", attempt.error);
    let output = attempt.output.as_ref().unwrap();
    assert_eq!(
        std::fs::read_to_string(
            fixture
                .runtime
                .files()
                .blob(&output["callback-result.txt"])?
        )?,
        "frozen input"
    );
    assert_eq!(task_record(&fixture, "fine-b")?.status, "planned");
    assert_eq!(task_record(&fixture, "next")?.status, "planned");
    let transcript = rpc::transcript(&root)?;
    assert_eq!(
        transcript
            .iter()
            .filter(|message| message["method"] == "turn/start")
            .count(),
        1
    );
    let thread = transcript
        .iter()
        .find(|message| message["method"] == "thread/start")
        .unwrap();
    assert_eq!(
        thread["params"]["dynamicTools"][0]["name"],
        "beaver_object_attempt"
    );
    for id in [100, 101, 102] {
        assert!(transcript
            .iter()
            .any(|message| message["id"] == id && message["result"]["success"] == true));
    }
    Ok(())
}

#[tokio::test]
async fn failure_timeout_interaction_and_root_exit_keep_partial_outputs_without_retry() -> Result<()>
{
    for (mode, expected) in [
        ("error", "[REDACTED_SECRET]"),
        ("hang", "TIMEOUT"),
        ("interaction", "INTERACTION_UNAVAILABLE"),
        ("exit-parent", "PROCESS_EXITED"),
    ] {
        let fixture = fixture()?;
        queue_fixture::enqueue(&fixture, &["head", "next"])?;
        let claim =
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
        let root = fixture.temp.path().join("rpc");
        let (_controls, receiver) = mpsc::channel(16);
        object_attempt_worker::execute(
            fixture.runtime.clone(),
            claim,
            factory(root.clone(), mode, Duration::from_secs(2)),
            Arc::new(AtomicBool::new(false)),
            receiver,
            Arc::new(|| {}),
        )
        .await
        .map_err(anyhow::Error::msg)?;
        rpc::assert_closed(&root)?;
        let attempt = attempts(&fixture, "head")?.remove(0);
        assert_eq!(attempt.state, State::Failed, "{mode}");
        let error = attempt.error.as_deref().unwrap();
        assert!(error.contains(expected), "{mode}: {error}");
        assert!(!error.contains("test-provider-secret"));
        assert!(attempt
            .output
            .as_ref()
            .unwrap()
            .contains_key("child-output.txt"));
        assert!(workspace(&fixture, &attempt)?.exists());
        assert!(
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "restart")?.is_none()
        );
        assert_eq!(
            rpc::transcript(&root)?
                .iter()
                .filter(|message| message["method"] == "turn/start")
                .count(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn wrong_rpc_identities_cannot_complete_and_steer_cannot_change_definition() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let root = fixture.temp.path().join("rpc");
    let (controls, receiver) = mpsc::channel(16);
    let worker = tokio::spawn(object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        factory(root.clone(), "identities", Duration::from_secs(10)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {}),
    ));
    rpc::wait_until(|| root.join("turn-ready").exists()).await;
    assert_eq!(attempts(&fixture, "head")?[0].state, State::Running);
    let (reply, response) = oneshot::channel();
    controls
        .send(Control::Steer {
            text: "replace definition".into(),
            reply,
        })
        .await?;
    assert!(response.await?.unwrap_err().contains("DEFINITION_FROZEN"));
    controls.send(Control::Interrupt).await?;
    worker.await?.map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root)?;
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::Interrupted);
    assert_eq!(attempt.fine.prompt, "Work on fine-a");
    Ok(())
}

#[tokio::test]
async fn unavailable_workspace_sandbox_fails_before_model_turn_without_advancing_queue(
) -> Result<()> {
    use fs2::FileExt;
    for mode in ["read-only", "full-access", "missing-sandbox"] {
        let fixture = fixture()?;
        queue_fixture::enqueue(&fixture, &["head", "next"])?;
        let claim =
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
        let root = fixture.temp.path().join("rpc");
        let (_controls, receiver) = mpsc::channel(16);
        object_attempt_worker::execute(
            fixture.runtime.clone(),
            claim,
            factory(root.clone(), mode, Duration::from_secs(10)),
            Arc::new(AtomicBool::new(false)),
            receiver,
            Arc::new(|| {}),
        )
        .await
        .map_err(anyhow::Error::msg)?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join("lease"))?
            .try_lock_exclusive()?;
        let attempt = attempts(&fixture, "head")?.remove(0);
        assert_eq!(attempt.state, State::Failed, "{mode}");
        assert_eq!(
            attempt.error.as_deref(),
            Some("OBJECT_ATTEMPT_WORKSPACE_SANDBOX_UNAVAILABLE")
        );
        assert!(!workspace(&fixture, &attempt)?.join("result.txt").exists());
        assert!(!root.join("child-ready").exists());
        assert!(!rpc::transcript(&root)?
            .iter()
            .any(|message| message["method"] == "turn/start"));
        assert_eq!(task_record(&fixture, "next")?.status, "planned");
        assert!(
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "restart")?.is_none()
        );
    }
    Ok(())
}

#[tokio::test]
async fn startup_failure_does_not_claim_writer_stopped_or_capture_output() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let launch_factory: Factory = Arc::new(|_, _| {
        Ok(crate::object_attempt_launch::Launch {
            command: tokio::process::Command::new("beaver-missing-attempt-executable"),
            cwd: PathBuf::new(),
            model: "test".into(),
            secrets: vec![],
            timeout: Duration::from_secs(1),
            godot: None,
        })
    });
    let (_controls, receiver) = mpsc::channel(16);
    assert!(object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        launch_factory,
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {})
    )
    .await
    .is_err());
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::Running);
    assert!(attempt.output.is_none());
    Ok(())
}
