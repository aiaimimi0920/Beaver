//! Validate references without treating old drafts as new commands against current definitions.
use crate::{
    object_framework::Baseline,
    object_task_types::{self, Granularity, PlanProposal, TaskProposal},
    project_derivation_plan_validation::Plans,
};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl Plans {
    pub fn proposal(&self, plan: &PlanProposal) -> Result<()> {
        object_task_types::validate_plan(plan, true)?;
        let tasks: BTreeMap<_, _> = plan.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
        let objects: BTreeSet<_> = plan.objects.iter().map(|o| o.id.as_str()).collect();
        for id in &objects {
            ensure!(
                !tasks.contains_key(id) && !self.tasks.contains_key(*id),
                "DERIVATION_PLAN_ID_COLLISION"
            );
        }
        for task in &plan.tasks {
            ensure!(
                !self.objects.contains_key(&task.id),
                "DERIVATION_PLAN_ID_COLLISION"
            );
            if let Some(id) = &task.object_id {
                ensure!(
                    objects.contains(id.as_str()) || self.objects.contains_key(id),
                    "DERIVATION_PLAN_OBJECT_MISSING"
                );
            }
            let parent = task
                .parent_task_id
                .as_deref()
                .map(|id| {
                    tasks
                        .get(id)
                        .map(|p| (p.granularity.clone(), p.object_id.clone()))
                        .or_else(|| {
                            self.tasks
                                .get(id)
                                .map(|p| (p.granularity.clone(), p.object_id.clone()))
                        })
                        .context("DERIVATION_PLAN_PARENT_MISSING")
                })
                .transpose()?;
            match task.granularity {
                Granularity::Coarse => ensure!(parent.is_none(), "DERIVATION_PLAN_COARSE_PARENT"),
                Granularity::Medium => {
                    ensure!(
                        parent.is_none_or(|p| p.0 == Granularity::Coarse),
                        "DERIVATION_PLAN_MEDIUM_PARENT"
                    );
                    self.baseline(task)?;
                }
                Granularity::Fine => ensure!(
                    parent.is_some_and(|p| p.0 == Granularity::Medium && p.1 == task.object_id),
                    "DERIVATION_PLAN_FINE_PARENT"
                ),
            }
            for id in &task.depends_on {
                ensure!(
                    id != &task.id
                        && (tasks.contains_key(id.as_str()) || self.tasks.contains_key(id)),
                    "DERIVATION_PLAN_DEPENDENCY_MISSING"
                );
            }
        }
        // Historical proposals may be stale against later dependency revisions. Check their
        // own graph here; current records are checked as a complete graph separately.
        acyclic(
            plan.tasks
                .iter()
                .map(|t| (t.id.clone(), t.depends_on.clone()))
                .collect(),
        )
    }

    fn baseline(&self, task: &TaskProposal) -> Result<()> {
        if let Some(Baseline::PinnedVersion {
            selected_version_id,
        }) = &task.baseline
        {
            let object = self
                .objects
                .get(task.object_id.as_deref().unwrap_or_default())
                .context("DERIVATION_PLAN_BASELINE_OBJECT_MISSING")?;
            let version = object
                .versions
                .iter()
                .find(|v| &v.version_id == selected_version_id)
                .context("DERIVATION_PLAN_BASELINE_VERSION_MISSING")?;
            crate::object_version_manifest::read_frozen_version(object, version)?;
        }
        Ok(())
    }
}

pub(crate) fn acyclic(graph: BTreeMap<String, Vec<String>>) -> Result<()> {
    let mut degrees = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, dependencies) in &graph {
        let internal: Vec<_> = dependencies
            .iter()
            .filter(|id| graph.contains_key(*id))
            .collect();
        degrees.insert(id.as_str(), internal.len());
        for dependency in internal {
            dependents.entry(dependency).or_default().push(id);
        }
    }
    let mut ready: VecDeque<_> = degrees
        .iter()
        .filter_map(|(id, n)| (*n == 0).then_some(*id))
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        for dependent in dependents.get(id).into_iter().flatten() {
            let count = degrees.get_mut(dependent).expect("known task");
            *count -= 1;
            if *count == 0 {
                ready.push_back(dependent);
            }
        }
    }
    ensure!(visited == graph.len(), "DERIVATION_PLAN_DEPENDENCY_CYCLE");
    Ok(())
}
