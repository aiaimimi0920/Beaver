use super::{call, recovery_tests::scheduler, resume_tests::finished};
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::{
    object_attempt, object_attempt_launch::Launch, object_tasks, scheduler::Scheduler,
    scheduler_runtime::TaskRuntime,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

#[test]
fn successful_rpc() -> Result<()> {
    if std::env::var_os("BEAVER_STAGE_ADVANCE_RPC").is_none() {
        return Ok(());
    }
    let mut stdout = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let input: Value = serde_json::from_str(&line?)?;
        let method = input["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Some(json!({})),
            "thread/start" => Some(json!({"thread":{"id":"thread"}})),
            "turn/start" => Some(json!({"turn":{"id":"turn"}})),
            _ => None,
        };
        if let Some(result) = result {
            writeln!(stdout, "{}", json!({"id":input["id"],"result":result}))?;
        }
        if method == "turn/start" {
            std::fs::write(
                "result.txt",
                if input.to_string().contains("Reduce movement speed") {
                    "feedback received: Reduce movement speed"
                } else {
                    "movement output"
                },
            )?;
            writeln!(
                stdout,
                "{}",
                json!({"method":"turn/completed","params":{
                "threadId":"thread","turn":{"id":"turn","status":"completed"}}})
            )?;
        }
        stdout.flush()?;
    }
    Ok(())
}

pub(super) fn successful_scheduler(f: &Fixture) -> Result<Scheduler> {
    let runtime = TaskRuntime::from_project(f.router.runtime_for_project("p")?);
    Ok(Scheduler::start_with_objects(
        Arc::new(move || Ok(vec![runtime.clone()])),
        Arc::new(|_, _| panic!("unexpected legacy launch")),
        Arc::new(|attempt, runtime| {
            let cwd = runtime
                .files()
                .resolve_workspace(
                    &attempt.preparation.run.id,
                    std::path::Path::new(&attempt.preparation.workspace),
                )
                .map_err(|e| e.to_string())?;
            let mut command =
                tokio::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
            command
                .args([
                    "--exact",
                    "object_attempt_runtime::advance_tests::successful_rpc",
                    "--nocapture",
                ])
                .env("BEAVER_STAGE_ADVANCE_RPC", "1")
                .current_dir(&cwd);
            Ok(Launch {
                command,
                cwd,
                model: "test-model".into(),
                secrets: vec![],
                timeout: Duration::from_secs(10),
                godot: None,
            })
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    ))
}

#[tokio::test]
async fn approved_successor_launches_once_and_reopens_with_immutable_approval() -> Result<()> {
    let f = Fixture::new()?;
    let runtime = f.router.runtime_for_project("p")?;
    let mut draft = draft_input();
    draft["plan"]["tasks"][1]["baseline"] = json!({"basePolicy":"empty"});
    draft["plan"]["tasks"].as_array_mut().unwrap().push(json!({
        "id":"fine-next","granularity":"fine","title":"Review","prompt":"Review movement",
        "acceptance":"Reviewed","objectId":"hero","parentTaskId":"medium","stageId":"review",
        "position":3,"dependsOn":["fine"]
    }));
    object_tasks::save_draft(&runtime, &serde_json::from_value(draft)?)?;
    object_tasks::commit(&runtime, &serde_json::from_value(commit_input())?)?;
    let snapshot = object_tasks::snapshot(&runtime, "p")?;
    let run = snapshot.runs[0].id.clone();
    let next = snapshot.tasks.iter().find(|t| t.id == "fine-next").unwrap();
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;
    let first = successful_scheduler(&f)?;
    let original = finished(&first, &f, &run, 1).await?;
    assert_eq!(original[0]["attempt"]["state"], "awaitingGate");
    first.shutdown().await.map_err(anyhow::Error::msg)?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&f, calls.clone())?;
    let views = call(
        &scheduler,
        &f.router,
        "objectTask.attempts",
        json!({"projectId":"p","runId":run}),
    )
    .await?;
    let query = json!({"projectId":"p","taskId":"medium"});
    let before = call(&scheduler, &f.router, "objectTask.recovery", query.clone()).await?;
    call(
        &scheduler,
        &f.router,
        "objectTask.checkAttempt",
        json!({"projectId":"p","requestId":"check","target":views[0]["attempt"]["target"]}),
    )
    .await?;
    call(
        &scheduler,
        &f.router,
        "objectTask.verifyRecovery",
        json!({"projectId":"p","requestId":"verify","target":before["target"]}),
    )
    .await?;
    let verified = call(&scheduler, &f.router, "objectTask.recovery", query).await?;
    let input = json!({"projectId":"p","requestId":"advance","target":verified["target"],"verificationRequestId":"verify",
        "advance":{"attemptId":views[0]["attempt"]["target"]["attemptId"],"checkRequestId":"check",
            "nextFineTaskId":"fine-next","nextFineRevision":next.revision,"acceptanceNote":"Owner reviewed movement"}});
    assert!(crate::business_catalog::validate("objectTask.advanceAttempt", &input).is_ok());
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.resumeRecovery",
        input.clone()
    )
    .await
    .is_err());
    let mut invalid = input.clone();
    invalid.as_object_mut().unwrap().remove("advance");
    assert!(
        call(&scheduler, &f.router, "objectTask.advanceAttempt", invalid)
            .await
            .is_err()
    );
    let mut foreign = input.clone();
    foreign["projectId"] = json!("other");
    assert!(
        call(&scheduler, &f.router, "objectTask.advanceAttempt", foreign)
            .await
            .is_err()
    );
    let (first, replay) = tokio::join!(
        call(
            &scheduler,
            &f.router,
            "objectTask.advanceAttempt",
            input.clone()
        ),
        call(
            &scheduler,
            &f.router,
            "objectTask.advanceAttempt",
            input.clone()
        )
    );
    let receipt = first?;
    assert_eq!(receipt["result"]["status"], "started");
    assert_eq!(receipt["result"]["fineTaskId"], "fine-next");
    if let Ok(replay) = replay {
        assert_eq!(receipt, replay);
    }
    finished(&scheduler, &f, &run, 2).await?;
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(runtime);
    f.router.close("p")?;
    let reopened = f.router.open_registered("p")?;
    assert_eq!(
        call(&scheduler, &f.router, "objectTask.advanceAttempt", input).await?,
        receipt
    );
    let tasks = object_tasks::snapshot(&reopened, "p")?.tasks;
    assert_eq!(
        tasks.iter().find(|t| t.id == "fine").unwrap().status,
        "accepted"
    );
    assert_eq!(object_attempt::list(&reopened, &run)?.len(), 2);
    for kind in [
        "object_attempt",
        "object_recovery_resume",
        "object_recovery_attempt_successor",
    ] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        assert!(f
            .router
            .runtime_for_project("other")?
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    Ok(())
}
