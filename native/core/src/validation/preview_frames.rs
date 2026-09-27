//! Durable viewer captures, separate from validation evidence and approvals.
use super::{operations, repository, requests::Request, service::State};
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const KIND: &str = "objectPreviewFrames";
const LIMIT: usize = 8;
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn call(
    state: &State,
    method: &str,
    input: &Value,
    frame: Option<Value>,
) -> Result<Option<Value>> {
    let project = operations::string(input, "projectId")?;
    let storage = (state.storage)(project)?;
    ensure!(!storage.draining, "PROJECT_UNAVAILABLE");
    let _permit = storage
        .work_gate
        .acquire()?
        .context("PROJECT_UNAVAILABLE")?;
    let mut store = storage
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
    let run = operations::owned_run(&store, input)?;
    ensure!(
        run.kind == "objectPreview" && run.status == "completed",
        "PREVIEW_REQUIRES_COMPLETED_CAPTURE"
    );
    let mut saved: Vec<Value> = store.get(KIND, &run.id)?.unwrap_or_default();
    if method == "validation.preview.saved" {
        for item in &saved {
            ensure!(
                item["projectId"] == project
                    && item["runId"] == run.id
                    && item["snapshotId"] == run.snapshot_id,
                "PREVIEW_SAVED_IDENTITY_MISMATCH"
            );
            validate_saved(item)?;
        }
        return Ok(Some(
            json!({"projectId":project,"runId":run.id,"snapshotId":run.snapshot_id,"frames":saved}),
        ));
    }
    let request = Request::new(method, input)?;
    if let Some(result) = request.replay(&store)? {
        return Ok(Some(result));
    }
    let Some(frame) = frame else {
        return Ok(None);
    };
    ensure!(saved.len() < LIMIT, "PREVIEW_SAVED_FRAME_LIMIT");
    validate_frame(&frame)?;
    if let Some(selection) = input.get("selection") {
        super::preview_selection::validate(selection, &frame)?;
    }
    let source = match store.get::<Value>("objectScenePreviewSource", &run.id)? {
        Some(source) => source,
        None => store
            .list::<Value>("objectScenePreview")?
            .into_iter()
            .find(|source| source["runId"] == run.id)
            .context("PREVIEW_SOURCE_MISSING")?,
    };
    ensure!(
        source["runId"] == run.id && source["target"]["projectId"] == project,
        "PREVIEW_SOURCE_MISMATCH"
    );
    let path = operations::string(&source["target"], "path")?;
    ensure!(
        run.snapshot
            .get(path)
            .is_some_and(|hash| source["target"]["sha256"] == *hash),
        "PREVIEW_SOURCE_HASH_MISMATCH"
    );
    let mut item = json!({"projectId":project,"runId":run.id,"snapshotId":run.snapshot_id,
        "source":source,"savedAt":repository::now(),"frame":frame});
    if let Some(selection) = input.get("selection") {
        item["selection"] = selection.clone();
    }
    item["id"] = json!(repository::digest(&item)?);
    saved.push(item.clone());
    ensure!(
        serde_json::to_vec(&saved)?.len() <= MAX_BYTES,
        "PREVIEW_SAVED_STORAGE_LIMIT"
    );
    let result = request.finish(&mut store, item, vec![(KIND, run.id, json!(saved))])?;
    Ok(Some(result))
}

pub(crate) fn validate_saved(item: &Value) -> Result<()> {
    validate_frame(&item["frame"])?;
    if let Some(selection) = item.get("selection") {
        super::preview_selection::validate(selection, &item["frame"])?;
    }
    let mut unsigned = item.clone();
    let id = unsigned
        .as_object_mut()
        .context("PREVIEW_SAVED_INVALID")?
        .remove("id")
        .context("PREVIEW_SAVED_ID_MISSING")?;
    ensure!(
        id == repository::digest(&unsigned)?,
        "PREVIEW_SAVED_DIGEST_MISMATCH"
    );
    Ok(())
}

fn validate_frame(frame: &Value) -> Result<()> {
    let data = frame["dataUrl"]
        .as_str()
        .and_then(|s| s.strip_prefix("data:image/png;base64,"))
        .context("PREVIEW_FRAME_PNG_REQUIRED")?;
    ensure!(data.len() <= MAX_BYTES, "PREVIEW_FRAME_TOO_LARGE");
    let bytes = STANDARD.decode(data)?;
    ensure!(
        frame["sha256"] == format!("{:x}", Sha256::digest(&bytes)),
        "PREVIEW_FRAME_HASH_MISMATCH"
    );
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    ensure!(
        frame["width"] == image.width() && frame["height"] == image.height(),
        "PREVIEW_FRAME_DIMENSIONS"
    );
    Ok(())
}
