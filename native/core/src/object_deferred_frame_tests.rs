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

pub(super) fn final_frame(f: &Fixture, publish: &Request) -> Result<Value> {
    let preview = publication::preview(&f.runtime, &review(publish))?;
    let hash = preview
        .paths
        .iter()
        .find(|p| p.path == "preview.png")
        .and_then(|p| p.after.clone())
        .unwrap();
    let mut target = input(publish);
    target.image = Some(
        crate::object_run_recovery::resume::rework::image::Feedback {
            path: "preview.png".into(),
            sha256: hash,
            width: 20,
            height: 10,
            regions: vec![crate::object_run_recovery::resume::rework::image::Region {
                x: 0.1,
                y: 0.2,
                width: 0.5,
                height: 0.5,
                prompt: "Final position".into(),
            }],
        },
    );
    attach_named(f, &mut target, "final-frame")
}

pub(crate) fn approve(f: &Fixture, publish: &mut Request) -> Result<()> {
    let preview = publication::preview(&f.runtime, &review(publish))?;
    let target = if preview
        .feedback
        .iter()
        .any(|item| item.relocation_requirement.is_some())
    {
        let mut frames =
            deferred::frames::list(&f.runtime, "project-1", &publish.target.attempt_id, None)?;
        Some(if frames.is_empty() {
            final_frame(f, publish)?
        } else {
            frames.remove(0)
        })
    } else {
        None
    };
    publish.preview_digest = preview.digest;
    publish.feedback = preview.feedback.iter().map(|item| FeedbackDecision {
        request_id: item.request_id.clone(),
        resolution: if item.later.is_some() { Resolution::Deferred } else { Resolution::Resolved },
        note: "Reviewed exact final output".into(),
        final_relocation: item.relocation_requirement.as_ref().map(|required| publication::relocation::Confirmation {
            source_digest: required.source_digest.clone(),
            target_frame: followup::frames::Reference {
                run_id: target.as_ref().unwrap()["runId"].as_str().unwrap().into(),
                frame_id: target.as_ref().unwrap()["id"].as_str().unwrap().into(),
            },
            regions: (0..required.region_count).map(|index| if index == 0 {
                crate::object_run_recovery::resume::rework::relocation::Region::Matched { source_region: index, target_region: 0 }
            } else {
                crate::object_run_recovery::resume::rework::relocation::Region::Absent { source_region: index, note: "Removed from final output".into() }
            }).collect(),
            confirmed: true,
        }),
    }).collect();
    Ok(())
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
