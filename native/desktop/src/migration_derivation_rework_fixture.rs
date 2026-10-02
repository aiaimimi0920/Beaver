//! Build repeated rework history through the real API and Scheduler before deriving it.
use super::*;

pub(super) async fn extend_source(
    f: &Fixture,
    source: &mut retry_fixture::RetrySource,
) -> Result<Vec<Value>> {
    f.router.open_registered("original")?;
    let starts = Starts::default();
    let s = successful_scheduler(f, "original", starts.clone())?;
    let query = json!({"projectId":"original","taskId":"build"});
    let mut reviews = Vec::new();
    for name in ["z-source-rework", "a-source-rework"] {
        let target = source
            .attempts
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["attempt"]["target"]["attemptId"]
                    == source.resumes.last().unwrap()["result"]["attemptId"]
            })
            .unwrap()["attempt"]["target"]
            .clone();
        let check_id = format!("{name}-check");
        let review_id = format!("{name}-review");
        let verification_id = format!("{name}-verify");
        let check = call(
            &s,
            &f.router,
            "objectTask.checkAttempt",
            json!({
                "projectId":"original", "requestId":check_id, "target":target
            }),
        )
        .await?;
        assert_eq!(check["passed"], true);
        reviews.push(call(&s, &f.router, "objectTask.prepareCandidateReview", json!({
            "projectId":"original", "requestId":review_id, "checkRequestId":check_id, "target":target
        })).await?);
        let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
        source.verifications.push(
            call(
                &s,
                &f.router,
                "objectTask.verifyRecovery",
                json!({
                    "projectId":"original", "requestId":verification_id, "target":current["target"]
                }),
            )
            .await?,
        );
        let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
        let feedback = format!(
            "Reduce movement speed; keep original/{review_id}/{} as 用户反馈",
            target["attemptId"].as_str().unwrap()
        );
        let request = json!({
            "projectId":"original", "requestId":name, "target":current["target"],
            "verificationRequestId":verification_id, "rework":{
                "reviewRequestId":review_id, "attemptId":target["attemptId"], "fineTaskId":target["fineTaskId"], "feedback":feedback
            }
        });
        let receipt = call(&s, &f.router, "objectTask.reworkCandidate", request.clone()).await?;
        assert_eq!(receipt["result"]["status"], "started");
        source.attempts = finished(
            &s,
            f,
            "original",
            "build",
            &source.first.run,
            source.attempts.as_array().unwrap().len() + 1,
        )
        .await?;
        assert_eq!(
            call(&s, &f.router, "objectTask.reworkCandidate", request).await?,
            receipt
        );
        assert_eq!(starts.lock().unwrap().len(), reviews.len());
        assert!(starts
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .fine
            .prompt
            .ends_with(&feedback));
        source.resumes.push(receipt);
    }
    let current = call(&s, &f.router, "objectTask.recovery", query).await?;
    source.verifications.push(call(&s, &f.router, "objectTask.verifyRecovery", json!({
        "projectId":"original", "requestId":"source-rework-terminal-verify", "target":current["target"]
    })).await?);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close("original")?;
    Ok(reviews)
}

pub(super) async fn replay_reviews(
    f: &Fixture,
    s: &Scheduler,
    prepared: &project_derivation_copy::Prepared,
    reviews: &[Value],
) -> Result<()> {
    for review in reviews {
        candidate_fixture::replay(f, s, prepared, Some(review)).await?;
    }
    Ok(())
}
