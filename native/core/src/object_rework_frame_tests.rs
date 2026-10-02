use super::*;
use crate::{
    object_attempt::{self, State as AttemptState},
    object_attempt_callback as callback, object_attempt_checks as checks,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::{self as recovery, candidate, resume},
};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicBool, Arc};

#[cfg(windows)]
#[tokio::test]
async fn candidate_rework_frame_reopens_retries_delivers_rpc_and_retains_publication_feedback(
) -> Result<()> {
    let f = fixture()?;
    let (original, mut feedback) = image_feedback(&f)?;
    let image = feedback.image.clone();
    let mut frame = super::frames::attach(&f, &mut feedback)?;
    let verify = super::super::object_recovery_verification::request(&f, "verify-frame")?;
    recovery::verify(&f.runtime, &verify, false)?;
    let request = resume::Request {
        project_id: "project-1".into(),
        request_id: "rework-frame".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: verify.request_id,
        advance: None,
        rework: Some(resume::rework::Approval {
            review_request_id: original.review_request_id,
            attempt_id: original.target.attempt_id,
            fine_task_id: "fine-b".into(),
            feedback: "Brighten numbered label".into(),
            image: None,
            preview_frame: feedback.preview_frame.clone(),
            relocation: None,
        }),
    };
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut bad = request.clone();
    bad.rework.as_mut().unwrap().image = image;
    assert!(resume::execute(&f.runtime, &bad, &AtomicBool::new(false)).is_err());
    bad.rework.as_mut().unwrap().image = None;
    bad.rework
        .as_mut()
        .unwrap()
        .preview_frame
        .as_mut()
        .unwrap()
        .frame_id = "foreign".into();
    assert!(resume::execute(&f.runtime, &bad, &AtomicBool::new(false)).is_err());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert!(resume::execute_with(
        &f.runtime,
        &request,
        &AtomicBool::new(false),
        || anyhow::bail!("host stopped")
    )
    .is_err());
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let f = Fixture {
        runtime: crate::project_storage::ProjectStore::open(temp.path(), "project-1")?
            .into_runtime(),
        temp,
    };
    assert!(resume::execute(&f.runtime, &bad, &AtomicBool::new(false))
        .err()
        .unwrap()
        .to_string()
        .contains("CONFLICT"));
    let (receipt, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    let mut lease = lease.unwrap();
    assert!(lease.record().fine.prompt.contains("feedbackImage"));
    assert_eq!(
        resume::execute(&f.runtime, &request, &AtomicBool::new(false))?.0,
        receipt
    );
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let reply = callback::call(
        &f.runtime,
        &lease,
        &json!({"tool":callback::TOOL,"threadId":"thread","turnId":"turn","arguments":{"operation":"feedbackImage"}}),
    )?;
    let data_url = frame["frame"]
        .as_object_mut()
        .unwrap()
        .remove("dataUrl")
        .unwrap();
    assert_eq!(reply["contentItems"][1]["imageUrl"], data_url);
    let context = callback::context(&f.runtime, &lease)?;
    assert_eq!(context["feedbackImage"]["previewFrame"], frame);
    assert_eq!(context["feedbackImage"]["reworkRequestId"], "rework-frame");
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::Interrupted,
        Some("interrupted".into()),
        &AtomicBool::new(false),
    )?;
    let verify = super::super::object_recovery_verification::request(&f, "retry-frame-verify")?;
    recovery::verify(&f.runtime, &verify, false)?;
    let (_, lease) = resume::execute(
        &f.runtime,
        &resume::Request {
            project_id: "project-1".into(),
            request_id: "retry-frame".into(),
            target: recovery::get(&f.runtime, "project-1", "head")?
                .unwrap()
                .target,
            verification_request_id: verify.request_id,
            advance: None,
            rework: None,
        },
        &AtomicBool::new(false),
    )?;
    let lease = lease.unwrap();
    assert_eq!(
        callback::context(&f.runtime, &lease)?["feedbackImage"],
        context["feedbackImage"]
    );
    let target = crate::object_attempt_view::Target::from_record(lease.record());
    let root = f.temp.path().join("rework-frame-rpc");
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
                .map_err(|e| e.to_string())?;
            super::super::object_attempt_rpc_fixture::launch(
                &launch_root,
                &cwd,
                "feedback-image-callback",
            )
            .map_err(|e| e.to_string())
        }),
        Arc::new(AtomicBool::new(false)),
        receiver,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    super::super::object_attempt_rpc_fixture::assert_closed(&root)?;
    let current = attempts(&f, "head")?
        .into_iter()
        .find(|a| a.id == target.attempt_id)
        .unwrap();
    assert_eq!(
        current.state,
        AttemptState::AwaitingGate,
        "{:?}",
        current.error
    );
    let delivered: Value = serde_json::from_slice(&std::fs::read(
        f.runtime
            .files()
            .blob(&current.output.as_ref().unwrap()["image-delivered.json"])?,
    )?)?;
    assert_eq!(delivered["contentItems"][1]["imageUrl"], data_url);
    let provenance: Value =
        serde_json::from_str(delivered["contentItems"][2]["text"].as_str().unwrap())?;
    assert_eq!(provenance["previewFrame"], frame);
    let target = crate::object_attempt_view::Target::from_record(&current);
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-frame-rework".into(),
            target: target.clone(),
        },
    )?;
    let mut publish = request_for_publication(&f, target)?;
    let preview = publication::preview(&f.runtime, &review(&publish))?;
    assert_eq!(preview.feedback.len(), 1);
    assert_eq!(preview.feedback[0].preview_frame, feedback.preview_frame);
    assert_eq!(
        preview.feedback[0].attempt_id,
        feedback.review.target.attempt_id
    );
    super::frames::approve(&f, &mut publish)?;
    assert_eq!(
        publication::publish(&f.runtime, &publish)?.state,
        State::Published
    );
    assert_eq!(
        deferred::frames::list(
            &f.runtime,
            "project-1",
            &feedback.review.target.attempt_id,
            feedback.preview_frame.as_ref()
        )?
        .len(),
        1
    );
    Ok(())
}

fn request_for_publication(
    f: &Fixture,
    target: crate::object_attempt_view::Target,
) -> Result<Request> {
    request(
        f,
        &candidate::Request {
            project_id: "project-1".into(),
            request_id: "review-frame-rework".into(),
            check_request_id: "check-frame-rework".into(),
            target,
        },
    )
}
