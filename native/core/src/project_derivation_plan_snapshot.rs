//! Reconstruct already-validated, never-dispatched task history at a plan boundary.
use crate::{
    object_task_definition::TaskDefinition, object_task_types::TaskRecord,
    project_derivation_plan_validation::Plans,
};
use anyhow::{ensure, Result};

impl Plans {
    pub fn tasks_at(&self, plan_revision: u64) -> Result<Vec<TaskRecord>> {
        ensure!(
            plan_revision <= self.state.revision,
            "DERIVATION_PLAN_SNAPSHOT_REVISION"
        );
        let mut tasks = Vec::new();
        for current in self.tasks.values() {
            let (origin, cancellation) = self.task_boundaries(current)?;
            if origin > plan_revision {
                continue;
            }
            let mut revisions: Vec<_> = self
                .revisions
                .values()
                .filter(|r| r.task_id == current.id)
                .collect();
            revisions.sort_by_key(|r| r.plan_revision);
            let definition = revisions
                .iter()
                .find(|r| r.plan_revision > plan_revision)
                .map(|r| r.before.clone())
                .unwrap_or_else(|| TaskDefinition::from_task(current));
            let cancelled = cancellation.is_some_and(|r| r <= plan_revision);
            let mut task = current.clone();
            definition.apply_to(&mut task);
            task.revision = revisions
                .iter()
                .filter(|r| r.plan_revision <= plan_revision)
                .count() as u64
                + u64::from(cancelled);
            task.status = if cancelled { "cancelled" } else { "planned" }.into();
            tasks.push(task);
        }
        Ok(tasks)
    }
}
