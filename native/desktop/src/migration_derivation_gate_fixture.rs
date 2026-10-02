use super::*;
use beaver_core::{object_attempt_launch::Launch, scheduler_runtime::TaskRuntime};
use std::{sync::Mutex, time::Duration};

pub(super) type Starts = Arc<Mutex<Vec<object_attempt::Attempt>>>;

pub(super) fn successful_scheduler(
    f: &Fixture,
    project: &str,
    starts: Starts,
) -> Result<Scheduler> {
    let runtime = TaskRuntime::from_project(f.router.runtime_for_project(project)?);
    Ok(Scheduler::start_with_objects(
        Arc::new(move || Ok(vec![runtime.clone()])),
        Arc::new(|_, _| panic!("unexpected legacy launch")),
        Arc::new(move |attempt, runtime| {
            starts.lock().unwrap().push(attempt.clone());
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

pub(super) async fn source(f: &Fixture, successor: bool) -> Result<retry_fixture::RetrySource> {
    source_with_stages(f, if successor { 2 } else { 1 }).await
}

pub(super) async fn source_with_stages(
    f: &Fixture,
    stages: usize,
) -> Result<retry_fixture::RetrySource> {
    let first = fixture::source_history_with_stages(f, stages).await?;
    f.router.open_registered("original")?;
    let starts = Starts::default();
    let s = successful_scheduler(f, "original", starts.clone())?;
    let query = json!({"projectId":"original","taskId":"build"});
    let before = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    let verification = call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({
            "projectId":"original","requestId":"source-retry-verify","target":before["target"]
        }),
    )
    .await?;
    let verified = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    let resume = call(
        &s,
        &f.router,
        "objectTask.resumeRecovery",
        json!({
            "projectId":"original","requestId":"source-retry","target":verified["target"],
            "verificationRequestId":"source-retry-verify"
        }),
    )
    .await?;
    assert_eq!(resume["result"]["status"], "started");
    let attempts = finished(&s, f, "original", "build", &first.run, 2).await?;
    let candidate = attempts
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["attempt"]["state"] == "awaitingGate")
        .unwrap();
    assert_eq!(starts.lock().unwrap().len(), 1);
    call(&s, &f.router, "objectTask.checkAttempt", json!({
        "projectId":"original","requestId":"source-candidate-check","target":candidate["attempt"]["target"]
    })).await?;
    let current = call(&s, &f.router, "objectTask.recovery", query).await?;
    let terminal = call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({
            "projectId":"original","requestId":"source-gate-verify","target":current["target"]
        }),
    )
    .await?;
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close("original")?;
    Ok(retry_fixture::RetrySource {
        first,
        attempts,
        verifications: vec![verification, terminal],
        resumes: vec![resume],
    })
}

pub(super) async fn replay_controls_checks(
    f: &Fixture,
    s: &Scheduler,
    prepared: &project_derivation_copy::Prepared,
) -> Result<()> {
    let runtime = f
        .router
        .runtime_for_project(&prepared.request.target_project_id)?;
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("original", "source-unpause"))?)
    );
    let (control, checks): (Value, Vec<Value>) = {
        let store = runtime.store();
        let store = store.lock().unwrap();
        (
            store
                .get(
                    "object_task_dispatch_receipt",
                    &mapped(prepared, "object_task_dispatch_receipt", &key),
                )?
                .unwrap(),
            store.list("object_attempt_check_report")?,
        )
    };
    assert_eq!(
        f.task_api("objectTask.setPaused", control["request"].clone())?,
        control
    );
    assert!(checks.len() >= 2);
    for check in checks {
        assert_eq!(
            call(
                s,
                &f.router,
                "objectTask.checkAttempt",
                check["request"].clone()
            )
            .await?,
            check
        );
    }
    Ok(())
}

pub(super) async fn prepare_action(
    f: &Fixture,
    s: &Scheduler,
    prepared: &project_derivation_copy::Prepared,
    target: &Value,
    view: &Value,
    successor: bool,
) -> Result<(&'static str, Value)> {
    let project = &prepared.request.target_project_id;
    let check = call(
        s,
        &f.router,
        "objectTask.checkAttempt",
        json!({
            "projectId":project,"requestId":"fresh-gate-check","target":target
        }),
    )
    .await?;
    assert_eq!(check["passed"], true);
    let mut input = json!({"projectId":project,"requestId":"explicit-gate-action",
        "target":view["target"],"verificationRequestId":"fresh-gate-verify"});
    let method = if successor {
        let next_source = if target["fineTaskId"] == mapped(prepared, "object_task", "fine") {
            "later-fine"
        } else {
            "final-fine"
        };
        let next_id = mapped(prepared, "object_task", next_source);
        let snapshot = f.task_api("objectTask.snapshot", json!({"projectId":project}))?;
        let next = snapshot["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == next_id)
            .unwrap();
        input["advance"] = json!({"attemptId":target["attemptId"],"checkRequestId":"fresh-gate-check",
            "nextFineTaskId":next_id,"nextFineRevision":next["revision"],"acceptanceNote":"Reviewed copied output"});
        "objectTask.advanceAttempt"
    } else {
        let before = f.task_api("objectTask.snapshot", json!({"projectId":project}))?;
        let history_query = json!({"projectId":project,"attemptId":target["attemptId"]});
        let old_reviews = call(
            s,
            &f.router,
            "objectTask.candidateReviews",
            history_query.clone(),
        )
        .await?;
        let request = json!({"projectId":project,"requestId":"fresh-gate-review", "target":target,
            "checkRequestId":"fresh-gate-check"});
        let review = call(
            s,
            &f.router,
            "objectTask.prepareCandidateReview",
            request.clone(),
        )
        .await?;
        assert!(review["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "result.txt"));
        assert_eq!(
            call(s, &f.router, "objectTask.prepareCandidateReview", request).await?,
            review
        );
        let reviews = call(s, &f.router, "objectTask.candidateReviews", history_query).await?;
        let reviews = reviews.as_array().unwrap();
        assert_eq!(reviews.len(), old_reviews.as_array().unwrap().len() + 1);
        assert!(reviews.contains(&review));
        assert!(old_reviews
            .as_array()
            .unwrap()
            .iter()
            .all(|old| reviews.contains(old)));
        assert_eq!(
            f.task_api("objectTask.snapshot", json!({"projectId":project}))?,
            before
        );
        input["rework"] = json!({"reviewRequestId":"fresh-gate-review","attemptId":target["attemptId"],
            "fineTaskId":target["fineTaskId"],"feedback":"Reduce movement speed"});
        "objectTask.reworkCandidate"
    };
    Ok((method, input))
}
