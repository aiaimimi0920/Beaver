use super::{advance_tests::successful_scheduler, call, resume_tests::finished};
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::object_tasks;
use serde_json::json;

#[tokio::test]
async fn candidate_review_routes_final_output_and_replays_without_dispatch() -> Result<()> {
    let f = Fixture::new()?;
    let runtime = f.router.runtime_for_project("p")?;
    let mut draft = draft_input();
    draft["plan"]["tasks"][1]["baseline"] = json!({"basePolicy":"empty"});
    object_tasks::save_draft(&runtime, &serde_json::from_value(draft)?)?;
    object_tasks::commit(&runtime, &serde_json::from_value(commit_input())?)?;
    let run = object_tasks::snapshot(&runtime, "p")?.runs[0].id.clone();
    object_tasks::enqueue(&runtime, "p", &["medium".into()])?;
    let scheduler = successful_scheduler(&f)?;
    let views = finished(&scheduler, &f, &run, 1).await?;
    let target = views[0]["attempt"]["target"].clone();
    call(
        &scheduler,
        &f.router,
        "objectTask.checkAttempt",
        json!({"projectId":"p","requestId":"check","target":target}),
    )
    .await?;
    let input =
        json!({"projectId":"p","requestId":"review","target":target,"checkRequestId":"check"});
    let before = serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?;
    let report = call(
        &scheduler,
        &f.router,
        "objectTask.prepareCandidateReview",
        input.clone(),
    )
    .await?;
    assert!(report["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["path"] == "result.txt"));
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.prepareCandidateReview",
            input.clone()
        )
        .await?,
        report
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.candidateReviews",
            json!({"projectId":"p","attemptId":target["attemptId"]})
        )
        .await?,
        json!([report])
    );
    let mut foreign = input.clone();
    foreign["projectId"] = json!("other");
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.prepareCandidateReview",
        foreign
    )
    .await
    .is_err());
    let mut extra = input;
    extra["publish"] = json!(true);
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.prepareCandidateReview",
        extra
    )
    .await
    .is_err());
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?,
        before
    );
    let query = json!({"projectId":"p","taskId":"medium"});
    let view = call(&scheduler, &f.router, "objectTask.recovery", query.clone()).await?;
    call(
        &scheduler,
        &f.router,
        "objectTask.verifyRecovery",
        json!({"projectId":"p","requestId":"verify-rework","target":view["target"]}),
    )
    .await?;
    let view = call(&scheduler, &f.router, "objectTask.recovery", query).await?;
    let rework = json!({"projectId":"p","requestId":"rework","target":view["target"],
        "verificationRequestId":"verify-rework","rework":{
        "reviewRequestId":"review","attemptId":target["attemptId"],
        "fineTaskId":target["fineTaskId"],"feedback":"Reduce movement speed"}});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.resumeRecovery",
        rework.clone()
    )
    .await
    .is_err());
    let mut foreign_image = rework.clone();
    foreign_image["rework"]["image"] = json!({"path":"foreign.png","sha256":"a".repeat(64),
        "width":20,"height":10,"regions":[{"x":0,"y":0,"width":1,"height":1,"prompt":"Fix"}]});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.reworkCandidate",
        foreign_image
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("OBJECT_REWORK_IMAGE_SOURCE_MISMATCH"));
    let mut foreign_frame = rework.clone();
    foreign_frame["rework"]["previewFrame"] =
        json!({"runId":"missing-preview","frameId":"missing-frame"});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.reworkCandidate",
        foreign_frame
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("PREVIEW_SAVED_NOT_FOUND"));
    let receipt = call(
        &scheduler,
        &f.router,
        "objectTask.reworkCandidate",
        rework.clone(),
    )
    .await?;
    assert_eq!(receipt["result"]["status"], "started");
    let after = finished(&scheduler, &f, &run, 2).await?;
    assert_eq!(after.as_array().unwrap().len(), 2);
    assert!(after.as_array().unwrap().contains(&views[0]));
    let records = beaver_core::object_attempt::list(&runtime, &run)?;
    let latest = records
        .iter()
        .find(|a| a.id == receipt["result"]["attemptId"].as_str().unwrap())
        .unwrap();
    let workspace = runtime
        .files()
        .resolve_workspace(&run, std::path::Path::new(&latest.preparation.workspace))?;
    assert_eq!(
        std::fs::read_to_string(workspace.join("result.txt"))?,
        "feedback received: Reduce movement speed"
    );
    assert_eq!(
        call(&scheduler, &f.router, "objectTask.reworkCandidate", rework).await?,
        receipt
    );
    assert_eq!(beaver_core::object_attempt::list(&runtime, &run)?.len(), 2);
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}
