use super::*;
use crate::{
    object_attempt, object_attempt_callback as callback, object_catalog_test_fixture::Fixture,
};
use serde_json::{json, Value};

#[path = "object_publication_frame_fixture.rs"]
mod fixture;
use fixture::{archive, attach, start_followup};

#[test]
fn publication_frame_picker_bounds_recent_history_but_old_references_still_resolve() -> Result<()> {
    let f = fixture()?;
    let op = publication::publish(&f.runtime, &ready(&f)?)?;
    let original = archive(&f, &op)?;
    let store = f.runtime.store();
    {
        let store = store.lock().unwrap();
        let run: Value = store.get("validationRun", "preview-run")?.unwrap();
        for index in 1..=10 {
            let id = format!("preview-run-{index}");
            let mut next_run = run.clone();
            next_run["id"] = json!(id);
            let mut frame = original.clone();
            frame["runId"] = json!(id);
            frame["source"]["runId"] = json!(id);
            frame["savedAt"] = json!(format!("2026-09-26T00:00:{index:02}Z"));
            frame.as_object_mut().unwrap().remove("id");
            frame["id"] = json!(crate::framework_checks::digest(&frame)?);
            store.put("validationRun", &id, &next_run)?;
            store.put("objectPreviewFrames", &id, &json!([frame]))?;
        }
    }
    let recent = followup::frames::list(&f.runtime, "project-1", "publish")?;
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0]["runId"], "preview-run-10");
    assert_eq!(recent[7]["runId"], "preview-run-3");
    let request = attach(&op, &original);
    assert_eq!(followup::create(&f.runtime, &request)?.request, request);
    Ok(())
}

#[test]
fn publication_frame_scope_and_digest_rejection_leave_the_plan_unchanged() -> Result<()> {
    let f = fixture()?;
    let op = publication::publish(&f.runtime, &ready(&f)?)?;
    let frame = archive(&f, &op)?;
    let request = attach(&op, &frame);
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(
        followup::frames::list(&f.runtime, "project-1", "publish")?,
        vec![frame.clone()]
    );
    for (path, value) in [
        ("/source/target/versionId", json!("foreign")),
        ("/source/target/objectId", json!("foreign")),
        ("/source/target/projectId", json!("foreign")),
        ("/snapshotId", json!("foreign")),
        ("/source/runId", json!("foreign")),
        ("/source/target/sha256", json!("b".repeat(64))),
        ("/selection/prompt", json!("tampered")),
        ("/selection/picks/0/result/hit/nodePath", json!("forged")),
        ("/selection/picks/1/result/nodePaths/0", json!("forged")),
        (
            "/frame/dataUrl",
            json!("data:image/png;base64,dGFtcGVyZWQ="),
        ),
    ] {
        let mut bad = frame.clone();
        *bad.pointer_mut(path).unwrap() = value;
        f.runtime.store().lock().unwrap().put(
            "objectPreviewFrames",
            "preview-run",
            &json!([bad]),
        )?;
        assert!(followup::create(&f.runtime, &request).is_err(), "{path}");
        assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    }
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("objectPreviewFrames", "preview-run", &json!([frame]))?;
    let receipt = followup::create(&f.runtime, &request)?;
    assert_eq!(followup::create(&f.runtime, &request)?, receipt);
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut changed = request.clone();
    changed.preview_frame.as_mut().unwrap().frame_id = "foreign".into();
    assert!(followup::create(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("CONFLICT"));
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, after);
    let mut lease = start_followup(&f, &receipt)?;
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let context = callback::context(&f.runtime, &lease)?;
    assert!(!serde_json::to_string(&context)?.contains("base64"));
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("objectPreviewFrames", "preview-run", &json!([]))?;
    let params = json!({"tool":callback::TOOL,"threadId":"thread","turnId":"turn",
        "arguments":{"operation":"feedbackImage"}});
    assert!(callback::call(&f.runtime, &lease, &params)
        .unwrap_err()
        .to_string()
        .contains("NOT_FOUND"));
    assert_eq!(followup::create(&f.runtime, &request)?, receipt);
    Ok(())
}

#[cfg(windows)]
#[tokio::test]
async fn publication_frame_reopens_and_delivers_original_png_regions_and_camera_over_rpc(
) -> Result<()> {
    use std::sync::{atomic::AtomicBool, Arc};
    let f = fixture()?;
    let op = publication::publish(&f.runtime, &ready(&f)?)?;
    let frame = archive(&f, &op)?;
    let request = attach(&op, &frame);
    let receipt = followup::create(&f.runtime, &request)?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let f = Fixture {
        runtime: crate::project_storage::ProjectStore::open(temp.path(), "project-1")?
            .into_runtime(),
        temp,
    };
    assert_eq!(followup::create(&f.runtime, &request)?, receipt);
    assert_eq!(
        followup::frames::list(&f.runtime, "project-1", "publish")?,
        vec![frame.clone()]
    );
    let lease = start_followup(&f, &receipt)?;
    let root = f.temp.path().join("archive-rpc");
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
    let attempt = attempts(&f, &receipt.medium_task_id)?.remove(0);
    assert_eq!(
        attempt.state,
        object_attempt::State::AwaitingGate,
        "{:?}",
        attempt.error
    );
    let hash = &attempt.output.as_ref().unwrap()["image-delivered.json"];
    let reply: Value = serde_json::from_slice(&std::fs::read(f.runtime.files().blob(hash)?)?)?;
    assert_eq!(
        reply["contentItems"][1]["imageUrl"],
        frame["frame"]["dataUrl"]
    );
    let provenance: Value =
        serde_json::from_str(reply["contentItems"][2]["text"].as_str().unwrap())?;
    assert!(provenance.get("file").is_none());
    assert_eq!(provenance["previewFrame"]["selection"], frame["selection"]);
    assert_eq!(
        provenance["previewFrame"]["selection"]["picks"][1]["result"]["nodePaths"],
        json!(["Box", "Occluded"])
    );
    assert_eq!(
        provenance["previewFrame"]["frame"]["camera"],
        frame["frame"]["camera"]
    );
    assert_eq!(provenance["previewFrame"]["id"], frame["id"]);
    assert!(provenance["previewFrame"]["frame"].get("dataUrl").is_none());
    assert_eq!(
        task_record(&f, &receipt.medium_task_id)?.status,
        "awaitingAcceptance"
    );
    Ok(())
}
