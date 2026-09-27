use super::service::Service;
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[test]
fn pick_receipts_accept_engine_numeric_round_trip_and_reject_foreign_frame() -> Result<()> {
    use super::{
        live_preview::{Camera, View},
        live_preview_pick::{receive, request},
    };
    let output = tempfile::tempdir()?;
    let mut view = View {
        touched: Instant::now(),
        revision: 1,
        frozen: true,
        camera: Camera::default(),
        width: 960,
        height: 540,
        status: "ready".into(),
        error: None,
        pick_request: Value::Null,
        frame: json!({"sessionId":"s","revision":1,"sequence":2,
            "frozen":true,"sha256":"a".repeat(64),"picking":{"capability":"frozen-static-mesh-ray","triangles":12,"skipped":0}}),
    };
    let input = json!({"requestId":"edge","revision":1,"sequence":2,"sha256":"a".repeat(64),"point":{"x":0,"y":1}});
    assert_eq!(request(&mut view, &input)?["status"], "pending");
    let mut receipt = view.pick_request.clone();
    receipt["point"] = json!({"x":0.0,"y":1.0});
    receipt["capability"] = json!("frozen-static-mesh-ray");
    receipt["triangles"] = json!(12);
    receipt["skipped"] = json!(0);
    receipt["hit"] = Value::Null;
    let mut foreign = receipt.clone();
    foreign["sequence"] = json!(3);
    std::fs::write(
        output.path().join("pick.json"),
        serde_json::to_vec(&foreign)?,
    )?;
    receive(output.path(), &mut view)?;
    assert!(view.frame["picks"].is_null());
    std::fs::write(
        output.path().join("pick.json"),
        serde_json::to_vec(&receipt)?,
    )?;
    receive(output.path(), &mut view)?;
    receive(output.path(), &mut view)?;
    assert_eq!(view.frame["picks"].as_array().unwrap().len(), 1);
    assert_eq!(request(&mut view, &input)?["result"], receipt);
    Ok(())
}

pub(super) fn exercise(service: &Service, read: &Value, frame: &Value) -> Result<Value> {
    let mut request = json!({"projectId":read["projectId"],"sessionId":read["sessionId"],
        "revision":frame["revision"],"sequence":frame["sequence"],"sha256":frame["sha256"],
        "requestId":"center-pick","point":{"x":0.5,"y":0.5}});
    let wait = |request: &Value| -> Result<Value> {
        let start = Instant::now();
        loop {
            let response = service.preview("validation.preview.pick", request)?;
            if response["status"] == "ready" {
                return Ok(response["result"].clone());
            }
            ensure!(start.elapsed() < Duration::from_secs(20), "Pick timed out");
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    let center = wait(&request)?;
    assert_eq!(center["hit"]["nodePath"], "Box");
    assert!((center["hit"]["position"][2].as_f64().unwrap() - 0.5).abs() < 0.001);
    assert!((center["hit"]["normal"][2].as_f64().unwrap() - 1.0).abs() < 0.001);
    assert_eq!(wait(&request)?, center);
    let mut conflict = request.clone();
    conflict["point"]["x"] = json!(0.4);
    assert!(service
        .preview("validation.preview.pick", &conflict)
        .unwrap_err()
        .to_string()
        .contains("PREVIEW_PICK_REQUEST_CONFLICT"));
    conflict = request.clone();
    conflict["sequence"] = json!(0);
    assert!(service
        .preview("validation.preview.pick", &conflict)
        .is_err());
    request["requestId"] = json!("corner-pick");
    request["point"] = json!({"x":0.01,"y":0.01});
    assert_eq!(wait(&request)?["hit"], Value::Null);
    request["requestId"] = json!("box-center");
    request["point"] = json!({"x":0.5,"y":0.5});
    request["rectangle"] = json!({"x":0.49,"y":0.49,"width":0.02,"height":0.02});
    let box_pick = wait(&request)?;
    assert_eq!(box_pick["capability"], "frozen-static-mesh-frustum");
    assert_eq!(box_pick["nodePaths"], json!(["Box", "Occluded"]));
    assert_eq!(box_pick["truncated"], false);
    assert_eq!(wait(&request)?, box_pick);
    let mut wrong_box = request.clone();
    wrong_box["rectangle"]["width"] = json!(0.03);
    assert!(service
        .preview("validation.preview.pick", &wrong_box)
        .unwrap_err()
        .to_string()
        .contains("CONFLICT"));
    request["requestId"] = json!("box-all");
    request["rectangle"] = json!({"x":0,"y":0,"width":1,"height":1});
    let all = wait(&request)?;
    assert_eq!(all["nodePaths"].as_array().unwrap().len(), 32);
    assert_eq!(all["truncated"], true);
    assert!(!all["nodePaths"].as_array().unwrap().iter().any(|p| [
        "Outside",
        "BehindCamera",
        "BeyondFar"
    ]
    .iter()
    .any(|name| p == name)));
    request["requestId"] = json!("box-empty");
    request["point"] = json!({"x":0.01,"y":0.01});
    request["rectangle"] = json!({"x":0,"y":0,"width":0.02,"height":0.02});
    assert_eq!(wait(&request)?["nodePaths"], json!([]));
    let trusted = service.preview("validation.preview.read", read)?["frame"].clone();
    let selection = json!({"kind":"image-regions","sequence":frame["sequence"],"sha256":frame["sha256"],
        "regions":[{"x":0.49,"y":0.49,"width":0.02,"height":0.02,"prompt":"make this yellow"},
            {"x":0.49,"y":0.49,"width":0.02,"height":0.02,"prompt":"include occluded mesh"}],
        "prompt":"keep the background","coordinateSpace":"normalized-image","hitCapability":"frozen-static-mesh",
        "picks":[{"region":0,"result":center},{"region":1,"result":box_pick}]});
    super::preview_selection::validate(&selection, &trusted)?;
    let mut forged = selection.clone();
    forged["picks"][0]["result"]["hit"]["nodePath"] = json!("AnotherObject");
    assert!(super::preview_selection::validate(&forged, &trusted)
        .unwrap_err()
        .to_string()
        .contains("PREVIEW_SELECTION_PICK_UNTRUSTED"));
    forged = selection.clone();
    forged["regions"][1]["width"] = json!(0.03);
    assert!(super::preview_selection::validate(&forged, &trusted)
        .unwrap_err()
        .to_string()
        .contains("OUTSIDE"));
    forged = selection.clone();
    forged["picks"][1]["result"]["nodePaths"] = json!(["AnotherObject"]);
    assert!(super::preview_selection::validate(&forged, &trusted)
        .unwrap_err()
        .to_string()
        .contains("UNTRUSTED"));
    Ok(selection)
}
