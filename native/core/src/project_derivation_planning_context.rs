//! Frozen context is validated as history, not replaced by today's object/task definitions.
use crate::{
    object_catalog::ObjectRecord,
    object_task_definition::TaskDefinition,
    object_task_types::{PlanAssumptionRecord, PlanProposal, RunRecord, TaskRecord},
    project_derivation_copy::Request,
    project_derivation_identity::IdentityMap,
    project_derivation_plan_rewrite::PlanIds,
    project_derivation_plan_validation::Plans,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanningContext {
    pub objects: Vec<ObjectRecord>,
    pub tasks: Vec<TaskRecord>,
    pub runs: Vec<RunRecord>,
    pub assumptions: Vec<PlanAssumptionRecord>,
}

impl PlanningContext {
    pub fn validate(&self, plans: &Plans, plan_revision: u64) -> Result<()> {
        ensure!(
            serde_json::to_vec(self)?.len() <= 256 * 1024,
            "DERIVATION_PLANNING_CONTEXT_LIMIT"
        );
        unique(self.objects.iter().map(|o| o.id.as_str()))?;
        unique(self.tasks.iter().map(|t| t.id.as_str()))?;
        unique(self.runs.iter().map(|r| r.id.as_str()))?;
        unique(
            self.assumptions
                .iter()
                .map(|a| (a.id.as_str(), a.plan_revision)),
        )?;
        for object in &self.objects {
            ensure!(
                object.project_id == plans.project,
                "DERIVATION_PLANNING_CONTEXT_OWNER"
            );
            crate::project_derivation_object_validation::snapshot(&plans.objects, object)?;
        }
        let mut proposal = PlanProposal::default();
        for task in &self.tasks {
            let current = plans.task(&task.id)?;
            let (origin, cancellation) = plans.task_boundaries(current)?;
            let cancelled = cancellation.is_some_and(|r| r <= plan_revision);
            let revision = plans
                .revisions
                .values()
                .filter(|r| r.task_id == task.id && r.plan_revision <= plan_revision)
                .count() as u64
                + u64::from(cancelled);
            ensure!(
                task.project_id == plans.project
                    && task.revision <= current.revision
                    && origin <= plan_revision
                    && task.revision == revision
                    && task.status == if cancelled { "cancelled" } else { "planned" },
                "DERIVATION_PLANNING_CONTEXT_TASK"
            );
            let mut normalized = task.clone();
            TaskDefinition::from_task(current).apply_to(&mut normalized);
            normalized.revision = current.revision;
            normalized.status = current.status.clone();
            ensure!(
                normalized == *current,
                "DERIVATION_PLANNING_CONTEXT_TASK_IDENTITY"
            );
            let definition = plans
                .revisions
                .values()
                .find(|r| r.task_id == task.id && r.previous_task_revision == task.revision)
                .map(|r| r.before.clone())
                .unwrap_or_else(|| TaskDefinition::from_task(current));
            ensure!(
                TaskDefinition::from_task(task) == definition,
                "DERIVATION_PLANNING_CONTEXT_DEFINITION"
            );
            if task.revision == current.revision {
                ensure!(task == current, "DERIVATION_PLANNING_CONTEXT_TASK");
            }
            proposal
                .tasks
                .push(TaskDefinition::from_task(task).proposal(task));
        }
        let expected_tasks = plans
            .tasks
            .values()
            .map(|task| plans.task_boundaries(task))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|(origin, _)| *origin <= plan_revision)
            .count();
        ensure!(
            self.tasks.len() == expected_tasks,
            "DERIVATION_PLANNING_CONTEXT_TASK_SET"
        );
        plans.proposal(&proposal)?;
        for run in &self.runs {
            let current = plans.run(&run.id)?;
            ensure!(
                run.project_id == plans.project
                    && run.revision <= current.revision
                    && run.revision == u64::from(run.status == "cancelled")
                    && matches!(run.status.as_str(), "planned" | "cancelled"),
                "DERIVATION_PLANNING_CONTEXT_RUN"
            );
            let task = self
                .tasks
                .iter()
                .find(|t| t.id == run.medium_task_id)
                .context("DERIVATION_PLANNING_CONTEXT_RUN_TASK")?;
            ensure!(
                task.run_id.as_ref() == Some(&run.id) && task.status == run.status,
                "DERIVATION_PLANNING_CONTEXT_RUN_TASK"
            );
            let mut normalized = run.clone();
            normalized.revision = current.revision;
            normalized.status = current.status.clone();
            ensure!(
                normalized == *current,
                "DERIVATION_PLANNING_CONTEXT_RUN_IDENTITY"
            );
        }
        ensure!(
            self.runs.len()
                == self
                    .tasks
                    .iter()
                    .filter(|t| t.granularity == crate::object_task_types::Granularity::Medium)
                    .count(),
            "DERIVATION_PLANNING_CONTEXT_RUN_SET"
        );
        for assumption in &self.assumptions {
            ensure!(
                assumption.plan_revision <= plan_revision
                    && plans.state.assumptions.contains(assumption),
                "DERIVATION_PLANNING_CONTEXT_ASSUMPTION"
            );
        }
        ensure!(
            self.assumptions.len()
                == plans
                    .state
                    .assumptions
                    .iter()
                    .filter(|a| a.plan_revision <= plan_revision)
                    .count(),
            "DERIVATION_PLANNING_CONTEXT_ASSUMPTION_SET"
        );
        Ok(())
    }

    pub fn rewrite(&mut self, map: &IdentityMap, request: &Request) -> Result<()> {
        let ids = PlanIds(request);
        for object in &mut self.objects {
            crate::project_derivation_object_records::rewrite_snapshot(map, request, object)?;
        }
        for task in &mut self.tasks {
            ids.task(task)?;
        }
        for run in &mut self.runs {
            ids.run(run)?;
        }
        for assumption in &mut self.assumptions {
            ids.provenance(&mut assumption.source_detail)?;
        }
        Ok(())
    }
}

fn unique<T: Ord>(ids: impl Iterator<Item = T>) -> Result<()> {
    let mut seen = BTreeSet::new();
    ensure!(
        ids.into_iter().all(|id| seen.insert(id)),
        "DERIVATION_PLANNING_CONTEXT_DUPLICATE"
    );
    Ok(())
}
