//! Build the source through the production Desktop advance API before deriving it.
use super::*;

pub(super) async fn source(f: &Fixture, successor: bool) -> Result<retry_fixture::RetrySource> {
    let mut source = gate_fixture::source_with_stages(f, if successor { 3 } else { 2 }).await?;
    f.router.open_registered("original")?;
    let starts = Starts::default();
    let s = successful_scheduler(f, "original", starts.clone())?;
    let query = json!({"projectId":"original","taskId":"build"});
    let current = call(&s, &f.router, "objectTask.recovery", query.clone()).await?;
    let previous = &source.resumes.last().unwrap()["result"]["attemptId"];
    let receipt = call(&s, &f.router, "objectTask.advanceAttempt", json!({
        "projectId":"original","requestId":"source-stage-advance","target":current["target"],
        "verificationRequestId":"source-gate-verify","advance":{
            "attemptId":previous,"checkRequestId":"source-candidate-check",
            "nextFineTaskId":"later-fine","nextFineRevision":0,"acceptanceNote":"Accepted source first stage"
        }
    })).await?;
    assert_eq!(receipt["result"]["status"], "started");
    source.attempts = finished(&s, f, "original", "build", &source.first.run, 3).await?;
    assert_eq!(starts.lock().unwrap().len(), 1);
    let current = call(&s, &f.router, "objectTask.recovery", query).await?;
    source.verifications.push(call(&s, &f.router, "objectTask.verifyRecovery", json!({
        "projectId":"original","requestId":"source-stage-terminal-verify","target":current["target"]
    })).await?);
    source.resumes.push(receipt);
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    f.router.close("original")?;
    Ok(source)
}
