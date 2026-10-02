//! Derivation-only ownership and never-dispatched state checks, not normal write validation.
use crate::{
    object_catalog::ObjectRecord,
    object_framework::{Identity, VERSION},
    object_task_definition::{TaskDefinition, TaskDefinitionRevision},
    object_task_draft_unlock::Receipt as UnlockReceipt,
    object_task_types::{
        self, CancelPlannedReceipt, CommitReceipt, Draft, Granularity, PlanProposal, PlanState,
        RunRecord, TaskRecord, MAX_REVISION,
    },
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

pub(crate) struct Plans {
    pub project: String,
    pub state: PlanState,
    pub objects: BTreeMap<String, ObjectRecord>,
    pub tasks: BTreeMap<String, TaskRecord>,
    pub runs: BTreeMap<String, RunRecord>,
    pub drafts: BTreeMap<String, Draft>,
    pub commits: BTreeMap<String, CommitReceipt>,
    pub cancels: BTreeMap<String, CancelPlannedReceipt>,
    pub revisions: BTreeMap<String, TaskDefinitionRevision>,
    pub unlocks: BTreeMap<String, UnlockReceipt>,
}

pub(crate) fn records<T: DeserializeOwned>(
    entities: &[Entity],
    kind: &str,
) -> Result<BTreeMap<String, T>> {
    entities
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| {
            Ok((
                e.id.clone(),
                serde_json::from_value(e.value.clone().context("invalid plan JSON")?)
                    .with_context(|| format!("derivation {kind}/{}", e.id))?,
            ))
        })
        .collect()
}

pub(crate) fn validate(entities: &[Entity], project: &str) -> Result<()> {
    for e in entities
        .iter()
        .filter(|e| crate::project_derivation_plan_records::supports(&e.kind))
    {
        ensure!(
            object_task_types::valid_id(&e.id),
            "DERIVATION_PLAN_INVALID_KEY"
        );
        let value = e.value.as_ref().context("invalid plan JSON")?;
        let (owner, key) = match e.kind.as_str() {
            "object_task_plan_state" => (&value["projectId"], &value["projectId"]),
            "object_task_draft_unlock_receipt" => {
                ensure!(
                    value.as_object().is_some_and(|o| o.len() == 2
                        && o.contains_key("request")
                        && o.contains_key("draft")),
                    "DERIVATION_PLAN_UNKNOWN_UNLOCK_FIELD"
                );
                (
                    &value["request"]["projectId"],
                    &value["request"]["requestId"],
                )
            }
            "object_task" | "object_run" | "object_task_draft" => {
                (&value["projectId"], &value["id"])
            }
            _ => (&value["projectId"], &value["requestId"]),
        };
        ensure!(
            owner == project && key == &e.id,
            "DERIVATION_PLAN_IDENTITY_MISMATCH"
        );
    }
    let plans = Plans::from_entities(entities, project)?;
    plans.current()?;
    plans.history()?;
    crate::project_derivation_planning_validation::validate(entities, &plans)?;
    crate::project_derivation_declaration_records::validate(entities, &plans)
}

impl Plans {
    pub fn from_entities(entities: &[Entity], project: &str) -> Result<Self> {
        let mut plans = Self::current_entities(entities, project)?;
        crate::project_derivation_execution_validation::project_plans(entities, &mut plans)?;
        Ok(plans)
    }

    pub fn current_entities(entities: &[Entity], project: &str) -> Result<Self> {
        let mut states: BTreeMap<String, PlanState> = records(entities, "object_task_plan_state")?;
        Ok(Self {
            project: project.into(),
            state: states.remove(project).unwrap_or(PlanState {
                project_id: project.into(),
                revision: 0,
                assumptions: vec![],
            }),
            objects: records(entities, "object")?,
            tasks: records(entities, "object_task")?,
            runs: records(entities, "object_run")?,
            drafts: records(entities, "object_task_draft")?,
            commits: records(entities, "object_task_commit_receipt")?,
            cancels: records(entities, "object_task_cancel_receipt")?,
            revisions: records(entities, "object_task_definition_revision")?,
            unlocks: records(entities, "object_task_draft_unlock_receipt")?,
        })
    }

    pub fn task(&self, id: &str) -> Result<&TaskRecord> {
        self.tasks.get(id).context("DERIVATION_PLAN_TASK_MISSING")
    }

    pub fn run(&self, id: &str) -> Result<&RunRecord> {
        self.runs.get(id).context("DERIVATION_PLAN_RUN_MISSING")
    }

    fn current(&self) -> Result<()> {
        ensure!(
            self.state.revision <= MAX_REVISION
                && self.state.assumptions.len() <= object_task_types::MAX_ASSUMPTION_HISTORY,
            "DERIVATION_PLAN_INVALID_STATE"
        );
        for assumption in &self.state.assumptions {
            ensure!(
                assumption.plan_revision > 0 && assumption.plan_revision <= self.state.revision,
                "DERIVATION_PLAN_ASSUMPTION_REVISION"
            );
            object_task_types::validate_plan(
                &PlanProposal {
                    assumptions: vec![object_task_types::PlanAssumption {
                        id: assumption.id.clone(),
                        statement: assumption.statement.clone(),
                        basis: assumption.basis.clone(),
                        source: assumption.source.clone(),
                        source_detail: assumption.source_detail.clone(),
                    }],
                    ..Default::default()
                },
                true,
            )?;
        }
        for task in self.tasks.values() {
            ensure!(
                matches!(task.status.as_str(), "planned" | "cancelled")
                    && task.revision <= MAX_REVISION,
                "DERIVATION_PLAN_EXECUTED_TASK"
            );
            self.proposal(&PlanProposal {
                tasks: vec![TaskDefinition::from_task(task).proposal(task)],
                ..Default::default()
            })?;
            self.identity(task)?;
        }
        crate::project_derivation_plan_proposals::acyclic(
            self.tasks
                .values()
                .map(|t| (t.id.clone(), t.depends_on.clone()))
                .collect(),
        )?;
        for run in self.runs.values() {
            self.run_identity(run)?;
            let task = self.task(&run.medium_task_id)?;
            ensure!(
                run.status == task.status
                    && run.baseline_version_id.is_none()
                    && run.revision == u64::from(run.status == "cancelled"),
                "DERIVATION_PLAN_EXECUTED_RUN"
            );
        }
        Ok(())
    }

    pub fn run_identity(&self, run: &RunRecord) -> Result<()> {
        let task = self.task(&run.medium_task_id)?;
        ensure!(
            run.project_id == self.project
                && task.granularity == Granularity::Medium
                && task.run_id.as_deref() == Some(&run.id)
                && task.object_id.as_deref() == Some(&run.object_id),
            "DERIVATION_PLAN_RUN_IDENTITY"
        );
        Ok(())
    }

    fn identity(&self, task: &TaskRecord) -> Result<()> {
        let valid = match &task.identity {
            Identity::Coarse { schema_version } => {
                *schema_version == VERSION
                    && task.granularity == Granularity::Coarse
                    && task.run_id.is_none()
            }
            Identity::Medium {
                schema_version,
                object_id,
                ..
            } => {
                let run = self.run(
                    task.run_id
                        .as_deref()
                        .context("DERIVATION_PLAN_RUN_MISSING")?,
                )?;
                *schema_version == VERSION
                    && task.granularity == Granularity::Medium
                    && task.object_id.as_deref() == Some(object_id)
                    && run.medium_task_id == task.id
            }
            Identity::Fine {
                schema_version,
                object_id,
                medium_task_id,
                run_id,
                stage_id,
            } => {
                let parent = self.task(medium_task_id)?;
                *schema_version == VERSION
                    && task.granularity == Granularity::Fine
                    && task.object_id.as_deref() == Some(object_id)
                    && task.parent_task_id.as_deref() == Some(medium_task_id)
                    && task.run_id.as_deref() == Some(run_id)
                    && parent.run_id.as_deref() == Some(run_id)
                    && task.stage_id.as_deref() == Some(stage_id)
            }
        };
        ensure!(valid, "DERIVATION_PLAN_TASK_IDENTITY");
        if let Some(parent) = &task.parent_task_id {
            ensure!(
                self.task(parent)?.status != "cancelled" || task.status == "cancelled",
                "DERIVATION_PLAN_CANCELLED_PARENT"
            );
        }
        Ok(())
    }
}
