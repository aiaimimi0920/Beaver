use super::flow::{Definition, Reference};
use crate::files::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub id: String,
    pub project_id: String,
    pub revision: u32,
    pub definition: Definition,
    pub updated_at: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub file: String,
    pub sha256: String,
    pub kind: String,
    pub point: String,
    pub start: f64,
    pub end: f64,
    pub references: Vec<Reference>,
    pub state: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodeCase {
    pub name: String,
    pub file: String,
    pub status: String,
    pub assertions: u32,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeReport {
    pub gut_version: String,
    pub directories: Vec<String>,
    pub cases: Vec<CodeCase>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(default)]
    pub report_sha256: String,
    #[serde(default)]
    pub output_sha256: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub release_id: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub managed: bool,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub snapshot: Snapshot,
    pub snapshot_id: String,
    pub flow: Option<Flow>,
    pub baseline_id: Option<String>,
    pub runner_version: String,
    pub engine_version: String,
    pub status: String,
    pub phase: String,
    pub completed_steps: usize,
    pub verdict: String,
    pub error: Option<String>,
    pub log: String,
    pub evidence: Vec<Evidence>,
    pub code: Option<CodeReport>,
    pub judgments: Vec<Value>,
    pub confirmations: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Baseline {
    pub id: String,
    pub project_id: String,
    pub flow_id: String,
    pub signature: String,
    pub run_id: String,
    pub snapshot_id: String,
    pub evidence_ids: Vec<String>,
    pub confirmed_at: String,
    pub source: String,
    pub previous_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: String,
    pub project_id: String,
    pub snapshot: Snapshot,
    pub snapshot_id: String,
    pub preset: String,
    pub visual_required: bool,
    pub policy_version: String,
    pub flow_ids: Vec<String>,
    pub flows: Vec<Flow>,
    pub scope_id: String,
    pub excluded: Vec<Value>,
    pub missing: Vec<String>,
    pub run_ids: Vec<String>,
    pub created_at: String,
    pub exports: Vec<Value>,
}
