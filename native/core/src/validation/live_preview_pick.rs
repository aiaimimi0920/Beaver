use super::live_preview::View;
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, path::Path, time::Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rectangle {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
impl Rectangle {
    fn valid(&self, p: &Point) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width > 0.0
            && self.height > 0.0
            && self.x + self.width <= 1.0
            && self.y + self.height <= 1.0
            && p.x >= self.x
            && p.x <= self.x + self.width
            && p.y >= self.y
            && p.y <= self.y + self.height
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Hit {
    node_path: String,
    triangle: u64,
    position: [f64; 3],
    normal: [f64; 3],
    distance: f64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    request_id: String,
    session_id: String,
    revision: u64,
    sequence: u64,
    sha256: String,
    point: Point,
    capability: String,
    hit: Option<Hit>,
    skipped: u64,
    triangles: u64,
    rectangle: Option<Rectangle>,
    node_paths: Option<Vec<String>>,
    truncated: Option<bool>,
}

fn point_valid(p: &Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y)
}

pub(super) fn validate(receipt: &Value, frame: &Value) -> Result<()> {
    let r: Receipt = serde_json::from_value(receipt.clone()).context("PREVIEW_PICK_INVALID")?;
    ensure!(
        frame["frozen"] == true
            && frame["sessionId"] == r.session_id
            && frame["revision"] == r.revision
            && frame["sequence"] == r.sequence
            && frame["sha256"] == r.sha256,
        "PREVIEW_PICK_FRAME_MISMATCH"
    );
    ensure!(
        !r.request_id.is_empty()
            && r.request_id.len() <= 128
            && point_valid(&r.point)
            && r.triangles <= 100000
            && frame["picking"]["triangles"] == r.triangles
            && frame["picking"]["skipped"] == r.skipped,
        "PREVIEW_PICK_INVALID"
    );
    if let Some(rectangle) = r.rectangle {
        let paths = r.node_paths.context("PREVIEW_PICK_BOX_INVALID")?;
        let unique: std::collections::BTreeSet<_> = paths.iter().collect();
        ensure!(
            r.capability == "frozen-static-mesh-frustum"
                && rectangle.valid(&r.point)
                && r.hit.is_none()
                && r.truncated.is_some()
                && paths.len() <= 32
                && paths.len() == unique.len()
                && (paths.is_empty() || r.triangles > 0)
                && paths.iter().all(|p| !p.is_empty() && p.len() <= 4096)
                && paths.iter().map(|p| p.len()).sum::<usize>() <= 32768,
            "PREVIEW_PICK_BOX_INVALID"
        );
    } else {
        ensure!(
            r.capability == "frozen-static-mesh-ray"
                && r.node_paths.is_none()
                && r.truncated.is_none(),
            "PREVIEW_PICK_INVALID"
        );
    }
    if let Some(h) = r.hit {
        ensure!(
            !h.node_path.is_empty()
                && h.node_path.len() <= 4096
                && h.triangle < r.triangles
                && h.position
                    .iter()
                    .chain(h.normal.iter())
                    .all(|v| v.is_finite())
                && h.distance.is_finite()
                && h.distance >= 0.0,
            "PREVIEW_PICK_HIT_INVALID"
        );
    }
    Ok(())
}

pub(super) fn request(view: &mut View, input: &Value) -> Result<Value> {
    ensure!(
        view.status == "ready"
            && view.frozen
            && view.frame["revision"] == view.revision
            && input["revision"] == view.revision
            && input["sequence"] == view.frame["sequence"]
            && input["sha256"] == view.frame["sha256"],
        "PREVIEW_PICK_FRAME_MISMATCH"
    );
    ensure!(
        view.frame["picking"]["capability"] == "frozen-static-mesh-ray",
        "PREVIEW_PICK_UNAVAILABLE"
    );
    let point: Point =
        serde_json::from_value(input["point"].clone()).context("PREVIEW_PICK_POINT_INVALID")?;
    ensure!(point_valid(&point), "PREVIEW_PICK_POINT_INVALID");
    let id = input["requestId"]
        .as_str()
        .context("PREVIEW_PICK_REQUEST_REQUIRED")?;
    ensure!(
        !id.is_empty() && id.len() <= 128,
        "PREVIEW_PICK_REQUEST_REQUIRED"
    );
    let mut request = json!({"requestId":id,"sessionId":view.frame["sessionId"],
        "revision":view.revision,"sequence":input["sequence"],"sha256":input["sha256"],"point":input["point"]});
    if let Some(value) = input.get("rectangle") {
        let rectangle: Rectangle =
            serde_json::from_value(value.clone()).context("PREVIEW_PICK_BOX_INVALID")?;
        ensure!(rectangle.valid(&point), "PREVIEW_PICK_BOX_INVALID");
        request["rectangle"] = value.clone();
    }
    view.touched = Instant::now();
    let receipts = view.frame["picks"].as_array().cloned().unwrap_or_default();
    if let Some(found) = receipts.iter().find(|r| r["requestId"] == id) {
        ensure!(
            matches_request(found, &request),
            "PREVIEW_PICK_REQUEST_CONFLICT"
        );
        return Ok(json!({"status":"ready","result":found}));
    }
    ensure!(receipts.len() < 32, "PREVIEW_PICK_LIMIT");
    if !view.pick_request.is_null()
        && !receipts
            .iter()
            .any(|r| r["requestId"] == view.pick_request["requestId"])
    {
        ensure!(view.pick_request == request, "PREVIEW_PICK_PENDING");
    }
    view.pick_request = request;
    Ok(json!({"status":"pending"}))
}

fn matches_request(receipt: &Value, request: &Value) -> bool {
    ["requestId", "sessionId", "revision", "sequence", "sha256"]
        .iter()
        .all(|key| receipt[key] == request[key])
        && ["x", "y"]
            .iter()
            .all(|key| receipt["point"][key].as_f64() == request["point"][key].as_f64())
        && receipt["rectangle"].is_null() == request["rectangle"].is_null()
        && ["x", "y", "width", "height"]
            .iter()
            .all(|key| receipt["rectangle"][key].as_f64() == request["rectangle"][key].as_f64())
}

pub(super) fn receive(output: &Path, view: &mut View) -> Result<()> {
    if view.pick_request.is_null() {
        return Ok(());
    }
    let path = output.join("pick.json");
    if !path.exists() {
        return Ok(());
    }
    ensure!(
        fs::metadata(&path)?.len() <= 256 * 1024,
        "PREVIEW_PICK_TOO_LARGE"
    );
    let receipt: Value = serde_json::from_slice(&fs::read(path)?)?;
    if !matches_request(&receipt, &view.pick_request) {
        return Ok(());
    }
    validate(&receipt, &view.frame)?;
    if !view.frame["picks"].is_array() {
        view.frame["picks"] = json!([]);
    }
    let picks = view.frame["picks"].as_array_mut().unwrap();
    if !picks.iter().any(|r| r["requestId"] == receipt["requestId"]) {
        picks.push(receipt);
    }
    Ok(())
}
