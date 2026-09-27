use super::{
    preview_frames,
    service::{State, Storage},
    test_support::Fixture,
};
use crate::store::Store;
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Cursor,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

#[test]
fn preview_selection_archive_survives_reopen_and_rejects_wrong_frames() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut run = fixture.run(None)?;
    run.kind = "objectPreview".into();
    run.status = "completed".into();
    fixture.save(&run)?;
    let original_run = serde_json::to_value(&run)?;
    let source = json!({"runId":run.id,"target":{"projectId":"p","path":"game.gd","sha256":run.snapshot["game.gd"]}});
    fixture
        .store
        .put("objectScenePreviewSource", &run.id, &source)?;
    let data = fixture._temp.path().join("data");
    let store = Arc::new(Mutex::new(fixture.store));
    let files = Arc::new(fixture.files);
    let storage = Storage {
        store: store.clone(),
        files: files.clone(),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    let state = State {
        store: store.clone(),
        files,
        storage: Arc::new(move |_| Ok(storage.clone())),
        enumerate: Arc::new(|| Ok(vec![])),
        resolve: Arc::new(|_| anyhow::bail!("engine must not start")),
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    let mut bytes = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(64, 64, image::Rgb([32, 64, 96]))
        .write_to(&mut bytes, image::ImageFormat::Png)?;
    let bytes = bytes.into_inner();
    let frame = json!({"frozen":true,"sequence":2,"sha256":format!("{:x}",Sha256::digest(&bytes)),
        "width":64,"height":64,"dataUrl":format!("data:image/png;base64,{}",STANDARD.encode(bytes))});
    let selection = json!({"kind":"image-regions","sequence":2,"sha256":frame["sha256"],
        "regions":[{"x":0.1,"y":0.2,"width":0.3,"height":0.4,"prompt":"move this"}],
        "prompt":"keep the rest","coordinateSpace":"normalized-image","hitCapability":"unavailable"});
    let input = json!({"projectId":"p","runId":run.id,"sessionId":"s","revision":1,
        "requestId":"capture-regions","selection":selection});
    let method = "validation.preview.capture";
    let saved = preview_frames::call(&state, method, &input, Some(frame.clone()))?.unwrap();
    assert_eq!(saved["selection"], selection);
    assert_eq!(saved["frame"], frame);
    assert_eq!(saved["source"], source);
    *store.lock().unwrap() = Store::open(&data)?;
    assert_eq!(
        preview_frames::call(&state, method, &input, None)?,
        Some(saved.clone())
    );
    let read = preview_frames::call(&state, "validation.preview.saved", &input, None)?.unwrap();
    assert_eq!(read["frames"], json!([saved.clone()]));
    assert_eq!(
        store
            .lock()
            .unwrap()
            .get::<Value>("validationRun", &run.id)?
            .unwrap(),
        original_run
    );
    for (index, invalid) in [
        {
            let mut v = selection.clone();
            v["sequence"] = json!(3);
            v
        },
        {
            let mut v = selection.clone();
            v["sha256"] = json!("b".repeat(64));
            v
        },
        {
            let mut v = selection.clone();
            v["regions"][0]["width"] = json!(1);
            v
        },
        {
            let mut v = selection.clone();
            v["hitCapability"] = json!("mesh");
            v
        },
        {
            let mut v = selection.clone();
            v["regions"][0]["prompt"] = json!("界".repeat(334));
            v
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut bad = input.clone();
        bad["selection"] = invalid;
        bad["requestId"] = json!(format!("bad-{index}"));
        assert!(preview_frames::call(&state, method, &bad, Some(frame.clone())).is_err());
    }
    let mut unfrozen = frame.clone();
    unfrozen["frozen"] = json!(false);
    let mut next = input.clone();
    next["requestId"] = json!("unfrozen");
    assert!(preview_frames::call(&state, method, &next, Some(unfrozen)).is_err());
    assert_eq!(
        preview_frames::call(&state, "validation.preview.saved", &input, None)?.unwrap()["frames"],
        json!([saved.clone()])
    );
    let mut tampered = saved;
    tampered["selection"]["prompt"] = json!("changed");
    store
        .lock()
        .unwrap()
        .put("objectPreviewFrames", &run.id, &json!([tampered]))?;
    assert!(
        preview_frames::call(&state, "validation.preview.saved", &input, None)
            .unwrap_err()
            .to_string()
            .contains("PREVIEW_SAVED_DIGEST_MISMATCH")
    );
    Ok(())
}
