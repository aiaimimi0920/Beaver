//! Reviewed-source branch of the real dispatcher / Scheduler derivation workflow.
use super::*;

pub(super) async fn create(f: &Fixture, source: &retry_fixture::RetrySource) -> Result<Value> {
    f.router.open_registered("original")?;
    let starts = Starts::default();
    let s = successful_scheduler(f, "original", starts.clone())?;
    let target = &source
        .attempts
        .as_array()
        .unwrap()
        .iter()
        .find(|v| {
            v["attempt"]["target"]["attemptId"]
                == source.resumes.last().unwrap()["result"]["attemptId"]
        })
        .unwrap()["attempt"]["target"];
    call(
        &s,
        &f.router,
        "objectTask.checkAttempt",
        json!({
            "projectId":"original", "requestId":"source-final-review-check", "target":target
        }),
    )
    .await?;
    let review = call(
        &s,
        &f.router,
        "objectTask.prepareCandidateReview",
        json!({
            "projectId":"original", "requestId":"source-frozen-review", "target":target,
            "checkRequestId":"source-final-review-check"
        }),
    )
    .await?;
    synchronize(&s, "original", "build").await?;
    assert!(starts.lock().unwrap().is_empty());
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close("original")?;
    Ok(review)
}

pub(super) async fn replay(
    f: &Fixture,
    s: &Scheduler,
    prepared: &project_derivation_copy::Prepared,
    source: Option<&Value>,
) -> Result<Option<Value>> {
    let Some(source) = source else {
        return Ok(None);
    };
    let project = &prepared.request.target_project_id;
    let request_id = mapped(
        prepared,
        "object_candidate_review",
        source["request"]["requestId"].as_str().unwrap(),
    );
    let target = &source["request"]["target"];
    let attempt_id = mapped(
        prepared,
        "object_attempt",
        target["attemptId"].as_str().unwrap(),
    );
    let reviews = call(
        s,
        &f.router,
        "objectTask.candidateReviews",
        json!({
            "projectId":project, "attemptId":attempt_id
        }),
    )
    .await?;
    let old = reviews
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["request"]["requestId"] == request_id)
        .unwrap()
        .clone();
    assert_eq!(old["request"]["projectId"], *project);
    assert_eq!(
        old["request"]["checkRequestId"],
        mapped(
            prepared,
            "object_attempt_check_report",
            source["request"]["checkRequestId"].as_str().unwrap()
        )
    );
    assert_eq!(old["request"]["target"]["attemptId"], attempt_id);
    assert_ne!(old["request"]["target"]["owner"], target["owner"]);
    assert_ne!(old["request"]["target"]["claimToken"], target["claimToken"]);
    for key in [
        "schemaVersion",
        "time",
        "outputDigest",
        "rules",
        "blockers",
        "objectRevisionAtReview",
    ] {
        assert_eq!(old[key], source[key], "{key}");
    }
    assert_ne!(old["sourceDigest"], source["sourceDigest"]);
    assert_eq!(
        call(
            s,
            &f.router,
            "objectTask.prepareCandidateReview",
            old["request"].clone()
        )
        .await?,
        old
    );
    Ok(Some(old))
}

pub(super) async fn reject_old_authority(
    f: &Fixture,
    s: &Scheduler,
    old: &Value,
    verified: &Value,
) -> Result<()> {
    let request = &old["request"];
    let project = request["projectId"].as_str().unwrap();
    let before = f.task_api("objectTask.snapshot", json!({"projectId":project}))?;
    let error = call(s, &f.router, "objectTask.reworkCandidate", json!({
        "projectId":project, "requestId":"stale-review-cannot-authorize", "target":verified["target"],
        "verificationRequestId":"fresh-gate-verify", "rework":{
            "reviewRequestId":request["requestId"], "attemptId":request["target"]["attemptId"],
            "fineTaskId":request["target"]["fineTaskId"], "feedback":"Reduce movement speed"
        }
    })).await.unwrap_err();
    assert_eq!(error.to_string(), "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH");
    assert_eq!(
        f.task_api("objectTask.snapshot", json!({"projectId":project}))?,
        before
    );
    Ok(())
}
