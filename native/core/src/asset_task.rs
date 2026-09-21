use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL: u32 = 1;
pub const MAX_FEEDBACK: usize = 128;
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Frame {
    pub id: String,
    pub session_id: String,
    pub generation: String,
    pub scene_revision: u64,
    pub view_revision: u64,
    pub captured_at: u64,
    pub width: u32,
    pub height: u32,
    pub view_matrix: [f64; 16],
    pub projection_matrix: [f64; 16],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pick {
    pub frame_id: String,
    pub object_id: String,
    pub object_name: String,
    pub instance_id: String,
    pub local: [f64; 3],
    pub world: [f64; 3],
    pub normal: [f64; 3],
    pub face: i64,
    pub vertices: u64,
    pub polygons: u64,
    pub point: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Annotation {
    pub kind: String,
    pub points: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: String,
    pub task_id: String,
    pub project_id: String,
    pub frame: Frame,
    pub pick: Option<Pick>,
    pub image_path: String,
    pub sha256: String,
    pub used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    pub name: String,
    pub status: String,
    pub dependencies: Vec<String>,
    pub objects: Vec<String>,
    pub evidence: String,
    pub round: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub id: String,
    pub source_task_id: String,
    pub task_id: String,
    pub fingerprint: String,
    pub timing: String,
    pub text: String,
    pub reference: Reference,
    pub annotations: Vec<Annotation>,
    pub round: u32,
    pub status: String,
    pub created_at: String,
    pub delivered_at: Option<String>,
    pub impact: String,
    pub affected_stages: Vec<String>,
    #[serde(default)]
    pub resume_stages: Vec<String>,
    pub image_observation: String,
    pub evidence: String,
    pub checkpoint: Option<String>,
    pub history: Vec<Value>,
}
impl Feedback {
    pub fn terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "completed" | "cancelled" | "forwarded"
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub protocol_version: u32,
    pub task_id: String,
    pub project_id: String,
    pub revision: u64,
    pub round: u32,
    pub phase: String,
    pub session_id: Option<String>,
    pub stages: Vec<Stage>,
    pub feedback: Vec<Feedback>,
    #[serde(default)]
    pub withdrawn: Vec<String>,
    pub checkpoint: Option<String>,
    pub last_frame: Option<Reference>,
    pub recovery: Option<String>,
    #[serde(default)]
    pub delivery: Option<crate::asset_delivery::Workflow>,
    #[serde(default)]
    pub work: crate::asset_work::Work,
}

impl State {
    pub fn ready(&self) -> bool {
        self.phase == "ready"
            && self.feedback.iter().all(Feedback::terminal)
            && crate::asset_delivery::complete(self)
    }
}

pub fn get(store: &Store, id: &str) -> Result<State> {
    store
        .get("asset-task", id)?
        .context("Asset task is not enabled")
}
pub fn save(store: &Store, state: &State) -> Result<()> {
    store.put("asset-task", &state.task_id, state)
}
pub fn enable(store: &Store, task: &Value) -> Result<State> {
    crate::object_framework::require_legacy(task)?;
    let id = task["id"].as_str().context("Missing task ID")?;
    if let Some(state) = store.get("asset-task", id)? {
        return Ok(state);
    }
    let state = initial(task)?;
    let mut task = task.clone();
    task["assetTask"] = json!(true);
    store.put("task", id, &task)?;
    save(store, &state)?;
    Ok(state)
}

pub(crate) fn initial(task: &Value) -> Result<State> {
    crate::object_framework::require_legacy(task)?;
    let id = task["id"].as_str().context("Missing task ID")?;
    let mut state = State {
        protocol_version: PROTOCOL,
        task_id: id.into(),
        project_id: task["projectId"]
            .as_str()
            .context("Missing project")?
            .into(),
        revision: 0,
        round: 1,
        phase: "producing".into(),
        session_id: None,
        stages: Vec::new(),
        feedback: Vec::new(),
        withdrawn: Vec::new(),
        checkpoint: None,
        last_frame: None,
        recovery: None,
        delivery: None,
        work: Default::default(),
    };
    if !task["assetFeedbackSeed"].is_null() {
        let mut feedback: Feedback = serde_json::from_value(task["assetFeedbackSeed"].clone())?;
        feedback.task_id = id.into();
        feedback.round = 1;
        feedback.status = "received".into();
        state.feedback.push(feedback);
    }
    state.checkpoint = task["assetRestore"].as_str().map(str::to_owned);
    Ok(state)
}

pub fn eligible(state: &State) -> Option<&Feedback> {
    state
        .feedback
        .iter()
        .find(|f| !f.terminal() && f.round <= state.round)
}

pub fn recover(store: &Store, id: &str, reason: &str) -> Result<()> {
    mark_uncertain(store, id, reason)?;
    let Some(mut state) = store.get::<State>("asset-task", id)? else {
        return Ok(());
    };
    state.session_id = None;
    save(store, &state)
}

pub fn mark_uncertain(store: &Store, id: &str, reason: &str) -> Result<()> {
    let Some(mut state) = store.get::<State>("asset-task", id)? else {
        return Ok(());
    };
    crate::framework_evidence::recovery(store, id, reason, None)?;
    for feedback in &mut state.feedback {
        if matches!(feedback.status.as_str(), "executing" | "checking") {
            feedback.status = "pendingVerification".into();
            feedback
                .history
                .push(json!({"at":now(),"status":"pendingVerification","evidence":reason}));
        }
    }
    state.recovery = Some(reason.into());
    crate::asset_work::interrupt(&mut state, reason);
    save(store, &state)
}

pub fn recover_all(store: &Store) -> Result<()> {
    for state in store.list::<State>("asset-task")? {
        if state.session_id.is_some()
            || state.work.attempts.iter().any(|a| a.status == "running")
            || state
                .feedback
                .iter()
                .any(|f| matches!(f.status.as_str(), "executing" | "checking"))
        {
            recover(store, &state.task_id, "Beaver restarted. Only the last saved scene is recoverable; inspect before replaying relative edits.")?;
        }
    }
    Ok(())
}

/// Called while holding the same store lock as final merge and feedback receipt.
pub fn finish_barrier(store: &Store, task: &mut Value) -> Result<bool> {
    let id = task["id"].as_str().context("Missing task ID")?.to_owned();
    let Some(mut state) = store.get::<State>("asset-task", &id)? else {
        return Ok(false);
    };
    if state.ready() {
        return Ok(false);
    }
    // Resumed asset work must recapture output instead of reusing a GUT candidate.
    task["validationOnly"] = json!(false);
    task["validationPrepared"] = json!(false);
    if state.phase == "adjusting" && eligible(&state).is_some_and(|f| f.delivered_at.is_none()) {
        task["status"] = json!("queued");
        task["assetResume"] = json!(true);
    } else {
        recover(store, &id, "Execution ended before verified asset delivery; inspect before replaying modifications.")?;
        state = get(store, &id)?;
        task["status"] = json!("failed");
        task["error"] =
            json!("资产阶段或反馈尚未完成核验。现场与队列已保留；请显式继续，先检查已有修改。");
    }
    save(store, &state)?;
    store.put("task", &id, task)?;
    Ok(true)
}

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 100
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("Invalid asset identity");
    }
    Ok(())
}
