use super::*;
use crate::object_attempt_runtime::call;
use anyhow::Context;
use beaver_core::{project_derivation_copy, scheduler::Scheduler};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "migration_derivation_execution_fixture.rs"]
mod fixture;
use fixture::{finished, mapped, scheduler, synchronize, unpause};

#[path = "migration_derivation_retry_tests.rs"]
mod retry;

#[tokio::test]
async fn migration_derivation_execution_active_consumer_never_launches_without_fresh_explicit_resume(
) -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let source = fixture::source_history(&f).await?;
    let source_before =
        data_backup::inventory(&f.source).context("source inventory before derivation")?;
    let other = f.router.runtime_for_project("other")?;
    let other_root = other.project_root().to_owned();
    drop(other);
    f.router.close("other")?;
    let other_before = data_backup::inventory(&other_root)
        .context("offline other-project inventory before derivation")?;
    let other = f.router.open_registered("other")?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let project = inspection["request"]["targetProjectId"].as_str().unwrap();
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    let prepared = project_derivation_copy::inspect(&f.preparation)?;
    let medium = mapped(&prepared, "object_task", "build");
    let run = mapped(&prepared, "object_run", &source.run);
    let attempt = mapped(
        &prepared,
        "object_attempt",
        source.attempt["target"]["attemptId"].as_str().unwrap(),
    );
    let check = mapped(&prepared, "object_attempt_check_report", "source-check");
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("original", "source-unpause"))?)
    );
    let receipt_key = mapped(&prepared, "object_task_dispatch_receipt", &key);
    let prepared_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let runtime = f.router.runtime_for_project(project)?;
    let old_control: Value = runtime
        .store()
        .lock()
        .unwrap()
        .get("object_task_dispatch_receipt", &receipt_key)?
        .unwrap();
    let old_check: Value = runtime
        .store()
        .lock()
        .unwrap()
        .get("object_attempt_check_report", &check)?
        .unwrap();
    assert_ne!(old_check["attemptDigest"], source.check["attemptDigest"]);
    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(&f, project, calls.clone())?;
    synchronize(&s, project, &medium).await?;
    let original = finished(&s, &f, project, &medium, &run, 1).await?;
    assert_eq!(original[0]["attempt"]["target"]["attemptId"], attempt);
    assert_eq!(original[0]["attempt"]["state"], "failed");
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["hostRegistrationChanged"],
        false
    );
    assert_eq!(
        f.task_api("objectTask.setPaused", old_control["request"].clone())?,
        old_control
    );
    assert_eq!(
        call(
            &s,
            &f.router,
            "objectTask.checkAttempt",
            old_check["request"].clone()
        )
        .await?,
        old_check
    );
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    drop(runtime);
    f.router.close(project)?;
    f.router.open_registered(project)?;

    // A genuinely new, active scheduler is required to prove reopen/replay safety.
    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(&f, project, calls.clone())?;
    synchronize(&s, project, &medium).await?;
    assert_eq!(finished(&s, &f, project, &medium, &run, 1).await?, original);
    assert_eq!(
        f.task_api("objectTask.setPaused", old_control["request"].clone())?,
        old_control
    );
    assert_eq!(
        call(
            &s,
            &f.router,
            "objectTask.checkAttempt",
            old_check["request"].clone()
        )
        .await?,
        old_check
    );
    let query = json!({"projectId":project,"taskId":medium});
    let before = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(before["paused"], true);
    call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({"projectId":project,"requestId":"paused-verify","target":before["target"]}),
    )
    .await?;
    let stale = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(stale["canResume"], false);
    unpause(&f, project, &medium, "explicit-copy-unpause", false)?;
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(finished(&s, &f, project, &medium, &run, 1).await?, original);
    assert!(call(&s, &f.router, "objectTask.resumeRecovery", json!({"projectId":project,"requestId":"stale-resume","verificationRequestId":"paused-verify","target":stale["target"]})).await.is_err());
    let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({"projectId":project,"requestId":"fresh-verify","target":current["target"]}),
    )
    .await?;
    let verified = call(&s, &f.router, "objectTask.recovery", query).await?;
    assert_eq!(verified["canResume"], true);
    let input = json!({"projectId":project,"requestId":"fresh-resume","verificationRequestId":"fresh-verify","target":verified["target"]});
    let (first, replay) = tokio::join!(
        call(&s, &f.router, "objectTask.resumeRecovery", input.clone()),
        call(&s, &f.router, "objectTask.resumeRecovery", input.clone())
    );
    let receipt = first?;
    assert_eq!(receipt["result"]["status"], "started");
    match replay {
        Ok(replayed) => assert_eq!(replayed, receipt),
        Err(error) => assert_eq!(error.to_string(), "OBJECT_RECOVERY_WRITER_ACTIVE"),
    }
    let history = finished(&s, &f, project, &medium, &run, 2).await?;
    assert!(history.as_array().unwrap().contains(&original[0]));
    assert_ne!(
        history[0]["attempt"]["target"]["attemptId"],
        history[1]["attempt"]["target"]["attemptId"]
    );
    assert_eq!(
        call(&s, &f.router, "objectTask.resumeRecovery", input).await?,
        receipt
    );
    assert_eq!(
        f.task_api("objectTask.setPaused", old_control["request"].clone())?,
        old_control
    );
    assert_eq!(
        call(
            &s,
            &f.router,
            "objectTask.checkAttempt",
            old_check["request"].clone()
        )
        .await?,
        old_check
    );
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(finished(&s, &f, project, &medium, &run, 2).await?, history);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    for kind in [
        "object_attempt",
        "object_recovery_verification",
        "object_recovery_head",
        "object_recovery_resume",
        "object_recovery_attempt_successor",
    ] {
        assert!(f.store.lock().unwrap().list::<Value>(kind)?.is_empty());
        assert!(other
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
    }
    assert_eq!(data_backup::inventory(&f.source)?, source_before);
    assert_eq!(data_backup::inventory(&f.preparation)?, prepared_before);
    drop(other);
    f.router.close("other")?;
    assert_eq!(data_backup::inventory(&other_root)?, other_before);
    assert_eq!(source.unpaused["result"]["paused"], false);
    Ok(())
}
