use super::*;
use crate::object_catalog_test_fixture::Fixture;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::io::Cursor;

pub(super) fn archive(f: &Fixture, op: &publication::Operation) -> Result<Value> {
    let mut png = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(64, 64, image::Rgb([20, 40, 60]))
        .write_to(&mut png, image::ImageFormat::Png)?;
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(png.get_ref()));
    let source_hash = "a".repeat(64);
    let pick = json!({"requestId":"pick-1","sessionId":"viewer","revision":3,"sequence":2,
        "sha256":hash,"point":{"x":0.2,"y":0.3},"capability":"frozen-static-mesh-ray",
        "triangles":12,"skipped":0,"hit":{"nodePath":"Box","triangle":0,
            "position":[0,0,0.5],"normal":[0,0,1],"distance":4.0}});
    let mut item = json!({
        "projectId":"project-1","runId":"preview-run","snapshotId":"preview-snapshot",
        "savedAt":"2026-09-26T00:00:00Z",
        "source":{"runId":"preview-run","sourceDigest":"source","projectConfig":"project.godot",
            "target":{"projectId":"project-1","objectId":op.request.target.object_id,
                "versionId":op.version_id,"path":"scene.tscn","sha256":source_hash}},
        "frame":{"sessionId":"viewer","sequence":2,"frozen":true,"revision":3,"width":64,"height":64,
            "picking":{"capability":"frozen-static-mesh-ray","triangles":12,"skipped":0},"picks":[pick.clone()],
            "sha256":hash,"dataUrl":format!("data:image/png;base64,{}",STANDARD.encode(png.into_inner())),
            "camera":{"yaw":0.5,"pitch":0.3,"distance":4}},
        "selection":{"kind":"image-regions","sequence":2,"sha256":hash,
            "regions":[{"x":0.1,"y":0.2,"width":0.3,"height":0.4,"prompt":"move this"}],
            "prompt":"keep the rest","coordinateSpace":"normalized-image","hitCapability":"frozen-static-mesh-ray",
            "picks":[{"region":0,"result":pick}]}
    });
    let box_pick = json!({"requestId":"box-1","sessionId":"viewer","revision":3,"sequence":2,
        "sha256":hash,"point":{"x":0.2,"y":0.3},"capability":"frozen-static-mesh-frustum",
        "rectangle":{"x":0.1,"y":0.2,"width":0.3,"height":0.4},
        "triangles":12,"skipped":0,"hit":null,"nodePaths":["Box","Occluded"],"truncated":false});
    item["frame"]["picks"]
        .as_array_mut()
        .unwrap()
        .push(box_pick.clone());
    item["selection"]["regions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"x":0.1,"y":0.2,"width":0.3,"height":0.4,"prompt":"move both"}));
    item["selection"]["picks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"region":1,"result":box_pick}));
    item["selection"]["hitCapability"] = json!("frozen-static-mesh");
    item["id"] = json!(crate::framework_checks::digest(&item)?);
    let store = f.runtime.store();
    let store = store.lock().unwrap();
    store.put(
        "validationRun",
        "preview-run",
        &json!({"id":"preview-run","projectId":"project-1",
        "kind":"objectPreview","status":"completed","snapshotId":"preview-snapshot",
        "snapshot":{"scene.tscn":source_hash}}),
    )?;
    store.put("objectPreviewFrames", "preview-run", &json!([item.clone()]))?;
    Ok(item)
}

pub(super) fn attach(op: &publication::Operation, frame: &Value) -> Request {
    let mut request = input(op);
    request.preview_frame = Some(followup::frames::Reference {
        run_id: frame["runId"].as_str().unwrap().into(),
        frame_id: frame["id"].as_str().unwrap().into(),
    });
    request
}

pub(super) fn start_followup(
    f: &Fixture,
    receipt: &followup::Receipt,
) -> Result<crate::object_attempt::Lease> {
    object_tasks::cancel_planned(
        &f.runtime,
        &CancelPlannedRequest {
            project_id: "project-1".into(),
            task_id: "next".into(),
            request_id: "cancel-next".into(),
            expected_task_revision: task_record(f, "next")?.revision,
            expected_plan_revision: object_tasks::snapshot(&f.runtime, "project-1")?.plan_revision,
        },
    )?;
    object_tasks::enqueue(&f.runtime, "project-1", &[receipt.medium_task_id.clone()])?;
    let claim = crate::object_run_preparation::claim_next(&f.runtime, "project-1", "frame-worker")?
        .unwrap();
    Ok(crate::object_attempt::start(&f.runtime, claim)?.unwrap())
}
