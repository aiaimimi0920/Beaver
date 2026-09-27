use super::*;
use crate::{
    object_attempt::{self, Lease, State as AttemptState},
    object_attempt_callback as callback, object_attempt_checks as checks,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{self as recovery, candidate, resume},
};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

fn historical_publication(f: &Fixture) -> Result<(followup::Receipt, deferred::Request)> {
    historical_with_frame(f, false)
}

fn historical_with_frame(
    f: &Fixture,
    archived: bool,
) -> Result<(followup::Receipt, deferred::Request)> {
    let (original, mut feedback) = image_feedback(f)?;
    if archived {
        super::frames::attach(f, &mut feedback)?;
    }
    deferred::create(&f.runtime, &feedback)?;
    // The accepted PNG now differs from the older PNG the owner commented on.
    let verify = super::super::object_recovery_verification::request(f, "verify-image")?;
    recovery::verify(&f.runtime, &verify, false)?;
    let (_, lease) = resume::execute(
        &f.runtime,
        &resume::Request {
            project_id: "project-1".into(),
            request_id: "rework-image".into(),
            target: recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .target,
            verification_request_id: verify.request_id,
            advance: None,
            rework: Some(resume::rework::Approval {
                review_request_id: original.review_request_id,
                attempt_id: original.target.attempt_id,
                fine_task_id: "fine-b".into(),
                feedback: "Change background".into(),
                image: None,
                preview_frame: None,
                relocation: None,
            }),
        },
        &AtomicBool::new(false),
    )?;
    let lease = lease.unwrap();
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 255, 0, 255]))
        .save(workspace(f, lease.record())?.join("preview.png"))?;
    let target = crate::object_attempt_view::Target::from_record(lease.record());
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-new-image".into(),
            target: target.clone(),
        },
    )?;
    let mut publish = request(
        f,
        &candidate::Request {
            project_id: "project-1".into(),
            request_id: "review-new-image".into(),
            check_request_id: "check-new-image".into(),
            target,
        },
    )?;
    let preview = publication::preview(&f.runtime, &review(&publish))?;
    publish.feedback = preview
        .feedback
        .iter()
        .map(|item| FeedbackDecision {
            request_id: item.request_id.clone(),
            resolution: if item.later.is_some() {
                Resolution::Deferred
            } else {
                Resolution::Resolved
            },
            note: "Reviewed".into(),
        })
        .collect();
    publication::publish(&f.runtime, &publish)?;
    let receipt = followup::list(&f.runtime, "project-1", "publish")?.remove(0);
    assert_ne!(receipt.source.attempt_id, feedback.review.target.attempt_id);
    Ok((receipt, feedback))
}

pub(super) fn start_followup(f: &Fixture, receipt: &followup::Receipt) -> Result<Lease> {
    object_tasks::cancel_planned(
        &f.runtime,
        &crate::object_task_types::CancelPlannedRequest {
            project_id: "project-1".into(),
            task_id: "next".into(),
            request_id: "cancel-next".into(),
            expected_task_revision: task_record(f, "next")?.revision,
            expected_plan_revision: object_tasks::snapshot(&f.runtime, "project-1")?.plan_revision,
        },
    )?;
    object_tasks::enqueue(&f.runtime, "project-1", &[receipt.medium_task_id.clone()])?;
    let claim = crate::object_run_preparation::claim_next(&f.runtime, "project-1", "image-worker")?
        .unwrap();
    Ok(object_attempt::start(&f.runtime, claim)?.unwrap())
}

fn params() -> Value {
    json!({"tool":callback::TOOL,"threadId":"thread","turnId":"turn",
        "arguments":{"operation":"feedbackImage"}})
}

#[cfg(windows)]
#[tokio::test]
async fn publication_deferred_image_survives_reopen_and_delivers_original_over_rpc() -> Result<()> {
    deliver_original(false).await
}

#[cfg(windows)]
#[tokio::test]
async fn publication_deferred_frame_survives_new_candidate_reopen_and_rpc() -> Result<()> {
    deliver_original(true).await
}

#[cfg(windows)]
async fn deliver_original(archived: bool) -> Result<()> {
    use base64::Engine;
    use std::sync::Arc;
    let f = fixture()?;
    let (receipt, feedback) = historical_with_frame(&f, archived)?;
    let frame = if archived {
        Some(
            deferred::frames::list(
                &f.runtime,
                "project-1",
                &feedback.review.target.attempt_id,
                feedback.preview_frame.as_ref(),
            )?
            .remove(0),
        )
    } else {
        None
    };
    let image = feedback.image.as_ref();
    let original_hash = frame
        .as_ref()
        .map(|v| v["source"]["target"]["sha256"].as_str().unwrap())
        .unwrap_or_else(|| &image.unwrap().sha256);
    let bytes = std::fs::read(f.runtime.files().blob(original_hash)?)?;
    assert!(receipt.request.preview_frame.is_none());
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let f = Fixture {
        runtime: crate::project_storage::ProjectStore::open(temp.path(), "project-1")?
            .into_runtime(),
        temp,
    };
    let lease = start_followup(&f, &receipt)?;
    assert_ne!(lease.record().input["preview.png"], original_hash);
    let root = f.temp.path().join("feedback-rpc");
    let launch_root = root.clone();
    let (_controls, receiver) = tokio::sync::mpsc::channel(16);
    crate::object_attempt_worker::execute_lease(
        f.runtime.clone(),
        lease,
        Arc::new(move |attempt, runtime| {
            let cwd = runtime
                .files()
                .resolve_workspace(
                    &attempt.preparation.run.id,
                    std::path::Path::new(&attempt.preparation.workspace),
                )
                .map_err(|error| error.to_string())?;
            super::super::object_attempt_rpc_fixture::launch(
                &launch_root,
                &cwd,
                "feedback-image-callback",
            )
            .map_err(|error| error.to_string())
        }),
        Arc::new(AtomicBool::new(false)),
        receiver,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    super::super::object_attempt_rpc_fixture::assert_closed(&root)?;
    let attempt = attempts(&f, &receipt.medium_task_id)?.remove(0);
    assert_eq!(
        attempt.state,
        AttemptState::AwaitingGate,
        "{:?}",
        attempt.error
    );
    let hash = &attempt.output.as_ref().unwrap()["image-delivered.json"];
    let reply: Value = serde_json::from_slice(&std::fs::read(f.runtime.files().blob(hash)?)?)?;
    let items = reply["contentItems"].as_array().unwrap();
    assert_eq!(
        items[1]["imageUrl"],
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        )
    );
    let provenance: Value = serde_json::from_str(items[2]["text"].as_str().unwrap())?;
    if let Some(mut frame) = frame {
        frame["frame"].as_object_mut().unwrap().remove("dataUrl");
        assert_eq!(provenance["previewFrame"], frame);
    } else {
        assert_eq!(
            provenance["file"]["attemptId"],
            feedback.review.target.attempt_id
        );
        assert_eq!(provenance["file"]["checkpoint"], "output");
        assert_eq!(provenance["image"], serde_json::to_value(image.unwrap())?);
    }
    assert_eq!(provenance["versionId"], receipt.request.version_id);
    assert_eq!(
        task_record(&f, &receipt.medium_task_id)?.status,
        "awaitingAcceptance"
    );
    Ok(())
}

#[test]
fn publication_deferred_image_rejects_redirect_corruption_and_interrupt() -> Result<()> {
    let f = fixture()?;
    let (receipt, feedback) = historical_publication(&f)?;
    let mut lease = start_followup(&f, &receipt)?;
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let read = params();
    for field in ["path", "attemptId", "sha256"] {
        let mut redirected = read.clone();
        redirected["arguments"][field] = json!("foreign");
        assert!(callback::call(&f.runtime, &lease, &redirected).is_err());
    }
    let mut wrong_turn = read.clone();
    wrong_turn["turnId"] = json!("foreign");
    assert!(callback::call(&f.runtime, &lease, &wrong_turn).is_err());
    let blob = f.runtime.files().blob(&feedback.image.unwrap().sha256)?;
    let bytes = std::fs::read(&blob)?;
    std::fs::write(&blob, b"tampered")?;
    assert!(callback::call(&f.runtime, &lease, &read)
        .unwrap_err()
        .to_string()
        .contains("HASH_MISMATCH"));
    std::fs::write(&blob, bytes)?;
    assert_eq!(
        callback::call(&f.runtime, &lease, &read)?["contentItems"][1]["type"],
        "inputImage"
    );
    crate::object_attempt_control::request(
        &f.runtime,
        &interrupt_request(&f, &receipt.medium_task_id)?,
        true,
    )?;
    assert!(callback::call(&f.runtime, &lease, &read)
        .unwrap_err()
        .to_string()
        .contains("INTERRUPTED"));
    Ok(())
}

#[test]
fn publication_deferred_image_is_unavailable_to_unrelated_fine() -> Result<()> {
    let f = fixture()?;
    let mut lease = start(&f)?;
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    assert!(callback::context(&f.runtime, &lease)?["feedbackImage"].is_null());
    assert!(callback::call(&f.runtime, &lease, &params())
        .unwrap_err()
        .to_string()
        .contains("UNAVAILABLE"));
    Ok(())
}
