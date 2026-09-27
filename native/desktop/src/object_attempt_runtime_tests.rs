use super::call;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::{object_tasks, scheduler::Scheduler, scheduler_runtime::TaskRuntime};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn scheduler(f: &Fixture, calls: Arc<AtomicUsize>) -> Result<Scheduler> {
    let runtime = TaskRuntime::from_project(f.router.runtime_for_project("p")?);
    Ok(Scheduler::start_with_objects(
        Arc::new(move || Ok(vec![runtime.clone()])),
        Arc::new(|_, _| panic!("object controls must not call the legacy factory")),
        Arc::new(move |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("simulated launch failure".into())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    ))
}

fn request() -> Value {
    json!({
        "projectId":"p", "requestId":"stop-1", "expectedTaskRevision":2,
        "target": {
            "taskId":"medium", "fineTaskId":"fine", "objectId":"hero",
            "runId":"run", "attemptId":"attempt", "owner":"worker",
            "claimToken":"claim", "generation":1, "threadId":null, "turnId":null
        }
    })
}

async fn invalid_control(scheduler: &Scheduler, f: &Fixture, input: Value) {
    let error = call(scheduler, &f.router, "objectTask.interrupt", input)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        !error.contains("OBJECT_ATTEMPT_NOT_FOUND"),
        "invalid input reached attempt lookup: {error}"
    );
}

#[tokio::test]
async fn controls_require_complete_identity_and_reject_ambiguous_input() -> Result<()> {
    let f = Fixture::new()?;
    let scheduler = scheduler(&f, Arc::default())?;
    for method in [
        "objectTask.attempts",
        "objectTask.interrupt",
        "objectTask.attemptFile",
        "objectTask.attemptTrace",
    ] {
        assert!(call(&scheduler, &f.router, method, json!({}))
            .await
            .is_err());
    }
    let valid = request();
    let file = json!({"projectId":"p","runId":"run","attemptId":"attempt","checkpoint":"input","path":"hero.gd","sha256":"a".repeat(64)});
    assert!(crate::business_catalog::validate("objectTask.attemptFile", &file).is_ok());
    for (key, value) in [
        ("checkpoint", json!("workspace")),
        ("sha256", json!(42)),
        ("extra", json!(true)),
    ] {
        let mut invalid = file.clone();
        invalid[key] = value;
        assert!(crate::business_catalog::validate("objectTask.attemptFile", &invalid).is_err());
    }
    assert!(crate::business_catalog::validate("objectTask.interrupt", &valid).is_ok());
    for key in valid["target"].as_object().unwrap().keys() {
        let mut invalid = valid.clone();
        invalid["target"].as_object_mut().unwrap().remove(key);
        invalid_control(&scheduler, &f, invalid).await;
    }
    for (pointer, value) in [
        ("/expectedTaskRevision", json!(-1)),
        ("/requestId", json!("")),
        ("/target/generation", json!(0)),
        ("/target/taskId", json!("bad id")),
        ("/target/owner", json!("")),
        ("/target/turnId", json!("")),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        invalid_control(&scheduler, &f, invalid).await;
    }
    let mut invalid = valid;
    invalid["target"]["legacyTaskId"] = json!("medium");
    invalid_control(&scheduler, &f, invalid).await;
    assert!(
        crate::business_catalog::validate("objectTask.attempts", &json!({"projectId":"p"}))
            .is_err()
    );
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

#[tokio::test]
async fn controls_require_an_open_project_without_host_fallback() -> Result<()> {
    let f = Fixture::new()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&f, calls.clone())?;
    for project in ["closed", "legacy", "missing"] {
        assert!(call(
            &scheduler,
            &f.router,
            "objectTask.attempts",
            json!({"projectId":project,"runId":"run"})
        )
        .await
        .is_err());
        let mut input = request();
        input["projectId"] = json!(project);
        assert!(call(&scheduler, &f.router, "objectTask.interrupt", input)
            .await
            .is_err());
    }
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.interrupt",
        json!({"projectId":"p"})
    )
    .await
    .is_err());
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for kind in ["task", "object_attempt", "object_attempt_interrupt_receipt"] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn query_and_late_interrupt_preserve_the_real_result_and_project_ownership() -> Result<()> {
    let f = Fixture::new()?;
    let runtime = f.router.runtime_for_project("p")?;
    let mut draft = draft_input();
    draft["plan"]["tasks"][1]["baseline"] = json!({"basePolicy":"empty"});
    object_tasks::save_draft(&runtime, &serde_json::from_value(draft)?)?;
    object_tasks::commit(&runtime, &serde_json::from_value(commit_input())?)?;
    let snapshot = object_tasks::snapshot(&runtime, "p")?;
    let run_id = snapshot.runs[0].id.clone();
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&f, calls.clone())?;
    let query = json!({"projectId":"p","runId":run_id});
    let views = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let views = call(&scheduler, &f.router, "objectTask.attempts", query.clone()).await?;
            if views[0]["availability"] == "finished" {
                break Ok::<Value, anyhow::Error>(views);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    let view = &views[0]["attempt"];
    assert_eq!(view["state"], "failed");
    assert_eq!(view["outputCaptured"], true);
    assert_eq!(view["error"], "OBJECT_ATTEMPT_LAUNCH_FAILED");
    let trace_request =
        json!({"projectId":"p","runId":run_id,"attemptId":view["target"]["attemptId"]});
    let trace = call(
        &scheduler,
        &f.router,
        "objectTask.attemptTrace",
        trace_request.clone(),
    )
    .await?;
    assert_eq!(trace["request"], trace_request);
    assert_eq!(trace["entries"], json!([]));
    assert_eq!(trace["truncated"], false);
    let mut foreign_trace = trace_request;
    foreign_trace["runId"] = json!("foreign-run");
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.attemptTrace",
        foreign_trace
    )
    .await
    .is_err());
    let file_error = call(
        &scheduler,
        &f.router,
        "objectTask.attemptFile",
        json!({
            "projectId":"p", "runId":run_id, "attemptId":view["target"]["attemptId"],
            "checkpoint":"output", "path":"missing.gd", "sha256":"a".repeat(64)
        }),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(file_error.contains("NOT_IN_CHECKPOINT"), "{file_error}");
    let check = json!({"projectId":"p","requestId":"check-1","target":view["target"]});
    let report = call(
        &scheduler,
        &f.router,
        "objectTask.checkAttempt",
        check.clone(),
    )
    .await?;
    assert_eq!(report["passed"], true);
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.checkAttempt",
            check.clone()
        )
        .await?,
        report
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.attemptChecks",
            json!({"projectId":"p","attemptId":view["target"]["attemptId"]})
        )
        .await?,
        json!([report])
    );
    let validation_input =
        json!({"projectId":"p","attemptId":view["target"]["attemptId"],"requestId":"check-1"});
    assert!(crate::validation_runtime::is_query(
        "validation.objectReport.get"
    ));
    crate::business_catalog::validate("validation.objectReport.get", &validation_input)
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let evidence = crate::validation_runtime::object_report(&f.router, &validation_input)?;
    assert_eq!(evidence["report"], report);
    assert_eq!(evidence["source"]["target"], view["target"]);
    assert_eq!(evidence["source"]["kind"], "objectAttempt");
    assert_eq!(evidence["source"]["candidateReviews"], json!([]));
    for project in ["closed", "legacy", "missing", "other"] {
        let mut input = validation_input.clone();
        input["projectId"] = json!(project);
        assert!(crate::validation_runtime::object_report(&f.router, &input).is_err());
        let mut foreign = check.clone();
        foreign["projectId"] = json!(project);
        assert!(
            call(&scheduler, &f.router, "objectTask.checkAttempt", foreign)
                .await
                .is_err()
        );
    }
    let mut incomplete = check;
    incomplete["target"]
        .as_object_mut()
        .unwrap()
        .remove("turnId");
    assert!(
        call(&scheduler, &f.router, "objectTask.checkAttempt", incomplete)
            .await
            .is_err()
    );
    let input = json!({
        "projectId":"p","requestId":"late-stop","target":view["target"],
        "expectedTaskRevision":view["taskRevision"]
    });
    let receipt = call(&scheduler, &f.router, "objectTask.interrupt", input.clone()).await?;
    assert_eq!(receipt["request"], input);
    assert_eq!(&receipt["result"], view);
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.attempts",
            json!({"projectId":"other","runId":run_id})
        )
        .await?,
        json!([])
    );
    let mut foreign = input.clone();
    foreign["projectId"] = json!("other");
    assert!(call(&scheduler, &f.router, "objectTask.interrupt", foreign)
        .await
        .is_err());
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(
        call(&scheduler, &f.router, "objectTask.interrupt", input).await?,
        receipt
    );
    assert_eq!(
        call(&scheduler, &f.router, "objectTask.attempts", query).await?,
        views
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let queue = object_tasks::queue(&runtime, "p")?;
    assert_eq!(queue[0].state, "failed");
    assert!(queue[0].owner.is_some());
    assert!(queue[0].claim_token.is_some());
    assert_eq!(
        runtime
            .store()
            .lock()
            .unwrap()
            .list::<Value>("object_attempt_interrupt_receipt")?
            .len(),
        1
    );
    for kind in [
        "task",
        "object_task",
        "object_attempt",
        "object_attempt_interrupt_receipt",
        "object_attempt_check_report",
    ] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
    }
    Ok(())
}
