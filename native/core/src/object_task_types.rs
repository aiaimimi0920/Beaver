use crate::object_framework::{Baseline, Identity};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

pub const MAX_REVISION: u64 = i64::MAX as u64;
pub const MAX_ASSUMPTION_HISTORY: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssumptionSource {
    User,
    Codex,
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanAssumption {
    pub id: String,
    pub statement: String,
    pub basis: String,
    #[serde(default = "default_assumption_source")]
    pub source: AssumptionSource,
    #[serde(default)]
    pub source_detail: Option<String>,
}

fn default_assumption_source() -> AssumptionSource {
    AssumptionSource::User
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanAssumptionRecord {
    pub id: String,
    pub statement: String,
    pub basis: String,
    pub source: AssumptionSource,
    pub source_detail: Option<String>,
    pub plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Granularity {
    Coarse,
    Medium,
    Fine,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkRequirement {
    #[default]
    Required,
    Optional,
}

impl WorkRequirement {
    // Keep old frozen definitions byte-compatible when the default is used.
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectProposal {
    pub id: String,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "其他".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskProposal {
    pub id: String,
    #[serde(default)]
    pub position: u64,
    pub granularity: Granularity,
    pub title: String,
    pub prompt: String,
    pub acceptance: String,
    #[serde(default, skip_serializing_if = "WorkRequirement::is_required")]
    pub requirement: WorkRequirement,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pending_planning: String,
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Baseline>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanProposal {
    #[serde(default)]
    pub objects: Vec<ObjectProposal>,
    #[serde(default)]
    pub tasks: Vec<TaskProposal>,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveDraftRequest {
    pub project_id: String,
    pub draft_id: String,
    pub expected_revision: u64,
    pub expected_plan_revision: u64,
    pub plan: PlanProposal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub project_id: String,
    pub id: String,
    pub revision: u64,
    pub plan_revision: u64,
    pub plan: PlanProposal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnlockDraftRequest {
    pub project_id: String,
    pub request_id: String,
    pub draft_id: String,
    pub expected_revision: u64,
    pub expected_plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitRequest {
    pub project_id: String,
    pub request_id: String,
    pub draft_id: String,
    pub expected_draft_revision: u64,
    pub expected_plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelPlannedRequest {
    pub project_id: String,
    pub task_id: String,
    pub request_id: String,
    pub expected_task_revision: u64,
    pub expected_plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelPlannedReceipt {
    pub project_id: String,
    pub task_id: String,
    pub request_id: String,
    pub previous_task_revision: u64,
    pub task_revision: u64,
    pub previous_plan_revision: u64,
    pub plan_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRecord {
    pub id: String,
    #[serde(default)]
    pub position: u64,
    pub project_id: String,
    pub granularity: Granularity,
    pub title: String,
    pub prompt: String,
    pub acceptance: String,
    #[serde(default, skip_serializing_if = "WorkRequirement::is_required")]
    pub requirement: WorkRequirement,
    pub object_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pending_planning: String,
    pub parent_task_id: Option<String>,
    pub depends_on: Vec<String>,
    pub run_id: Option<String>,
    pub stage_id: Option<String>,
    pub identity: Identity,
    pub status: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunRecord {
    pub id: String,
    pub project_id: String,
    pub object_id: String,
    pub medium_task_id: String,
    pub baseline_version_id: Option<String>,
    pub status: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitReceipt {
    pub project_id: String,
    pub request_id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    pub previous_plan_revision: u64,
    pub plan_revision: u64,
    pub object_ids: Vec<String>,
    pub task_ids: Vec<String>,
    pub runs: Vec<RunRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    #[serde(default)]
    pub planning_states: Vec<crate::object_task_planning_declaration::State>,
    pub plan_revision: u64,
    pub tasks: Vec<TaskRecord>,
    pub runs: Vec<RunRecord>,
    pub assumptions: Vec<PlanAssumptionRecord>,
    pub dispatch_controls: Vec<crate::object_task_dispatch::Control>,
    pub coarse_dispatch_controls: Vec<crate::object_task_coarse_dispatch::Control>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanState {
    pub project_id: String,
    pub revision: u64,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRecord>,
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
}

pub fn validate_plan(plan: &PlanProposal, allow_empty: bool) -> Result<()> {
    ensure!(
        plan.objects.len() <= 128,
        "OBJECT_TASK_LIMIT: too many objects"
    );
    ensure!(plan.tasks.len() <= 500, "OBJECT_TASK_LIMIT: too many tasks");
    ensure!(
        plan.assumptions.len() <= 500,
        "OBJECT_TASK_LIMIT: too many assumptions"
    );
    ensure!(
        allow_empty || !plan.tasks.is_empty(),
        "OBJECT_TASK_EMPTY_PLAN"
    );
    let mut object_ids = std::collections::HashSet::new();
    for object in &plan.objects {
        ensure!(valid_id(&object.id), "INVALID_OBJECT_TASK_ID: object id");
        ensure!(
            object_ids.insert(&object.id),
            "DUPLICATE_OBJECT_ID: {}",
            object.id
        );
        ensure!(
            !object.name.trim().is_empty() && object.name.len() <= 256,
            "INVALID_OBJECT_TASK: object name"
        );
        ensure!(
            !object.category.trim().is_empty() && object.category.len() <= 128,
            "INVALID_OBJECT_TASK: object category"
        );
    }
    let mut task_ids = std::collections::HashSet::new();
    let mut assumption_ids = std::collections::HashSet::new();
    for assumption in &plan.assumptions {
        ensure!(
            valid_id(&assumption.id),
            "INVALID_OBJECT_TASK_ID: assumption"
        );
        ensure!(
            assumption_ids.insert(&assumption.id),
            "DUPLICATE_OBJECT_TASK_ASSUMPTION_ID: {}",
            assumption.id
        );
        ensure!(
            !assumption.statement.trim().is_empty() && assumption.statement.len() <= 2_000,
            "INVALID_OBJECT_TASK_ASSUMPTION: statement"
        );
        ensure!(
            !assumption.basis.trim().is_empty() && assumption.basis.len() <= 4_000,
            "INVALID_OBJECT_TASK_ASSUMPTION: basis"
        );
        ensure!(
            assumption
                .source_detail
                .as_ref()
                .is_none_or(|detail| detail.len() <= 1_000),
            "INVALID_OBJECT_TASK_ASSUMPTION: source detail"
        );
    }
    for task in &plan.tasks {
        ensure!(
            task.position <= 1_000_000_000,
            "INVALID_OBJECT_TASK: position"
        );
        ensure!(valid_id(&task.id), "INVALID_OBJECT_TASK_ID: task id");
        ensure!(
            task_ids.insert(&task.id),
            "DUPLICATE_OBJECT_TASK_ID: {}",
            task.id
        );
        ensure!(
            !task.title.trim().is_empty() && task.title.len() <= 300,
            "INVALID_OBJECT_TASK: title"
        );
        ensure!(
            !task.prompt.trim().is_empty() && task.prompt.len() <= 20_000,
            "INVALID_OBJECT_TASK: prompt"
        );
        ensure!(
            task.acceptance.len() <= 10_000,
            "INVALID_OBJECT_TASK: acceptance"
        );
        ensure!(
            task.pending_planning.len() <= 4_000
                && (task.granularity != Granularity::Fine || task.pending_planning.is_empty()),
            "INVALID_OBJECT_TASK: pending planning"
        );
        ensure!(
            task.depends_on.len() <= 500 && task.depends_on.iter().all(|id| valid_id(id)),
            "INVALID_OBJECT_TASK: dependencies"
        );
        let mut dependencies = std::collections::HashSet::new();
        ensure!(
            task.depends_on.iter().all(|id| dependencies.insert(id)),
            "DUPLICATE_OBJECT_TASK_DEPENDENCY: {}",
            task.id
        );
        match task.granularity {
            Granularity::Coarse => ensure!(
                task.object_id.is_none() && task.stage_id.is_none() && task.baseline.is_none(),
                "INVALID_OBJECT_TASK_IDENTITY: coarse"
            ),
            Granularity::Medium => ensure!(
                task.object_id.as_deref().is_some_and(valid_id) && task.stage_id.is_none(),
                "INVALID_OBJECT_TASK_IDENTITY: medium"
            ),
            Granularity::Fine => ensure!(
                task.object_id.as_deref().is_some_and(valid_id)
                    && task.stage_id.as_deref().is_some_and(valid_id)
                    && task.parent_task_id.as_deref().is_some_and(valid_id)
                    && task.baseline.is_none(),
                "INVALID_OBJECT_TASK_IDENTITY: fine"
            ),
        }
        if let Some(Baseline::PinnedVersion {
            selected_version_id,
        }) = &task.baseline
        {
            ensure!(
                valid_id(selected_version_id),
                "INVALID_OBJECT_TASK_BASELINE_VERSION"
            );
        }
        if let Some(parent) = &task.parent_task_id {
            ensure!(valid_id(parent), "INVALID_OBJECT_TASK_ID: parent");
        }
    }
    Ok(())
}
