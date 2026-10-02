use serde_json::{json, Value};

pub(super) fn schema() -> Value {
    let index = json!({"type":"integer","minimum":0,"maximum":7});
    super::schema(
        json!({
            "sourceAttemptId":super::id(),
            "sourceFrame":super::schema(json!({"runId":super::id(),"frameId":super::id()}), &["runId", "frameId"]),
            "confirmed":{"type":"boolean","enum":[true]},
            "regions":{"type":"array","minItems":1,"maxItems":8,"items":{"oneOf":[
                super::schema(json!({"status":{"type":"string","enum":["matched"]},"sourceRegion":index,"targetRegion":index}), &["status", "sourceRegion", "targetRegion"]),
                super::schema(json!({"status":{"type":"string","enum":["absent"]},"sourceRegion":index,"note":{"type":"string","minLength":1,"maxLength":1000}}), &["status", "sourceRegion", "note"])
            ]}}
        }),
        &["sourceAttemptId", "sourceFrame", "confirmed", "regions"],
    )
}

pub(super) fn final_schema() -> Value {
    let mut value = schema();
    let properties = value["properties"].as_object_mut().unwrap();
    properties.remove("sourceAttemptId");
    let frame = properties.remove("sourceFrame").unwrap();
    properties.insert("targetFrame".into(), frame);
    properties.insert(
        "sourceDigest".into(),
        json!({"type":"string","pattern":"^[0-9a-f]{64}$"}),
    );
    value["required"] = json!(["sourceDigest", "targetFrame", "confirmed", "regions"]);
    value
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn publication_final_relocation_catalog_is_optional_strict_and_explicit() {
        let tools = crate::object_task_catalog::tools();
        let publish = tools
            .iter()
            .find(|tool| tool["name"] == "objectTask.publishCandidate")
            .unwrap();
        let decision = &publish["inputSchema"]["properties"]["feedback"]["items"];
        assert_eq!(
            decision["required"],
            json!(["requestId", "resolution", "note"])
        );
        assert_eq!(decision["additionalProperties"], false);
        let final_mapping = &decision["properties"]["finalRelocation"];
        assert_eq!(
            final_mapping["required"],
            json!(["sourceDigest", "targetFrame", "confirmed", "regions"])
        );
        assert_eq!(final_mapping["additionalProperties"], false);
        let properties = &final_mapping["properties"];
        assert_eq!(properties["confirmed"]["enum"], json!([true]));
        assert_eq!(properties["sourceDigest"]["pattern"], "^[0-9a-f]{64}$");
        assert!(properties.get("sourceAttemptId").is_none());
        assert!(properties.get("sourceFrame").is_none());
        assert_eq!(
            properties["targetFrame"]["required"],
            json!(["runId", "frameId"])
        );
        assert_eq!(properties["targetFrame"]["additionalProperties"], false);
        assert_eq!(properties["regions"]["minItems"], 1);
        assert_eq!(properties["regions"]["maxItems"], 8);
        for region in properties["regions"]["items"]["oneOf"].as_array().unwrap() {
            assert_eq!(region["additionalProperties"], false);
            assert_eq!(region["properties"]["sourceRegion"]["maximum"], 7);
        }
        let frames = tools
            .iter()
            .find(|tool| tool["name"] == "objectTask.publicationFrames")
            .unwrap();
        let input = &frames["inputSchema"];
        assert_eq!(
            input["required"],
            json!(["projectId", "publicationRequestId"])
        );
        assert_eq!(
            input["properties"]["previewFrame"]["required"],
            json!(["runId", "frameId"])
        );
        assert_eq!(
            input["properties"]["previewFrame"]["additionalProperties"],
            false
        );
        assert_eq!(frames["annotations"]["readOnlyHint"], true);
    }
}
