use super::*;
use crate::{
    object_catalog_test_fixture::Fixture, object_task_planning_fixture as fixture,
    object_task_planning_rpc_fixture as fake,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn duplicate_commands_spend_once_and_an_immediate_answer_waits_for_disposal() -> Result<()> {
    let project = Fixture::new()?;
    let output = tempfile::tempdir()?;
    let root = output.path().to_owned();
    let count = Arc::new(AtomicUsize::new(0));
    let invoked = count.clone();
    let service = Service::new(
        Arc::new(move |runtime, key| {
            let index = invoked.fetch_add(1, Ordering::SeqCst);
            if index > 0 {
                fake::assert_closed(&root.join("0"))?;
            }
            let output = root.join(index.to_string());
            std::fs::create_dir(&output)?;
            fake::launch(
                &output,
                if index == 0 { "questions" } else { "proposal" },
                round::context(runtime, key)?,
            )
        }),
        fake::notify(),
        1,
    );
    let input = fixture::input();
    service.start(project.runtime.clone(), &input)?;
    service.start(project.runtime.clone(), &input)?;
    fake::wait_until(|| {
        fixture::session(&project.runtime).unwrap().status == Status::AwaitingInput
    })
    .await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let waiting = fixture::session(&project.runtime)?;
    let answer = AnswerRequest {
        project_id: waiting.project_id.clone(),
        session_id: waiting.id.clone(),
        request_id: "answer".into(),
        expected_revision: waiting.revision,
        answers: [("style".into(), "User choice".into())].into(),
    };
    service.answer(project.runtime.clone(), &answer)?;
    service.answer(project.runtime.clone(), &answer)?;
    fake::wait_until(|| fixture::session(&project.runtime).unwrap().status == Status::Proposed)
        .await;
    assert_eq!(count.load(Ordering::SeqCst), 2);
    let proposed = fixture::session(&project.runtime)?;
    assert_eq!(
        proposed.decisions[0].source,
        crate::object_tasks::AssumptionSource::User
    );
    assert_eq!(proposed.decisions[0].answer, "User choice");
    service.adopt(&project.runtime, &fixture::command(&proposed, "adopt"))?;
    service.shutdown().await?;
    fake::assert_closed(&output.path().join("1"))?;
    assert_eq!(project.count("task")?, 0);
    assert_eq!(project.count("object_task")?, 0);
    Ok(())
}

#[tokio::test]
async fn capacity_rejection_and_shutdown_leave_no_running_or_leaked_jobs() -> Result<()> {
    let project = Fixture::new()?;
    let output = tempfile::tempdir()?;
    let root = output.path().to_owned();
    let service = Service::new(
        Arc::new(move |_, _| fake::launch(&root, "hang", serde_json::json!({}))),
        fake::notify(),
        1,
    );
    service.start(project.runtime.clone(), &fixture::input())?;
    fake::wait_until(|| output.path().join("rpc.jsonl").exists()).await;
    let mut second = fixture::input();
    second.draft_id = "other".into();
    second.request_id = "other-start".into();
    let rejected = service.start(project.runtime.clone(), &second)?;
    assert_eq!(rejected.status, Status::Failed);
    assert!(rejected.error.unwrap().contains("CAPACITY"));
    service.shutdown().await?;
    fake::assert_closed(output.path())?;
    assert_eq!(
        fixture::session(&project.runtime)?.status,
        Status::Interrupted
    );
    assert!(service.0.state.lock().unwrap().jobs.is_empty());
    second.request_id = "after-shutdown".into();
    assert_eq!(
        service.start(project.runtime.clone(), &second)?.status,
        Status::Interrupted
    );
    Ok(())
}

#[tokio::test]
async fn cancellation_is_durable_before_worker_exit_and_retry_does_not_relaunch() -> Result<()> {
    let project = Fixture::new()?;
    let output = tempfile::tempdir()?;
    let root = output.path().to_owned();
    let service = Service::new(
        Arc::new(move |_, _| fake::launch(&root, "hang", serde_json::json!({}))),
        fake::notify(),
        1,
    );
    let input = fixture::input();
    service.start(project.runtime.clone(), &input)?;
    fake::wait_until(|| {
        fixture::session(&project.runtime)
            .unwrap()
            .turn_id
            .is_some()
    })
    .await;
    let cancel = fixture::command(&fixture::session(&project.runtime)?, "cancel");
    assert_eq!(
        service.cancel(&project.runtime, &cancel)?.status,
        Status::Cancelled
    );
    service.cancel(&project.runtime, &cancel)?;
    assert_eq!(
        service.start(project.runtime.clone(), &input)?.status,
        Status::Cancelled
    );
    service.shutdown().await?;
    fake::assert_closed(output.path())?;
    assert_eq!(
        fixture::session(&project.runtime)?.status,
        Status::Cancelled
    );
    Ok(())
}

#[tokio::test]
async fn preparation_failure_and_recovery_do_not_relaunch_without_a_new_command() -> Result<()> {
    let project = Fixture::new()?;
    let count = Arc::new(AtomicUsize::new(0));
    let invoked = count.clone();
    let service = Service::new(
        Arc::new(move |_, _| {
            invoked.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("TEST_PREPARATION_FAILED")
        }),
        fake::notify(),
        1,
    );
    let mut input = fixture::input();
    service.start(project.runtime.clone(), &input)?;
    fake::wait_until(|| fixture::session(&project.runtime).unwrap().status == Status::Failed).await;
    service.start(project.runtime.clone(), &input)?;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    input.request_id = "recovered".into();
    planning::start(&project.runtime, &input)?;
    planning::recover(&mut project.runtime.store().lock().unwrap())?;
    assert_eq!(
        service.start(project.runtime.clone(), &input)?.status,
        Status::Interrupted
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    service.shutdown().await?;
    Ok(())
}
