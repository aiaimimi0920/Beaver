use super::*;
use beaver_core::object_attempt;

#[path = "migration_derivation_retry_fixture.rs"]
mod retry_fixture;
use retry_fixture::{old_operation, replay_history};

#[path = "migration_derivation_gate_tests.rs"]
mod gate;

#[tokio::test]
async fn migration_derivation_retry_history_requires_fresh_explicit_resume_and_launches_once(
) -> Result<()> {
    let _operation = super::super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let source = retry_fixture::source(&f).await?;
    let source_before = data_backup::inventory(&f.source)?;
    let other = f.router.runtime_for_project("other")?;
    let other_root = other.project_root().to_owned();
    drop(other);
    f.router.close("other")?;
    let other_before = data_backup::inventory(&other_root)?;
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
    let run = mapped(&prepared, "object_run", &source.first.run);
    let prepared_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(&f, project, calls.clone())?;
    synchronize(&s, project, &medium).await?;
    let original = finished(&s, &f, project, &medium, &run, 3).await?;
    for view in source.attempts.as_array().unwrap() {
        let id = mapped(
            &prepared,
            "object_attempt",
            view["attempt"]["target"]["attemptId"].as_str().unwrap(),
        );
        assert!(original
            .as_array()
            .unwrap()
            .iter()
            .any(
                |v| v["attempt"]["target"]["attemptId"] == id && v["attempt"]["state"] == "failed"
            ));
    }
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["hostRegistrationChanged"],
        false
    );
    replay_history(&f, &s, &prepared, &source).await?;
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(finished(&s, &f, project, &medium, &run, 3).await?, original);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close(project)?;
    f.router.open_registered(project)?;

    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(&f, project, calls.clone())?;
    synchronize(&s, project, &medium).await?;
    replay_history(&f, &s, &prepared, &source).await?;
    assert_eq!(finished(&s, &f, project, &medium, &run, 3).await?, original);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let query = json!({"projectId":project,"taskId":medium});
    let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(current["paused"], true);
    assert_eq!(current["reportMatchesRecords"], false);
    let stale = old_operation(
        &f,
        &prepared,
        "object_recovery_verification",
        &source.verifications.last().unwrap()["request"],
    )?;
    assert!(call(&s, &f.router, "objectTask.resumeRecovery", json!({
        "projectId":project,"requestId":"stale-copy-resume",
        "verificationRequestId":stale["request"]["requestId"],"target":stale["request"]["target"]
    })).await.is_err());
    call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({
            "projectId":project,"requestId":"copy-paused-verify","target":current["target"]
        }),
    )
    .await?;
    let paused = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(paused["canResume"], false);
    unpause(&f, project, &medium, "explicit-copy-unpause", false)?;
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(finished(&s, &f, project, &medium, &run, 3).await?, original);
    assert!(call(
        &s,
        &f.router,
        "objectTask.resumeRecovery",
        json!({
            "projectId":project,"requestId":"paused-copy-resume",
            "verificationRequestId":"copy-paused-verify","target":paused["target"]
        })
    )
    .await
    .is_err());
    let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    let previous_id = mapped(
        &prepared,
        "object_attempt",
        source.resumes.last().unwrap()["result"]["attemptId"]
            .as_str()
            .unwrap(),
    );
    let runtime = f.router.runtime_for_project(project)?;
    let previous = object_attempt::list(&runtime, &run)?
        .into_iter()
        .find(|a| a.id == previous_id)
        .unwrap();
    call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({
            "projectId":project,"requestId":"copy-fresh-verify","target":current["target"]
        }),
    )
    .await?;
    let verified = call(&s, &f.router, "objectTask.recovery", query).await?;
    assert_eq!(verified["canResume"], true);
    let input = json!({"projectId":project,"requestId":"copy-fresh-resume",
        "verificationRequestId":"copy-fresh-verify","target":verified["target"]});
    let (first, second) = tokio::join!(
        call(&s, &f.router, "objectTask.resumeRecovery", input.clone()),
        call(&s, &f.router, "objectTask.resumeRecovery", input.clone())
    );
    let (receipt, replay) = match (first, second) {
        (Ok(receipt), replay) | (replay, Ok(receipt)) => (receipt, replay),
        (Err(first), Err(second)) => {
            anyhow::bail!("both explicit resumes failed: {first}; {second}")
        }
    };
    assert_eq!(receipt["result"]["status"], "started");
    match replay {
        Ok(replayed) => assert_eq!(replayed, receipt),
        Err(error) => assert_eq!(error.to_string(), "OBJECT_RECOVERY_WRITER_ACTIVE"),
    }
    let history = finished(&s, &f, project, &medium, &run, 4).await?;
    assert!(original
        .as_array()
        .unwrap()
        .iter()
        .all(|v| history.as_array().unwrap().contains(v)));
    let fresh_id = receipt["result"]["attemptId"].as_str().unwrap();
    let fresh = object_attempt::list(&runtime, &run)?
        .into_iter()
        .find(|a| a.id == fresh_id)
        .unwrap();
    assert_eq!(Some(&fresh.input), previous.output.as_ref());
    assert!(fresh.thread_id.is_none() && fresh.turn_id.is_none());
    assert!(!original
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["attempt"]["target"]["attemptId"] == fresh.id));
    assert_eq!(
        call(&s, &f.router, "objectTask.resumeRecovery", input).await?,
        receipt
    );
    replay_history(&f, &s, &prepared, &source).await?;
    synchronize(&s, project, &medium).await?;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(finished(&s, &f, project, &medium, &run, 4).await?, history);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    for kind in [
        "object_attempt",
        "object_recovery_verification",
        "object_recovery_head",
        "object_recovery_resume",
        "object_recovery_resume_head",
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
    Ok(())
}
