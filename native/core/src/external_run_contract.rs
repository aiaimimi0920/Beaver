//! Identity is supplied by the host-owned registry, never by caller JSON.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunRequest {
    pub task_id: String,
    pub run_id: String,
    pub revision: u64,
    pub request_id: String,
    pub arguments: Value,
}

impl RunRequest {
    pub(crate) fn validate(&self) -> Result<()> {
        for value in [&self.task_id, &self.run_id, &self.request_id] {
            ensure!(
                !value.is_empty() && value.len() <= 160,
                "Invalid external request identity"
            );
        }
        ensure!(
            serde_json::to_vec(&self.arguments)?.len() <= 2_000_000,
            "External arguments too large"
        );
        Ok(())
    }
    pub(crate) fn fingerprint(&self, method: &str) -> Value {
        serde_json::json!({"method":method,"revision":self.revision,"arguments":self.arguments})
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolRequest {
    pub tool: String,
    pub arguments: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinishRequest {
    pub outcome: String,
    pub error: Option<String>,
}

pub const MODE: &str = "external-agent";

pub fn enabled(task: &Value) -> bool {
    task["executionMode"] == MODE
}
