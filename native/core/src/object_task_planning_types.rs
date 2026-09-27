use crate::object_task_types::{Draft, PlanProposal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRequest {
    pub project_id: String,
    pub request_id: String,
    pub draft_id: String,
    pub expected_draft_revision: u64,
    pub expected_plan_revision: u64,
    pub goal: String,
    pub acceptance: String,
    pub ask_ratio: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionRequest {
    pub project_id: String,
    pub session_id: String,
    pub request_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerRequest {
    pub project_id: String,
    pub session_id: String,
    pub request_id: String,
    pub expected_revision: u64,
    pub answers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Running,
    AwaitingInput,
    Proposed,
    Adopted,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub question: Value,
    pub answer: String,
    pub source: crate::object_task_types::AssumptionSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub revision: u64,
    pub status: Status,
    pub input: StartRequest,
    pub round_id: String,
    pub round: u32,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub questions: Vec<Value>,
    pub decisions: Vec<Decision>,
    pub proposal: Option<PlanProposal>,
    pub conflict: Option<String>,
    pub error: Option<String>,
    pub adopted_draft: Option<Draft>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Record {
    pub project_id: String,
    pub session: Session,
    pub baseline: PlanProposal,
    pub context: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Head {
    pub project_id: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Receipt {
    pub project_id: String,
    pub session_id: String,
    pub operation: String,
    pub request: Value,
}

/// Only a newly committed user command may start a model process.
pub struct Transition {
    pub session: Session,
    pub launch: bool,
}

#[derive(Clone)]
pub struct RoundKey {
    pub project_id: String,
    pub session_id: String,
    pub round_id: String,
}

impl From<&Session> for RoundKey {
    fn from(session: &Session) -> Self {
        Self {
            project_id: session.project_id.clone(),
            session_id: session.id.clone(),
            round_id: session.round_id.clone(),
        }
    }
}
