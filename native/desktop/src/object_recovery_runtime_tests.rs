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

pub(super) fn scheduler(fixture: &Fixture, calls: Arc<AtomicUsize>) -> Result<Scheduler> {
    let runtime = TaskRuntime::from_project(fixture.router.runtime_for_project("p")?);
    Ok(Scheduler::start_with_objects(
        Arc::new(move || Ok(vec![runtime.clone()])),
        Arc::new(|_, _| panic!("object controls must not call legacy factory")),
        Arc::new(move |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("simulated launch failure".into())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    ))
}

pub(super) fn commit_plan(fixture: &Fixture) -> Result<String> {
    let runtime = fixture.router.runtime_for_project("p")?;
    let mut draft = draft_input();
    draft["plan"]["tasks"][1]["baseline"] = json!({"basePolicy":"empty"});
    object_tasks::save_draft(&runtime, &serde_json::from_value(draft)?)?;
    object_tasks::commit(&runtime, &serde_json::from_value(commit_input())?)?;
    Ok(object_tasks::snapshot(&runtime, "p")?.runs[0].id.clone())
}

fn request() -> Value {
    json!({
        "projectId":"p", "requestId":"verify-1", "target": {
            "taskId":"medium", "objectId":"hero", "runId":"run", "owner":"worker",
            "claimToken":"claim", "writerGeneration":1, "taskRevision":3,
            "runRevision":2, "objectRevision":1, "controlRevision":0, "recoveryGeneration":0
        }
    })
}

#[tokio::test]
async fn recovery_query_remains_durable_after_scheduler_shutdown() -> Result<()> {
    let fixture = Fixture::new()?;
    commit_plan(&fixture)?;
    let runtime = fixture.router.runtime_for_project("p")?;
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;

    let scheduler = scheduler(&fixture, Arc::new(AtomicUsize::new(0)))?;
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    let after = call(
        &scheduler,
        &fixture.router,
        "objectTask.recovery",
        json!({"projectId":"p","taskId":"medium"}),
    )
    .await?;
    assert_eq!(after["target"]["taskId"], "medium");
    assert_eq!(after["preparationState"], "ready");
    assert_eq!(after["canDispose"], false);
    Ok(())
}

#[tokio::test]
async fn unprepared_query_and_unopened_projects_never_create_recovery_or_execution() -> Result<()> {
    let fixture = Fixture::new()?;
    commit_plan(&fixture)?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&fixture, calls.clone())?;
    assert_eq!(
        call(
            &scheduler,
            &fixture.router,
            "objectTask.recovery",
            json!({"projectId":"p","taskId":"medium"})
        )
        .await?,
        Value::Null
    );
    assert_eq!(
        call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            request()
        )
        .await
        .unwrap_err()
        .to_string(),
        "OBJECT_RECOVERY_NOT_REQUIRED"
    );
    for project in ["closed", "legacy", "missing"] {
        assert!(call(
            &scheduler,
            &fixture.router,
            "objectTask.recovery",
            json!({"projectId":project,"taskId":"medium"})
        )
        .await
        .is_err());
        let mut input = request();
        input["projectId"] = json!(project);
        assert!(call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            input
        )
        .await
        .is_err());
    }
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for kind in [
        "object_attempt",
        "object_recovery_verification",
        "object_recovery_head",
    ] {
        assert!(fixture.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        for project in ["p", "other"] {
            assert!(fixture
                .router
                .runtime_for_project(project)?
                .store()
                .lock()
                .unwrap()
                .list::<Value>(kind)?
                .is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn recovery_routes_reject_incomplete_identity_unknown_fields_and_unsafe_revisions(
) -> Result<()> {
    let fixture = Fixture::new()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&fixture, calls.clone())?;
    let valid = request();
    assert!(crate::business_catalog::validate("objectTask.verifyRecovery", &valid).is_ok());
    let mut invalid = vec![(json!({}), "INVALID_INPUT:".to_owned())];
    for key in valid["target"].as_object().unwrap().keys() {
        let mut input = valid.clone();
        input["target"].as_object_mut().unwrap().remove(key);
        invalid.push((input, format!("missing field `{key}`")));
    }
    let invalid_request = "INVALID_OBJECT_RECOVERY_REQUEST";
    for (pointer, value, expected) in [
        ("/requestId", json!(""), invalid_request),
        ("/target/taskId", json!("bad id"), invalid_request),
        ("/target/owner", json!(""), invalid_request),
        ("/target/writerGeneration", json!(0), invalid_request),
        ("/target/taskRevision", json!(-1), "invalid value:"),
        ("/target/runRevision", json!(1.5), "invalid type:"),
        (
            "/target/objectRevision",
            json!(9_007_199_254_740_992_u64),
            invalid_request,
        ),
        ("/target/controlRevision", json!("1"), "invalid type:"),
        (
            "/target/recoveryGeneration",
            json!(9_007_199_254_740_991_u64),
            invalid_request,
        ),
    ] {
        let mut input = valid.clone();
        *input.pointer_mut(pointer).unwrap() = value;
        invalid.push((input, expected.to_owned()));
    }
    let mut extra = valid.clone();
    extra["force"] = json!(true);
    invalid.push((extra, "INVALID_INPUT:".to_owned()));
    let mut extra = valid;
    extra["target"]["attemptId"] = json!("attempt");
    invalid.push((extra, "unknown field `attemptId`".to_owned()));
    for (input, expected) in invalid {
        let error = call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            input,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(
            error.starts_with(&expected),
            "expected {expected}, got {error}"
        );
    }
    for input in [
        json!({}),
        json!({"projectId":"p"}),
        json!({"projectId":"p","taskId":"medium","force":true}),
    ] {
        let error = call(&scheduler, &fixture.router, "objectTask.recovery", input)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("INVALID_INPUT:"), "{error}");
    }
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn verification_then_disposition_survive_shutdown_and_project_reopen() -> Result<()> {
    let fixture = Fixture::new()?;
    let run_id = commit_plan(&fixture)?;
    let runtime = fixture.router.runtime_for_project("p")?;
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;
    let calls = Arc::new(AtomicUsize::new(0));
    let first = scheduler(&fixture, calls.clone())?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let views = call(
                &first,
                &fixture.router,
                "objectTask.attempts",
                json!({"projectId":"p","runId":run_id}),
            )
            .await?;
            if views[0]["availability"] == "finished" {
                assert_eq!(views[0]["attempt"]["state"], "failed");
                assert_eq!(views[0]["attempt"]["outputCaptured"], true);
                break Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    first.shutdown().await.map_err(anyhow::Error::msg)?;
    // A fresh loop proves the live message route without racing the old active entry's removal.
    let scheduler = scheduler(&fixture, calls.clone())?;
    let query = json!({"projectId":"p","taskId":"medium"});
    let before = call(
        &scheduler,
        &fixture.router,
        "objectTask.recovery",
        query.clone(),
    )
    .await?;
    assert_eq!(before["operation"], Value::Null);
    let input = json!({"projectId":"p","requestId":"verify-1","target":before["target"]});
    let mut foreign = input.clone();
    foreign["projectId"] = json!("other");
    assert!(call(
        &scheduler,
        &fixture.router,
        "objectTask.verifyRecovery",
        foreign
    )
    .await
    .is_err());
    for field in ["taskId", "objectId", "runId"] {
        let mut foreign = input.clone();
        foreign["target"][field] = json!("foreign");
        assert!(call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            foreign
        )
        .await
        .is_err());
    }
    let queue = object_tasks::queue(&runtime, "p")?;
    let snapshot = serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?;
    let receipt = call(
        &scheduler,
        &fixture.router,
        "objectTask.verifyRecovery",
        input.clone(),
    )
    .await?;
    assert_eq!(receipt["request"], input);
    assert_eq!(receipt["generation"], 1);
    assert_eq!(
        receipt["result"],
        json!({
            "recordsCurrent":true,"writerStatus":"stopRecorded","contentStatus":"verified",
            "workspaceStatus":"matchesCheckpoint","paused":false,"issues":[]
        })
    );
    let after = call(
        &scheduler,
        &fixture.router,
        "objectTask.recovery",
        query.clone(),
    )
    .await?;
    assert_eq!(after["operation"], receipt);
    assert_eq!(after["reportMatchesRecords"], true);
    assert_eq!(after["canDispose"], true);
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    assert_eq!(
        call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            input.clone()
        )
        .await?,
        receipt
    );
    drop(runtime);
    fixture.router.close("p")?;
    let reopened = fixture.router.open_registered("p")?;
    assert_eq!(
        call(&scheduler, &fixture.router, "objectTask.recovery", query).await?,
        after
    );
    assert_eq!(
        call(
            &scheduler,
            &fixture.router,
            "objectTask.verifyRecovery",
            input
        )
        .await?,
        receipt
    );
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&reopened, "p")?)?,
        snapshot
    );
    assert_eq!(
        serde_json::to_value(object_tasks::queue(&reopened, "p")?)?,
        serde_json::to_value(&queue)?
    );
    assert_eq!(queue[0].state, "failed");
    assert!(queue[0].owner.is_some() && queue[0].claim_token.is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for kind in ["object_recovery_verification", "object_recovery_head"] {
        assert_eq!(
            reopened.store().lock().unwrap().list::<Value>(kind)?.len(),
            1
        );
        assert!(fixture.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        assert!(fixture
            .router
            .runtime_for_project("other")?
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
    }
    let workspace = reopened.files().workspace(&run_id)?;
    let disposition = json!({
        "projectId":"p", "requestId":"remove-workspace", "target":after["target"],
        "verificationRequestId":"verify-1", "choice":"cancelAndRemoveWorkspace"
    });
    let result = call(
        &scheduler,
        &fixture.router,
        "objectTask.disposeRecovery",
        disposition.clone(),
    )
    .await?;
    assert_eq!(result["result"]["status"], "cancelledAndWorkspaceRemoved");
    assert!(!workspace.exists());
    drop(reopened);
    fixture.router.close("p")?;
    fixture.router.open_registered("p")?;
    assert_eq!(
        call(
            &scheduler,
            &fixture.router,
            "objectTask.disposeRecovery",
            disposition
        )
        .await?,
        result
    );
    let history = call(
        &scheduler,
        &fixture.router,
        "objectTask.recovery",
        json!({"projectId":"p","taskId":"medium"}),
    )
    .await?;
    assert_eq!(history["disposition"], result);
    let attempts = call(
        &scheduler,
        &fixture.router,
        "objectTask.attempts",
        json!({"projectId":"p","runId":run_id}),
    )
    .await?;
    assert_eq!(attempts.as_array().unwrap().len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    Ok(())
}
