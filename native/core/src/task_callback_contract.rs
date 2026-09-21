use crate::asset_task::Stage;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const VERSION: u32 = 5;
pub const INSTRUCTIONS: &str = include_str!("../../../resources/instructions/task-callback.md");

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum Request {
    State,
    Receipt {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    Report {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "expectedRevision")]
        expected_revision: u64,
        report: Report,
    },
    Stages {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "expectedRevision")]
        expected_revision: u64,
        #[serde(rename = "assetRevision")]
        asset_revision: u64,
        stages: Vec<Stage>,
    },
    DeliveryPlan {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "expectedRevision")]
        expected_revision: u64,
        #[serde(rename = "assetRevision")]
        asset_revision: u64,
        template: crate::asset_delivery::Template,
    },
    Work {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "expectedRevision")]
        expected_revision: u64,
        #[serde(rename = "assetRevision")]
        asset_revision: u64,
        #[serde(rename = "stageId")]
        stage_id: String,
        change: crate::asset_work_contract::Action,
    },
    SubmitDelivery {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "expectedRevision")]
        expected_revision: u64,
        #[serde(rename = "assetRevision")]
        asset_revision: u64,
        #[serde(rename = "stageId")]
        stage_id: String,
        #[serde(rename = "inputCandidates")]
        input_candidates: Vec<String>,
        paths: Vec<String>,
        summary: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub kind: String,
    pub summary: String,
    pub inputs: Value,
    pub outputs: Value,
    #[serde(default)]
    pub stage_id: Option<String>,
    #[serde(default)]
    pub asset_revision: Option<u64>,
    #[serde(default)]
    pub tools: Vec<Value>,
}

impl Report {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            ["progress", "checkpoint", "result", "blocked"].contains(&self.kind.as_str()),
            "Invalid report kind"
        );
        ensure!(
            !self.summary.trim().is_empty() && self.summary.len() <= 12000,
            "Report summary must contain 1 to 12000 bytes"
        );
        ensure!(
            self.inputs.is_object() && self.outputs.is_object(),
            "Inputs and outputs must be objects"
        );
        ensure!(
            self.tools.len() <= 100 && self.tools.iter().all(Value::is_object),
            "Tools must contain at most 100 objects"
        );
        ensure!(
            self.stage_id.is_some() == self.asset_revision.is_some(),
            "stageId and assetRevision must be supplied together"
        );
        Ok(())
    }
}

pub fn schema() -> Value {
    let stages = crate::asset_tool::definition()["inputSchema"]["properties"]["stages"].clone();
    json!({"type":"object","additionalProperties":false,"required":["operation"],
    "properties":{
        "operation":{"type":"string","enum":["state","receipt","report","stages","deliveryPlan","work","submitDelivery"]},
        "change":crate::asset_work_contract::schema(),
        "requestId":{"type":"string","minLength":1,"maxLength":100},
        "expectedRevision":{"type":"integer","minimum":0,"description":"Current callback journal revision, not asset revision. Required for mutations."},
        "assetRevision":{"type":"integer","minimum":0},
        "stages":stages,
        "template":{"type":"object","additionalProperties":false,"required":["id","version","stages"],"properties":{
            "id":{"type":"string","maxLength":100},"version":{"type":"integer","minimum":1},
            "stages":{"type":"array","minItems":2,"maxItems":20,"items":{"type":"object","additionalProperties":false,"required":["id","name"],"properties":{"id":{"type":"string","maxLength":100},"name":{"type":"string","maxLength":300}}}}}},
        "stageId":{"type":"string","maxLength":100},
        "inputCandidates":{"type":"array","maxItems":20,"items":{"type":"string"}},
        "paths":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"string","maxLength":2000}},
        "summary":{"type":"string","maxLength":12000},
        "report":{"type":"object","additionalProperties":false,
            "required":["kind","summary","inputs","outputs"],"properties":{
                "kind":{"type":"string","enum":["progress","checkpoint","result","blocked"]},
                "summary":{"type":"string","maxLength":12000},
                "inputs":{"type":"object"},"outputs":{"type":"object"},
                "stageId":{"type":"string","maxLength":100},
                "assetRevision":{"type":"integer","minimum":0},
                "tools":{"type":"array","maxItems":100,"items":{"type":"object"}}
            }}
    }})
}

pub fn definition() -> Value {
    json!({"type":"function","name":"beaver_task",
        "description":"Read task context and receipts, report facts or update stages. deliveryPlan defines an immutable linear template. work registers/revises/cancels/reopens current-stage subtasks or begins/finishes serial attempts. begin requires inputFiles (source editable, dependency immutable, [] for no file inputs). Beaver freezes inputs and checks dependencies at successful finish, submission and approval. submitDelivery freezes outputs and ends this turn for owner review. Never approve your own candidate. Mutations require requestId and expectedRevision; stage/work mutations also require assetRevision.",
        "inputSchema":schema()})
}

pub fn business_tools() -> Vec<Value> {
    vec![
        json!({"name":"task.callback","description":"Owner adapter to the same task-scoped callback used by managed Codex. Requires the currently running task's thread/turn. No scheduling or completion side effects from reports.",
            "inputSchema":{"type":"object","additionalProperties":false,"required":["id","threadId","turnId","request"],"properties":{
                "id":{"type":"string"},"threadId":{"type":"string"},"turnId":{"type":"string"},"request":schema()}},
            "annotations":{"readOnlyHint":false,"destructiveHint":false,"openWorldHint":false}}),
        json!({"name":"task.callbackState","description":"Inspect current task/asset state and the latest 20 callback receipt summaries even after interruption. Supply requestId to read one durable receipt, including original request and response.",
            "inputSchema":{"type":"object","additionalProperties":false,"required":["id"],"properties":{
                "id":{"type":"string"},"requestId":{"type":"string"}}},
            "annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}),
    ]
}
