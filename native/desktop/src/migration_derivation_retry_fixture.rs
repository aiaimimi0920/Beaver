use super::*;

pub(super) struct RetrySource {
    pub first: fixture::Source,
    pub attempts: Value,
    pub verifications: Vec<Value>,
    pub resumes: Vec<Value>,
}

pub(super) async fn source(f: &Fixture) -> Result<RetrySource> {
    let first = fixture::source_history(f).await?;
    f.router.open_registered("original")?;
    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(f, "original", calls.clone())?;
    synchronize(&s, "original", "build").await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut verifications = Vec::new();
    let mut resumes = Vec::new();
    let query = json!({"projectId":"original","taskId":"build"});
    for index in 0..2 {
        let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
        let verification_id = format!("source-retry-verify-{index}");
        verifications.push(
            call(
                &s,
                &f.router,
                "objectTask.verifyRecovery",
                json!({
                    "projectId":"original","requestId":verification_id,"target":current["target"]
                }),
            )
            .await?,
        );
        let verified = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
        assert_eq!(verified["canResume"], true);
        resumes.push(
            call(
                &s,
                &f.router,
                "objectTask.resumeRecovery",
                json!({
                    "projectId":"original","requestId":format!("source-retry-resume-{index}"),
                    "verificationRequestId":verification_id,"target":verified["target"]
                }),
            )
            .await?,
        );
        assert_eq!(resumes[index]["result"]["status"], "started");
        let attempts = finished(&s, f, "original", "build", &first.run, index + 2).await?;
        assert!(attempts
            .as_array()
            .unwrap()
            .iter()
            .any(|view| view["attempt"] == first.attempt));
        assert_eq!(calls.load(Ordering::SeqCst), index + 1);
    }
    let current = call(&s, &f.router, "objectTask.recovery", query).await?;
    verifications.push(call(&s, &f.router, "objectTask.verifyRecovery", json!({
        "projectId":"original","requestId":"source-terminal-verify","target":current["target"]
    })).await?);
    let attempts = finished(&s, f, "original", "build", &first.run, 3).await?;
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close("original")?;
    Ok(RetrySource {
        first,
        attempts,
        verifications,
        resumes,
    })
}

pub(super) fn old_operation(
    f: &Fixture,
    prepared: &project_derivation_copy::Prepared,
    kind: &str,
    request: &Value,
) -> Result<Value> {
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            "original",
            request["requestId"].as_str().unwrap()
        ))?)
    );
    let runtime = f
        .router
        .runtime_for_project(&prepared.request.target_project_id)?;
    let saved: Value = runtime
        .store()
        .lock()
        .unwrap()
        .get(kind, &mapped(prepared, kind, &key))?
        .unwrap();
    Ok(saved["operation"].clone())
}

pub(super) async fn replay_history(
    f: &Fixture,
    s: &Scheduler,
    prepared: &project_derivation_copy::Prepared,
    source: &RetrySource,
) -> Result<()> {
    for (method, kind, operations) in [
        (
            "objectTask.verifyRecovery",
            "object_recovery_verification",
            &source.verifications,
        ),
        (
            "objectTask.resumeRecovery",
            "object_recovery_resume",
            &source.resumes,
        ),
    ] {
        for source_op in operations {
            let expected = old_operation(f, prepared, kind, &source_op["request"])?;
            let method = if expected["request"].get("advance").is_some() {
                "objectTask.advanceAttempt"
            } else if let Some(approval) = source_op["request"].get("rework") {
                let copied = &expected["request"]["rework"];
                assert_eq!(copied["feedback"], approval["feedback"]);
                for (field, kind) in [
                    ("reviewRequestId", "object_candidate_review"),
                    ("attemptId", "object_attempt"),
                    ("fineTaskId", "object_task"),
                ] {
                    assert_eq!(
                        copied[field],
                        mapped(prepared, kind, approval[field].as_str().unwrap())
                    );
                }
                "objectTask.reworkCandidate"
            } else {
                method
            };
            assert_eq!(
                call(s, &f.router, method, expected["request"].clone()).await?,
                expected
            );
        }
    }
    Ok(())
}
