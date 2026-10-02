use super::{advance_tests::successful_scheduler, call, resume_tests::finished};
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::object_tasks;
use serde_json::json;

#[tokio::test]
async fn publication_routes_accept_and_retain_history_without_redispatch() -> Result<()> {
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
    assert_eq!(views[0]["attempt"]["state"], "awaitingGate", "{views:#}");
    let target = views[0]["attempt"]["target"].clone();
    call(
        &scheduler,
        &f.router,
        "objectTask.checkAttempt",
        json!({"projectId":"p","requestId":"check","target":target}),
    )
    .await?;
    call(
        &scheduler,
        &f.router,
        "objectTask.prepareCandidateReview",
        json!({"projectId":"p","requestId":"review","target":target,"checkRequestId":"check"}),
    )
    .await?;
    let review = json!({"projectId":"p","target":target,"reviewRequestId":"review"});
    let deferred = json!({"projectId":"p","requestId":"defer","review":review,
        "feedback":"Improve next time", "later":{"title":"Follow-up","acceptance":"Visible improvement"}});
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.attemptFrames",
            json!({"projectId":"p","attemptId":target["attemptId"]})
        )
        .await?,
        json!([])
    );
    let mut invalid = deferred.clone();
    invalid["relocation"] = json!({"sourceAttemptId":target["attemptId"],
        "sourceFrame":{"runId":"old","frameId":"old"},"confirmed":true,
        "regions":[{"status":"matched","sourceRegion":0,"targetRegion":0}]});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.deferCandidateFeedback",
        invalid.clone()
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("FEEDBACK_RELOCATION_TARGET_REQUIRED"));
    invalid["previewFrame"] = json!({"runId":"missing","frameId":"missing"});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.deferCandidateFeedback",
        invalid
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("FEEDBACK_RELOCATION_SOURCE_MISMATCH"));
    let mut invalid = deferred.clone();
    invalid["previewFrame"] = json!({"runId":"missing","frameId":"missing"});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.deferCandidateFeedback",
        invalid
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("PREVIEW_SAVED_NOT_FOUND"));
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.publicationPreview",
        review.clone()
    )
    .await?["feedback"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.deferCandidateFeedback",
            deferred.clone()
        )
        .await?,
        deferred
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.deferCandidateFeedback",
            deferred.clone()
        )
        .await?,
        deferred
    );
    let before = serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?;
    let preview = call(
        &scheduler,
        &f.router,
        "objectTask.publicationPreview",
        review.clone(),
    )
    .await?;
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?,
        before
    );
    let query = json!({"projectId":"p","taskId":"medium"});
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.publications",
            query.clone()
        )
        .await?,
        json!([])
    );
    let mut foreign = review.clone();
    foreign["projectId"] = json!("other");
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.publicationPreview",
        foreign
    )
    .await
    .is_err());
    let mut request = review;
    request["requestId"] = json!("publish");
    request["previewDigest"] = preview["digest"].clone();
    request["acceptanceNote"] = json!("Reviewed complete output");
    request["confirmFiles"] = json!(true);
    request["confirmReplacement"] = json!(true);
    request["feedback"] = json!([{"requestId":preview["feedback"][0]["requestId"],"resolution":"deferred","note":"Schedule after publication"}]);
    let mut unexpected_final = request.clone();
    unexpected_final["feedback"][0]["finalRelocation"] = json!({
        "sourceDigest":"c".repeat(64),"confirmed":true,
        "targetFrame":{"runId":"missing","frameId":"missing"},
        "regions":[{"status":"absent","sourceRegion":0,"note":"No counterpart"}]
    });
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.publishCandidate",
        unexpected_final
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("OBJECT_PUBLICATION_RELOCATION_SOURCE_CHANGED"));
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?,
        before
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.publications",
            query.clone()
        )
        .await?,
        json!([])
    );
    let published = call(
        &scheduler,
        &f.router,
        "objectTask.publishCandidate",
        request.clone(),
    )
    .await?;
    assert_eq!(published["state"], "published");
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.publicationFrames",
            json!({"projectId":"p","publicationRequestId":"publish"})
        )
        .await?,
        json!([])
    );
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.publicationFrames",
        json!({"projectId":"p","publicationRequestId":"publish",
            "previewFrame":{"runId":"missing","frameId":"missing"}})
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("PREVIEW_SAVED_NOT_FOUND"));
    let planned = call(
        &scheduler,
        &f.router,
        "objectTask.publicationFollowups",
        json!({"projectId":"p","publicationRequestId":"publish"}),
    )
    .await?;
    assert_eq!(planned.as_array().unwrap().len(), 1);
    let followup = json!({
        "projectId":"p","requestId":"followup","publicationRequestId":"publish",
        "versionId":published["versionId"],"title":"Next improvement",
        "feedback":"Improve the output","acceptance":"The requested improvement is visible"
    });
    let receipt = call(
        &scheduler,
        &f.router,
        "objectTask.createPublicationFollowup",
        followup.clone(),
    )
    .await?;
    let before_invalid_frame = object_tasks::snapshot(&runtime, "p")?;
    let mut invalid_frame = followup.clone();
    invalid_frame["requestId"] = json!("invalid-frame");
    invalid_frame["previewFrame"] = json!({"runId":"missing","frameId":"missing"});
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.createPublicationFollowup",
        invalid_frame
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("PREVIEW_SAVED_NOT_FOUND"));
    assert_eq!(object_tasks::snapshot(&runtime, "p")?, before_invalid_frame);
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.createPublicationFollowup",
            followup
        )
        .await?,
        receipt
    );
    let receipts = call(
        &scheduler,
        &f.router,
        "objectTask.publicationFollowups",
        json!({"projectId":"p","publicationRequestId":"publish"}),
    )
    .await?;
    let receipts = receipts.as_array().unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts.contains(&planned[0]));
    assert!(receipts.contains(&receipt));
    let snapshot = object_tasks::snapshot(&runtime, "p")?;
    let medium = snapshot
        .tasks
        .iter()
        .find(|task| task.id == receipt["mediumTaskId"].as_str().unwrap())
        .unwrap();
    assert_eq!(medium.status, "planned");
    assert_eq!(
        serde_json::to_value(&medium.identity)?["baseline"]["selectedVersionId"],
        published["versionId"]
    );
    assert_eq!(
        call(
            &scheduler,
            &f.router,
            "objectTask.publishCandidate",
            request
        )
        .await?,
        published
    );
    assert_eq!(
        call(&scheduler, &f.router, "objectTask.publications", query).await?,
        json!([published])
    );
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.abortPublication",
        json!({"projectId":"p","requestId":"publish","confirmAbort":false})
    )
    .await
    .is_err());
    assert!(call(
        &scheduler,
        &f.router,
        "objectTask.abortPublication",
        json!({"projectId":"p","requestId":"publish","confirmAbort":true})
    )
    .await
    .is_err());
    assert_eq!(beaver_core::object_attempt::list(&runtime, &run)?.len(), 1);
    assert_ne!(
        serde_json::to_value(object_tasks::snapshot(&runtime, "p")?)?,
        before
    );
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}
