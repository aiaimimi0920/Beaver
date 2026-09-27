use super::*;

#[test]
fn object_catalog_commands_require_identity_and_do_not_expose_acceptance() {
    let cases = [
        (
            "object.register",
            json!({"projectId":"p","requestId":"r","name":"Empty"}),
        ),
        (
            "object.updateRegistration",
            json!({"projectId":"p","requestId":"r","objectId":"o","expectedRevision":0,"name":"Edited","components":[],"files":[],"references":[]}),
        ),
        (
            "object.captureVersion",
            json!({"projectId":"p","requestId":"r","objectId":"o","expectedRevision":0}),
        ),
        (
            "object.acceptVersion",
            json!({"projectId":"p","requestId":"r","objectId":"o","versionId":"v","expectedRevision":0}),
        ),
    ];
    for (method, input) in cases {
        assert!(validate(method, &input).is_ok());
        let tool = tools()
            .into_iter()
            .find(|tool| tool["name"] == method)
            .unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], false);
        for key in ["projectId", "requestId"] {
            let mut invalid = input.clone();
            invalid.as_object_mut().unwrap().remove(key);
            assert!(validate(method, &invalid).is_err());
        }
        for key in ["versions", "status", "accepted", "force"] {
            let mut invalid = input.clone();
            invalid[key] = json!(true);
            assert!(validate(method, &invalid).is_err());
            assert!(tool["inputSchema"]["properties"][key].is_null());
        }
        if method != "object.register" {
            for revision in [json!(-1), json!(0.5), json!("1"), Value::Null] {
                let mut invalid = input.clone();
                invalid["expectedRevision"] = revision;
                assert!(validate(method, &invalid).is_err());
            }
        }
    }

    let acceptance = json!({
        "projectId":"p",
        "requestId":"r",
        "objectId":"o",
        "versionId":"v",
        "expectedRevision":0
    });
    for field in ["objectId", "versionId", "expectedRevision"] {
        let mut invalid = acceptance.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(validate("object.acceptVersion", &invalid).is_err());
    }
    for version_id in [json!(1), Value::Null, json!([])] {
        let mut invalid = acceptance.clone();
        invalid["versionId"] = version_id;
        assert!(validate("object.acceptVersion", &invalid).is_err());
    }
    let tool = tools()
        .into_iter()
        .find(|tool| tool["name"] == "object.acceptVersion")
        .unwrap();
    assert_eq!(
        tool["inputSchema"]["properties"]["versionId"]["type"],
        "string"
    );
    assert_eq!(
        tool["inputSchema"]["required"],
        json!([
            "projectId",
            "requestId",
            "objectId",
            "versionId",
            "expectedRevision"
        ])
    );
}
#[test]
fn object_catalog_schema_keeps_ownership_and_pinned_references_distinct_from_tasks() {
    let catalog = tools();
    for method in ["object.register", "object.updateRegistration"] {
        let tool = catalog.iter().find(|tool| tool["name"] == method).unwrap();
        let properties = &tool["inputSchema"]["properties"];
        assert_eq!(
            properties["components"]["items"]["required"],
            json!(["id", "kind", "name"])
        );
        assert_eq!(
            properties["files"]["items"]["required"],
            json!(["path", "role"])
        );
        let reference = &properties["references"]["items"];
        assert_eq!(
            reference["required"],
            json!(["projectId", "objectId", "versionId"])
        );
        assert_eq!(reference["properties"]["versionId"]["type"], "string");
        assert_eq!(reference["additionalProperties"], false);
        assert!(reference["properties"]["path"].is_null());
        assert_eq!(properties["category"]["type"], "string");
        assert_eq!(properties["tags"]["type"], "array");
        assert_eq!(properties["tags"]["items"]["type"], "string");
        assert_eq!(
            properties["thumbnailPath"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            properties["parentObjectId"]["type"],
            json!(["string", "null"])
        );
    }
    let task = catalog
        .iter()
        .find(|tool| tool["name"] == "task.create")
        .unwrap();
    assert_eq!(
        task["inputSchema"]["properties"]["references"]["items"]["required"],
        json!(["path", "note"])
    );
    let mut edit = json!({"projectId":"p","requestId":"r","objectId":"o","expectedRevision":0,"name":"Edited","components":[],"files":[],"references":[]});
    edit.as_object_mut().unwrap().remove("files");
    assert!(validate("object.updateRegistration", &edit).is_err());
}
