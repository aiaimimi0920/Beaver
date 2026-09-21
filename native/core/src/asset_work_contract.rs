use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub title: String,
    pub goal: String,
    pub acceptance: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Action {
    Create {
        definition: Definition,
    },
    Revise {
        #[serde(rename = "subtaskId")]
        subtask_id: String,
        definition: Definition,
    },
    Cancel {
        #[serde(rename = "subtaskId")]
        subtask_id: String,
        reason: String,
    },
    Reopen {
        #[serde(rename = "subtaskId")]
        subtask_id: String,
        reason: String,
    },
    Begin {
        #[serde(rename = "subtaskId")]
        subtask_id: String,
        inputs: Value,
        #[serde(rename = "inputFiles")]
        input_files: Vec<crate::asset_work_inputs::Declaration>,
        #[serde(default, rename = "recoveryNote")]
        recovery_note: String,
    },
    Finish {
        #[serde(rename = "attemptId")]
        attempt_id: String,
        outcome: Outcome,
        summary: String,
        outputs: Value,
        #[serde(default)]
        tools: Vec<Value>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Completed,
    Failed,
}

pub fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["action"],"properties":{
        "action":{"type":"string","enum":["create","revise","cancel","reopen","begin","finish"]},
        "definition":{"type":"object","additionalProperties":false,"required":["title","goal","acceptance"],"properties":{
            "title":{"type":"string","minLength":1,"maxLength":300},
            "goal":{"type":"string","minLength":1,"maxLength":12000},
            "acceptance":{"type":"string","minLength":1,"maxLength":12000}}},
        "subtaskId":{"type":"string","maxLength":100},
        "attemptId":{"type":"string","maxLength":100},
        "reason":{"type":"string","maxLength":12000},
        "inputs":{"type":"object"},"recoveryNote":{"type":"string","maxLength":12000},
        "inputFiles":{"type":"array","maxItems":32,"description":"Required for begin; use [] only when there are no file inputs. Paths must be workspace-relative. Source files may be edited; dependencies must stay unchanged.","items":{
            "type":"object","additionalProperties":false,"required":["path","role"],"properties":{
                "path":{"type":"string","maxLength":2000},
                "role":{"type":"string","enum":["source","dependency"]},
                "expectedSha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}},
        "outcome":{"type":"string","enum":["completed","failed"]},
        "summary":{"type":"string","maxLength":12000},"outputs":{"type":"object"},
        "tools":{"type":"array","maxItems":100,"items":{"type":"object"}}
    }})
}
