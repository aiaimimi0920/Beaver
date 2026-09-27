use super::{
    call,
    recovery_tests::{commit_plan, scheduler},
};
use crate::object_task_test_fixture::Fixture;
use anyhow::Result;
use beaver_core::{object_tasks, scheduler::Scheduler};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

pub(super) async fn finished(
    scheduler: &Scheduler,
    f: &Fixture,
    run: &str,
    count: usize,
) -> Result<Value> {
    Ok(tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let views = call(
                scheduler,
                &f.router,
                "objectTask.attempts",
                json!({"projectId":"p","runId":run}),
            )
            .await?;
            if views.as_array().is_some_and(|items| {
                items.len() == count && items.iter().all(|item| item["availability"] == "finished")
            }) {
                scheduler
                    .synchronize("object:1:pmedium".into())
                    .await
                    .map_err(anyhow::Error::msg)?;
                return Ok::<_, anyhow::Error>(views);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??)
}

#[tokio::test]
async fn confirmed_retry_launches_once_and_closed_replay_preserves_project_history() -> Result<()> {
    let f = Fixture::new()?;
    let run = commit_plan(&f)?;
    let runtime = f.router.runtime_for_project("p")?;
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&f, calls.clone())?;
    let original = finished(&scheduler, &f, &run, 1).await?;
    let query = json!({"projectId":"p","taskId":"medium"});
    let before = call(&scheduler, &f.router, "objectTask.recovery", query.clone()).await?;
    call(
        &scheduler,
        &f.router,
        "objectTask.verifyRecovery",
        json!({"projectId":"p","requestId":"verify","target":before["target"]}),
    )
    .await?;
    let verified = call(&scheduler, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(verified["canResume"], true);
    let input = json!({"projectId":"p","requestId":"resume","verificationRequestId":"verify","target":verified["target"]});
    assert!(crate::business_catalog::validate("objectTask.resumeRecovery", &input).is_ok());
    for field in ["verificationRequestId", "target", "requestId"] {
        let mut invalid = input.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(
            call(&scheduler, &f.router, "objectTask.resumeRecovery", invalid)
                .await
                .is_err()
        );
    }
    let mut foreign = input.clone();
    foreign["projectId"] = json!("other");
    assert!(
        call(&scheduler, &f.router, "objectTask.resumeRecovery", foreign)
            .await
            .is_err()
    );
    let (first, replay) = tokio::join!(
        call(
            &scheduler,
            &f.router,
            "objectTask.resumeRecovery",
            input.clone()
        ),
        call(
            &scheduler,
            &f.router,
            "objectTask.resumeRecovery",
            input.clone()
        )
    );
    let receipt = first?;
    assert_eq!(receipt["result"]["status"], "started");
    if let Ok(replayed) = replay {
        assert_eq!(replayed, receipt);
    }
    let history = finished(&scheduler, &f, &run, 2).await?;
    assert!(history.as_array().unwrap().contains(&original[0]));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.resumeRecovery",
            input.clone()
        )
        .await?,
        receipt
    );
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(runtime);
    f.router.close("p")?;
    f.router.open_registered("p")?;
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.resumeRecovery",
            input.clone()
        )
        .await?,
        receipt
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.attempts",
            json!({"projectId":"p","runId":run})
        )
        .await?,
        history
    );
    let mut changed = input;
    changed["verificationRequestId"] = json!("foreign");
    assert!(
        call(&scheduler, &f.router, "objectTask.resumeRecovery", changed)
            .await
            .is_err()
    );
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
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    Ok(())
}
