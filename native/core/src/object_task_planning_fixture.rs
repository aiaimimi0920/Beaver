use crate::{
    object_task_planning::{self as planning, Session, SessionRequest, StartRequest},
    object_tasks::PlanProposal,
    project_runtime::ProjectRuntime,
};
use anyhow::Result;
use serde_json::{json, Value};

pub(crate) fn input() -> StartRequest {
    StartRequest {
        project_id: "project-1".into(),
        request_id: "planning:1".into(),
        draft_id: "main".into(),
        expected_draft_revision: 0,
        expected_plan_revision: 0,
        goal: "Create a hero object".into(),
        acceptance: "A reviewable model".into(),
        ask_ratio: 100,
    }
}

pub(crate) fn session(runtime: &ProjectRuntime) -> Result<Session> {
    Ok(planning::get(runtime, "project-1", "main")?.unwrap())
}

pub(crate) fn command(session: &Session, id: &str) -> SessionRequest {
    SessionRequest {
        project_id: session.project_id.clone(),
        session_id: session.id.clone(),
        request_id: id.into(),
        expected_revision: session.revision,
    }
}

pub(crate) fn question(id: &str, importance: u8) -> Value {
    json!({"id":id,"question":"Which style?","options":[{"label":"Stylized"},{"label":"Realistic"}],
        "importance":importance,"recommended":"Stylized","reason":"Fits the brief"})
}

pub(crate) fn proposal() -> PlanProposal {
    serde_json::from_value(json!({
        "objects":[{"id":"hero","name":"Hero","category":"character"}],
        "tasks":[{"id":"make-hero","granularity":"medium","objectId":"hero",
            "title":"Make hero","prompt":"Build the hero","acceptance":"Visible model"}],
        "assumptions":[{"id":"model-choice","statement":"Low poly","basis":"Simple game",
            "source":"user","sourceDetail":"untrusted source claim"}]
    }))
    .unwrap()
}
