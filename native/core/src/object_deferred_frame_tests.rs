use super::*;
use crate::object_catalog_test_fixture::Fixture;
use serde_json::{json, Value};

pub(super) fn attach(f: &Fixture, request: &mut deferred::Request) -> Result<Value> {
    attach_named(f, request, "attempt-preview")
}

pub(super) fn attach_named(
    f: &Fixture,
    request: &mut deferred::Request,
    archive: &str,
) -> Result<Value> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    let image = request.image.take().unwrap();
    let bytes = std::fs::read(f.runtime.files().blob(&image.sha256)?)?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let source = f.runtime.store().lock().unwrap().transaction(|db| {
        crate::object_attempt_view::read(db, "project-1", &request.review.target.attempt_id)
    })?;
    let store = f.runtime.store();
    let store = store.lock().unwrap();
    let output = source.output.as_ref().unwrap();
    let mut item = json!({
        "projectId":"project-1","runId":archive,"snapshotId":"attempt-snapshot",
        "savedAt":"2026-09-26T00:00:00Z",
        "source":{"runId":archive,"sourceDigest":crate::framework_checks::digest(output)?,"projectConfig":"project.godot",
          "target":{"projectId":"project-1","runId":source.preparation.run.id,"attemptId":source.id,
            "checkpoint":"output","path":image.path,"sha256":image.sha256}},
        "frame":{"sequence":2,"frozen":true,"revision":3,"width":20,"height":10,
          "sha256":hash,"dataUrl":format!("data:image/png;base64,{}",STANDARD.encode(bytes)),"camera":{"yaw":0.5}},
        "selection":{"kind":"image-regions","sequence":2,"sha256":hash,"regions":image.regions,
          "prompt":"keep the rest","coordinateSpace":"normalized-image","hitCapability":"unavailable"}
    });
    item["id"] = json!(crate::framework_checks::digest(&item)?);
    store.put("validationRun", archive, &json!({"id":archive,"projectId":"project-1",
        "kind":"objectPreview","status":"completed","snapshotId":"attempt-snapshot","snapshot":output}))?;
    store.put("objectPreviewFrames", archive, &json!([item.clone()]))?;
    request.preview_frame = Some(followup::frames::Reference {
        run_id: archive.into(),
        frame_id: item["id"].as_str().unwrap().into(),
    });
    Ok(item)
}

#[test]
fn publication_deferred_frames_validate_binding_and_retry_without_mutation() -> Result<()> {
    let f = fixture()?;
    let (_, mut request) = image_feedback(&f)?;
    let image = request.image.clone();
    let item = attach(&f, &mut request)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut bad = request.clone();
    bad.image = image;
    assert!(deferred::create(&f.runtime, &bad)
        .unwrap_err()
        .to_string()
        .contains("CONFLICT"));
    for field in ["runId", "attemptId", "checkpoint", "sha256"] {
        let mut tampered = item.clone();
        tampered["source"]["target"][field] = json!("foreign");
        tampered.as_object_mut().unwrap().remove("id");
        tampered["id"] = json!(crate::framework_checks::digest(&tampered)?);
        f.runtime.store().lock().unwrap().put(
            "objectPreviewFrames",
            "attempt-preview",
            &json!([tampered.clone()]),
        )?;
        let mut bad = request.clone();
        bad.preview_frame.as_mut().unwrap().frame_id = tampered["id"].as_str().unwrap().into();
        assert!(deferred::create(&f.runtime, &bad).is_err(), "{field}");
        assert!(publication::preview(&f.runtime, &request.review)?
            .feedback
            .is_empty());
    }
    f.runtime.store().lock().unwrap().put(
        "objectPreviewFrames",
        "attempt-preview",
        &json!([item.clone()]),
    )?;
    assert_eq!(
        deferred::frames::list(
            &f.runtime,
            "project-1",
            &request.review.target.attempt_id,
            request.preview_frame.as_ref()
        )?,
        vec![item]
    );
    assert_eq!(deferred::create(&f.runtime, &request)?, request);
    assert_eq!(deferred::create(&f.runtime, &request)?, request);
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert_eq!(
        publication::preview(&f.runtime, &request.review)?.feedback[0].preview_frame,
        request.preview_frame
    );
    Ok(())
}
