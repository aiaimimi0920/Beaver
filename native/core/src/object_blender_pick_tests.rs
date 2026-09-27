use super::*;

pub(super) fn exercise(service: &Service, read: &Value, frame: &Value) -> Result<Value> {
    assert_eq!(frame["picking"]["skipped"], 1);
    let mut request = json!({"projectId":read["projectId"],"sessionId":read["sessionId"],
        "revision":frame["revision"],"sequence":frame["sequence"],"sha256":frame["sha256"],
        "requestId":"center","point":{"x":0.5,"y":0.5}});
    let wait = |input: &Value| -> Result<Value> {
        let start = Instant::now();
        loop {
            let response = service.preview("validation.preview.pick", input)?;
            if response["status"] == "ready" {
                return Ok(response["result"].clone());
            }
            ensure!(
                start.elapsed() < Duration::from_secs(20),
                "Blender pick timeout"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    let center = wait(&request)?;
    assert_eq!(center["hit"]["nodePath"], "/objects/Cube");
    assert!((center["hit"]["position"][2].as_f64().unwrap() - 1.0).abs() < 0.001);
    assert!((center["hit"]["normal"][2].as_f64().unwrap() - 1.0).abs() < 0.001);
    assert_eq!(wait(&request)?, center);
    let mut conflict = request.clone();
    conflict["sequence"] = json!(0);
    assert!(service
        .preview("validation.preview.pick", &conflict)
        .is_err());
    conflict = request.clone();
    conflict["point"]["x"] = json!(0.4);
    assert!(service
        .preview("validation.preview.pick", &conflict)
        .unwrap_err()
        .to_string()
        .contains("CONFLICT"));
    request["requestId"] = json!("empty");
    request["point"] = json!({"x":0.01,"y":0.01});
    assert!(wait(&request)?["hit"].is_null());
    request["requestId"] = json!("box-center");
    request["point"] = json!({"x":0.5,"y":0.5});
    request["rectangle"] = json!({"x":0.49,"y":0.49,"width":0.02,"height":0.02});
    let rectangle = wait(&request)?;
    assert_eq!(
        rectangle["nodePaths"],
        json!(["/objects/Cube", "/objects/Occluded"])
    );
    assert_eq!(rectangle["truncated"], false);
    request["requestId"] = json!("box-all");
    request["rectangle"] = json!({"x":0,"y":0,"width":1,"height":1});
    let all = wait(&request)?;
    assert_eq!(all["nodePaths"].as_array().unwrap().len(), 32);
    assert_eq!(all["truncated"], true);
    for path in all["nodePaths"].as_array().unwrap() {
        assert!(![
            "Outside",
            "BehindCamera",
            "BeyondFar",
            "UnsupportedModifier"
        ]
        .iter()
        .any(|name| path.as_str().unwrap().ends_with(name)));
    }
    request["requestId"] = json!("box-empty");
    request["point"] = json!({"x":0.01,"y":0.01});
    request["rectangle"] = json!({"x":0,"y":0,"width":0.02,"height":0.02});
    assert_eq!(wait(&request)?["nodePaths"], json!([]));
    let selection = json!({"kind":"image-regions","sequence":frame["sequence"],"sha256":frame["sha256"],
        "regions":[{"x":0.49,"y":0.49,"width":0.02,"height":0.02,"prompt":"Yellow front face"},
            {"x":0.49,"y":0.49,"width":0.02,"height":0.02,"prompt":"Include occluded cube"}],
        "prompt":"Blender frozen static mesh selection","coordinateSpace":"normalized-image",
        "hitCapability":"frozen-static-mesh","picks":[{"region":0,"result":center},{"region":1,"result":rectangle}]});
    let capture = json!({"projectId":read["projectId"],"sessionId":read["sessionId"],
        "revision":frame["revision"],"requestId":"forged","selection":selection});
    let mut forged = capture.clone();
    forged["selection"]["picks"][0]["result"]["hit"]["nodePath"] = json!("/objects/Forged");
    let current = service.preview("validation.preview.read", read)?;
    forged["runId"] = current["runId"].clone();
    assert!(service
        .preview("validation.preview.capture", &forged)
        .unwrap_err()
        .to_string()
        .contains("PREVIEW_SELECTION_PICK_UNTRUSTED"));
    Ok(selection)
}
