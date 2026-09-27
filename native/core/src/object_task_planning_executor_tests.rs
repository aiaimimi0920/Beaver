use super::*;
use crate::{
    object_catalog_test_fixture::Fixture, object_task_planning as planning,
    object_task_planning_fixture as fixture, object_task_planning_rpc_fixture as fake,
    object_tasks::AssumptionSource,
};
use anyhow::Result;
use std::time::Duration;

#[tokio::test]
async fn manual_questions_and_proposals_release_the_owned_process_and_home() -> Result<()> {
    for (mode, expected) in [
        ("questions", planning::Status::AwaitingInput),
        ("proposal", planning::Status::Proposed),
    ] {
        let project = Fixture::new()?;
        let runtime = project.runtime.clone();
        let session = planning::start(&runtime, &fixture::input())?.session;
        let key = RoundKey::from(&session);
        let output = tempfile::tempdir()?;
        let launch = fake::launch(output.path(), mode, round::context(&runtime, &key)?)?;
        let home = launch.cwd.clone();
        let (_sender, receiver) = watch::channel(false);
        run(&runtime, &key, launch, receiver, &fake::notify())
            .await
            .map_err(anyhow::Error::msg)?;
        assert_eq!(fixture::session(&runtime)?.status, expected);
        fake::assert_closed(output.path())?;
        assert!(!home.exists());
        let messages = fake::transcript(output.path())?;
        let start = messages
            .iter()
            .find(|m| m["method"] == "thread/start")
            .unwrap();
        assert_eq!(start["params"]["sandbox"], "read-only");
        assert_eq!(start["params"]["ephemeral"], true);
        assert_eq!(start["params"]["dynamicTools"].as_array().unwrap().len(), 2);
    }
    Ok(())
}

#[tokio::test]
async fn automatic_decisions_and_correctable_errors_reach_a_structured_proposal() -> Result<()> {
    for mode in ["automatic", "correction"] {
        let project = Fixture::new()?;
        let runtime = project.runtime.clone();
        let mut input = fixture::input();
        input.ask_ratio = 0;
        let session = planning::start(&runtime, &input)?.session;
        let key = RoundKey::from(&session);
        let output = tempfile::tempdir()?;
        let launch = fake::launch(output.path(), mode, round::context(&runtime, &key)?)?;
        let (_sender, receiver) = watch::channel(false);
        run(&runtime, &key, launch, receiver, &fake::notify())
            .await
            .map_err(anyhow::Error::msg)?;
        let session = fixture::session(&runtime)?;
        assert_eq!(session.status, planning::Status::Proposed);
        assert_eq!(session.proposal.as_ref().unwrap().tasks.len(), 1);
        let replies = fake::transcript(output.path())?;
        if mode == "automatic" {
            assert_eq!(session.decisions[0].source, AssumptionSource::Automatic);
            assert_eq!(session.decisions[0].answer, "Stylized");
            let reply = replies.iter().find(|m| m["id"] == 100).unwrap();
            assert_eq!(reply["result"]["success"], true);
        } else {
            for id in 100..104 {
                let reply = replies.iter().find(|m| m["id"] == id).unwrap();
                assert_eq!(reply["result"]["success"], false, "call {id}");
            }
            assert_eq!(
                session.revision, 4,
                "denied callbacks cannot mutate the session"
            );
        }
        fake::assert_closed(output.path())?;
    }
    Ok(())
}

#[tokio::test]
async fn empty_completion_provider_error_and_timeouts_are_errors_and_close_the_process(
) -> Result<()> {
    for (mode, error) in [
        ("empty", "OBJECT_PLANNING_NO_PROPOSAL"),
        ("error", "[REDACTED_SECRET]"),
        ("hang", "OBJECT_PLANNING_TIMEOUT"),
        ("initialize-hang", "OBJECT_PLANNING_TIMEOUT"),
    ] {
        let project = Fixture::new()?;
        let runtime = project.runtime.clone();
        let key = RoundKey::from(&planning::start(&runtime, &fixture::input())?.session);
        let output = tempfile::tempdir()?;
        let mut launch = fake::launch(output.path(), mode, json!({}))?;
        launch.timeout = Duration::from_secs(2);
        let (_sender, receiver) = watch::channel(false);
        let result = run(&runtime, &key, launch, receiver, &fake::notify())
            .await
            .unwrap_err();
        assert!(result.contains(error), "{result}");
        assert!(!result.contains("test-provider-secret"));
        fake::assert_closed(output.path())?;
    }
    Ok(())
}

#[tokio::test]
async fn cancellation_closes_a_hung_handshake_and_late_round_cannot_spawn() -> Result<()> {
    let project = Fixture::new()?;
    let runtime = project.runtime.clone();
    let session = planning::start(&runtime, &fixture::input())?.session;
    let key = RoundKey::from(&session);
    let output = tempfile::tempdir()?;
    let launch = fake::launch(output.path(), "initialize-hang", json!({}))?;
    let (sender, receiver) = watch::channel(false);
    let notify: Notify = Arc::new(|_| {});
    let cancel = async {
        fake::wait_until(|| output.path().join("rpc.jsonl").exists()).await;
        sender.send(true).unwrap();
    };
    let (result, ()) = tokio::join!(run(&runtime, &key, launch, receiver, &notify), cancel);
    assert_eq!(result.unwrap_err(), "OBJECT_PLANNING_INTERRUPTED");
    fake::assert_closed(output.path())?;
    planning::cancel(
        &runtime,
        &fixture::command(&fixture::session(&runtime)?, "cancel"),
    )?;
    let late_output = tempfile::tempdir()?;
    let launch = fake::launch(late_output.path(), "proposal", json!({}))?;
    let (_sender, receiver) = watch::channel(false);
    assert!(run(&runtime, &key, launch, receiver, &notify)
        .await
        .unwrap_err()
        .contains("STALE_ROUND"));
    assert!(!late_output.path().join("lease").exists());
    Ok(())
}
