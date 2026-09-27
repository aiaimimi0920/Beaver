use crate::object_task_types::{TaskProposal, TaskRecord, WorkRequirement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskDefinition {
    pub title: String,
    pub prompt: String,
    pub acceptance: String,
    #[serde(default, skip_serializing_if = "WorkRequirement::is_required")]
    pub requirement: WorkRequirement,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pending_planning: String,
    pub depends_on: Vec<String>,
}

impl TaskDefinition {
    pub fn from_task(task: &TaskRecord) -> Self {
        Self {
            title: task.title.clone(),
            prompt: task.prompt.clone(),
            acceptance: task.acceptance.clone(),
            requirement: task.requirement,
            pending_planning: task.pending_planning.clone(),
            depends_on: task.depends_on.clone(),
        }
    }

    pub(crate) fn proposal(&self, task: &TaskRecord) -> TaskProposal {
        TaskProposal {
            id: task.id.clone(),
            position: task.position,
            granularity: task.granularity.clone(),
            title: self.title.clone(),
            prompt: self.prompt.clone(),
            acceptance: self.acceptance.clone(),
            requirement: self.requirement,
            pending_planning: self.pending_planning.clone(),
            object_id: task.object_id.clone(),
            parent_task_id: task.parent_task_id.clone(),
            depends_on: self.depends_on.clone(),
            stage_id: task.stage_id.clone(),
            baseline: match &task.identity {
                crate::object_framework::Identity::Medium { baseline, .. } => {
                    Some(baseline.clone())
                }
                _ => None,
            },
        }
    }

    pub(crate) fn apply_to(&self, task: &mut TaskRecord) {
        task.title = self.title.clone();
        task.prompt = self.prompt.clone();
        task.acceptance = self.acceptance.clone();
        task.requirement = self.requirement;
        task.pending_planning = self.pending_planning.clone();
        task.depends_on = self.depends_on.clone();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisePlannedRequest {
    pub project_id: String,
    pub task_id: String,
    pub request_id: String,
    pub expected_task_revision: u64,
    pub expected_plan_revision: u64,
    pub definition: TaskDefinition,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DefinitionAdopter {
    Owner,
}

/// The immutable history entry is also the idempotent command receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskDefinitionRevision {
    pub project_id: String,
    pub task_id: String,
    pub request_id: String,
    pub previous_task_revision: u64,
    pub task_revision: u64,
    pub previous_plan_revision: u64,
    pub plan_revision: u64,
    pub before: TaskDefinition,
    pub after: TaskDefinition,
    pub reason: String,
    pub adopted_by: DefinitionAdopter,
    pub created_at: String,
    pub affected_task_ids: Vec<String>,
}
