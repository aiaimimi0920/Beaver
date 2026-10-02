use super::*;

#[path = "migration_derivation_candidate_fixture.rs"]
mod candidate_fixture;
#[path = "migration_derivation_gate_fixture.rs"]
mod gate_fixture;
#[path = "migration_derivation_rework_fixture.rs"]
mod rework_fixture;
#[path = "migration_derivation_stage_fixture.rs"]
mod stage_fixture;
use gate_fixture::{replay_controls_checks, successful_scheduler, Starts};

#[tokio::test]
async fn migration_derivation_gate_advances_only_after_explicit_new_approval() -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(true, false, false, false).await
}

#[tokio::test]
async fn migration_derivation_gate_final_candidate_rework_launches_once_with_feedback() -> Result<()>
{
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(false, false, false, false).await
}

#[tokio::test]
async fn migration_derivation_reviewed_candidate_requires_fresh_review_and_consumes_feedback_once(
) -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(false, true, false, false).await
}

#[tokio::test]
async fn migration_derivation_stage_history_advances_next_fine_only_once() -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(true, false, true, false).await
}

#[tokio::test]
async fn migration_derivation_stage_history_requires_fresh_review_and_consumes_feedback_once(
) -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(false, true, true, false).await
}

#[tokio::test]
async fn migration_derivation_rework_history_reopens_and_only_fresh_feedback_launches_once(
) -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(false, true, false, true).await
}

#[tokio::test]
async fn migration_derivation_staged_rework_history_preserves_accepted_prefix_and_launches_once(
) -> Result<()> {
    let _operation = super::super::super::super::super::TEST_OPERATION
        .lock()
        .unwrap();
    gate_flow(false, true, true, true).await
}

async fn gate_flow(successor: bool, reviewed: bool, staged: bool, reworked: bool) -> Result<()> {
    let f = Fixture::new()?;
    let mut source = if staged {
        stage_fixture::source(&f, successor).await?
    } else {
        gate_fixture::source(&f, successor).await?
    };
    let historical_reviews = if reworked {
        rework_fixture::extend_source(&f, &mut source).await?
    } else {
        vec![]
    };
    let count = source.attempts.as_array().unwrap().len();
    let source_review = if reviewed {
        Some(candidate_fixture::create(&f, &source).await?)
    } else {
        None
    };
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
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    let prepared = project_derivation_copy::inspect(&f.preparation)?;
    let project = &prepared.request.target_project_id;
    let medium = mapped(&prepared, "object_task", "build");
    let run = mapped(&prepared, "object_run", &source.first.run);
    let fine = mapped(
        &prepared,
        "object_task",
        if staged { "later-fine" } else { "fine" },
    );
    let prepared_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let starts = Starts::default();
    let s = successful_scheduler(&f, project, starts.clone())?;
    let original = finished(&s, &f, project, &medium, &run, count).await?;
    for source_view in source.attempts.as_array().unwrap() {
        let id = mapped(
            &prepared,
            "object_attempt",
            source_view["attempt"]["target"]["attemptId"]
                .as_str()
                .unwrap(),
        );
        let view = original
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["attempt"]["target"]["attemptId"] == id)
            .unwrap();
        assert_eq!(view["attempt"]["state"], source_view["attempt"]["state"]);
    }
    let terminal_id = mapped(
        &prepared,
        "object_attempt",
        source.resumes.last().unwrap()["result"]["attemptId"]
            .as_str()
            .unwrap(),
    );
    let terminal = original
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["attempt"]["target"]["attemptId"] == terminal_id)
        .unwrap();
    assert_eq!(terminal["attempt"]["target"]["fineTaskId"], fine);
    assert_eq!(terminal["attempt"]["state"], "awaitingGate");
    let target = terminal["attempt"]["target"].clone();
    replay_history(&f, &s, &prepared, &source).await?;
    replay_controls_checks(&f, &s, &prepared).await?;
    let old_review = candidate_fixture::replay(&f, &s, &prepared, source_review.as_ref()).await?;
    rework_fixture::replay_reviews(&f, &s, &prepared, &historical_reviews).await?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["hostRegistrationChanged"],
        false
    );
    let query = json!({"projectId":project,"taskId":medium});
    let view = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    assert_eq!(view["paused"], true);
    assert_eq!(view["canResume"], false);
    assert_eq!(view["canDispose"], false);
    synchronize(&s, project, &medium).await?;
    assert!(starts.lock().unwrap().is_empty());
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close(project)?;
    let runtime = f.router.open_registered(project)?;
    let candidate = object_attempt::list(&runtime, &run)?
        .into_iter()
        .find(|a| a.id == target["attemptId"].as_str().unwrap())
        .unwrap();
    let s = successful_scheduler(&f, project, starts.clone())?;
    assert_eq!(
        finished(&s, &f, project, &medium, &run, count).await?,
        original
    );
    replay_history(&f, &s, &prepared, &source).await?;
    replay_controls_checks(&f, &s, &prepared).await?;
    candidate_fixture::replay(&f, &s, &prepared, source_review.as_ref()).await?;
    rework_fixture::replay_reviews(&f, &s, &prepared, &historical_reviews).await?;
    unpause(&f, project, &medium, "gate-copy-unpause", false)?;
    synchronize(&s, project, &medium).await?;
    assert!(starts.lock().unwrap().is_empty());
    let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    call(
        &s,
        &f.router,
        "objectTask.verifyRecovery",
        json!({
            "projectId":project,"requestId":"fresh-gate-verify","target":current["target"]
        }),
    )
    .await?;
    let verified = call(&s, &f.router, "objectTask.recovery", query).await?;
    assert_eq!(verified["canDispose"], true);
    assert_eq!(verified["canResume"], false);
    if let Some(old) = &old_review {
        candidate_fixture::reject_old_authority(&f, &s, old, &verified).await?;
        synchronize(&s, project, &medium).await?;
        assert!(starts.lock().unwrap().is_empty());
    }
    let (method, input) =
        gate_fixture::prepare_action(&f, &s, &prepared, &target, &verified, successor).await?;
    synchronize(&s, project, &medium).await?;
    assert!(starts.lock().unwrap().is_empty());
    let (first, replay) = tokio::join!(
        call(&s, &f.router, method, input.clone()),
        call(&s, &f.router, method, input.clone())
    );
    let receipt = first?;
    assert_eq!(receipt["result"]["status"], "started");
    match replay {
        Ok(replayed) => assert_eq!(replayed, receipt),
        Err(error) => assert_eq!(error.to_string(), "OBJECT_RECOVERY_WRITER_ACTIVE"),
    }
    let history = finished(&s, &f, project, &medium, &run, count + 1).await?;
    assert!(original
        .as_array()
        .unwrap()
        .iter()
        .all(|v| history.as_array().unwrap().contains(v)));
    let started = {
        let starts = starts.lock().unwrap();
        assert_eq!(starts.len(), 1);
        starts[0].clone()
    };
    assert_eq!(Some(&started.input), candidate.output.as_ref());
    assert!(started.thread_id.is_none() && started.turn_id.is_none());
    assert_ne!(started.id, candidate.id);
    assert_eq!(started.id, receipt["result"]["attemptId"].as_str().unwrap());
    let snapshot = beaver_core::object_tasks::snapshot(&runtime, project)?;
    let first_fine = snapshot.tasks.iter().find(|t| t.id == fine).unwrap();
    if staged {
        assert_eq!(
            snapshot
                .tasks
                .iter()
                .find(|t| t.id == mapped(&prepared, "object_task", "fine"))
                .unwrap()
                .status,
            "accepted"
        );
    }
    if successor {
        assert_eq!(first_fine.status, "accepted");
        assert_eq!(
            started.fine.id,
            mapped(
                &prepared,
                "object_task",
                if staged { "final-fine" } else { "later-fine" }
            )
        );
    } else {
        assert_eq!(first_fine.status, "awaitingAcceptance");
        assert_eq!(started.fine.id, fine);
        assert!(started.fine.prompt.contains("Reduce movement speed"));
        let workspace = runtime
            .files()
            .resolve_workspace(&run, std::path::Path::new(&started.preparation.workspace))?;
        assert_eq!(
            fs::read_to_string(workspace.join("result.txt"))?,
            "feedback received: Reduce movement speed"
        );
    }
    assert_eq!(call(&s, &f.router, method, input.clone()).await?, receipt);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    drop(runtime);
    f.router.close(project)?;
    f.router.open_registered(project)?;
    let s = successful_scheduler(&f, project, starts.clone())?;
    assert_eq!(call(&s, &f.router, method, input).await?, receipt);
    candidate_fixture::replay(&f, &s, &prepared, source_review.as_ref()).await?;
    rework_fixture::replay_reviews(&f, &s, &prepared, &historical_reviews).await?;
    replay_history(&f, &s, &prepared, &source).await?;
    assert_eq!(
        finished(&s, &f, project, &medium, &run, count + 1).await?,
        history
    );
    assert_eq!(starts.lock().unwrap().len(), 1);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close(project)?;
    for kind in [
        "object_attempt",
        "object_recovery_verification",
        "object_recovery_resume",
        "object_candidate_review",
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
