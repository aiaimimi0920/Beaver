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
