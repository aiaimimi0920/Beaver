//! Receipt boundaries and revision provenance; old snapshots need not equal current projections.
use crate::{
    object_task_definition::TaskDefinition,
    object_task_types::{Draft, PlanProposal, MAX_REVISION},
    project_derivation_plan_validation::Plans,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeSet;

impl Plans {
    pub fn history(&self) -> Result<()> {
        let mut changes = BTreeSet::new();
        for receipt in self.commits.values() {
            self.step(
                receipt.previous_plan_revision,
                receipt.plan_revision,
                true,
                &mut changes,
            )?;
            let draft = self
                .drafts
                .get(&receipt.draft_id)
                .context("DERIVATION_PLAN_DRAFT_MISSING")?;
            ensure!(
                receipt.draft_revision > 0 && receipt.draft_revision < draft.revision,
                "DERIVATION_PLAN_COMMIT_REVISION"
            );
            unique(&receipt.object_ids)?;
            unique(&receipt.task_ids)?;
            ensure!(!receipt.task_ids.is_empty(), "DERIVATION_PLAN_EMPTY_COMMIT");
            for id in &receipt.object_ids {
                ensure!(
                    self.objects.contains_key(id),
                    "DERIVATION_PLAN_OBJECT_MISSING"
                );
            }
            let expected: BTreeSet<_> = receipt
                .task_ids
                .iter()
                .map(|id| self.task(id))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .filter_map(|t| t.run_id.as_ref())
                .collect();
            let mut runs = BTreeSet::new();
            for run in &receipt.runs {
                ensure!(runs.insert(&run.id), "DERIVATION_PLAN_DUPLICATE_RUN");
                self.run_identity(run)?;
                let current = self.run(&run.id)?;
                let mut snapshot = run.clone();
                ensure!(
                    matches!(run.status.as_str(), "planned" | "cancelled")
                        && run.revision == u64::from(run.status == "cancelled")
                        && run.revision <= current.revision,
                    "DERIVATION_PLAN_RUN_RECEIPT_STATE"
                );
                snapshot.status = current.status.clone();
                snapshot.revision = current.revision;
                ensure!(snapshot == *current, "DERIVATION_PLAN_RUN_RECEIPT_IDENTITY");
            }
            ensure!(runs == expected, "DERIVATION_PLAN_COMMIT_RUNS");
        }
        for receipt in self.cancels.values() {
            self.step(
                receipt.previous_plan_revision,
                receipt.plan_revision,
                false,
                &mut changes,
            )?;
            let task = self.task(&receipt.task_id)?;
            ensure!(
                task.status == "cancelled"
                    && receipt.previous_task_revision < MAX_REVISION
                    && receipt.task_revision == receipt.previous_task_revision + 1
                    && task.revision == receipt.task_revision,
                "DERIVATION_PLAN_CANCEL_REVISION"
            );
        }
        for receipt in self.revisions.values() {
            self.step(
                receipt.previous_plan_revision,
                receipt.plan_revision,
                false,
                &mut changes,
            )?;
            let task = self.task(&receipt.task_id)?;
            ensure!(
                receipt.previous_task_revision < MAX_REVISION
                    && receipt.task_revision == receipt.previous_task_revision + 1
                    && receipt.task_revision <= task.revision
                    && receipt.before != receipt.after
                    && !receipt.reason.trim().is_empty()
                    && receipt.reason.len() <= 2_000
                    && chrono::DateTime::parse_from_rfc3339(&receipt.created_at).is_ok(),
                "DERIVATION_PLAN_DEFINITION_REVISION"
            );
            for definition in [&receipt.before, &receipt.after] {
                self.proposal(&PlanProposal {
                    tasks: vec![definition.proposal(task)],
                    ..Default::default()
                })?;
            }
            unique(&receipt.affected_task_ids)?;
            ensure!(
                receipt.affected_task_ids.contains(&task.id),
                "DERIVATION_PLAN_REVISION_SCOPE"
            );
            for id in &receipt.affected_task_ids {
                self.task(id)?;
            }
        }
        ensure!(
            changes.len() as u64 == self.state.revision
                && changes.iter().enumerate().all(|(i, n)| *n == i as u64 + 1),
            "DERIVATION_PLAN_HISTORY_GAP"
        );
        self.task_history()?;
        for draft in self.drafts.values() {
            self.draft(draft)?;
        }
        for receipt in self.unlocks.values() {
            let request = &receipt.request;
            let draft = &receipt.draft;
            self.draft(draft)?;
            let current = self
                .drafts
                .get(&request.draft_id)
                .context("DERIVATION_PLAN_DRAFT_MISSING")?;
            ensure!(
                request.expected_revision < MAX_REVISION
                    && draft.id == request.draft_id
                    && draft.revision == request.expected_revision + 1
                    && draft.revision <= current.revision
                    && draft.plan_revision == request.expected_plan_revision
                    && draft.committed_request_id.is_none()
                    && draft.plan == PlanProposal::default(),
                "DERIVATION_PLAN_UNLOCK_RECEIPT"
            );
            ensure!(
                self.commits
                    .values()
                    .any(|c| c.draft_id == draft.id
                        && c.draft_revision + 1 == request.expected_revision),
                "DERIVATION_PLAN_UNLOCK_COMMIT_MISSING"
            );
        }
        Ok(())
    }

    fn step(
        &self,
        before: u64,
        after: u64,
        allow_noop: bool,
        changes: &mut BTreeSet<u64>,
    ) -> Result<()> {
        ensure!(
            before <= after
                && after <= self.state.revision
                && (after - before == 1 || (allow_noop && after == before)),
            "DERIVATION_PLAN_RECEIPT_REVISION"
        );
        if after > before {
            ensure!(changes.insert(after), "DERIVATION_PLAN_DUPLICATE_REVISION");
        }
        Ok(())
    }

    fn draft(&self, draft: &Draft) -> Result<()> {
        ensure!(
            draft.project_id == self.project
                && crate::object_task_types::valid_id(&draft.id)
                && draft.revision > 0
                && draft.revision <= MAX_REVISION
                && draft.plan_revision <= self.state.revision,
            "DERIVATION_PLAN_DRAFT_REVISION"
        );
        self.proposal(&draft.plan)?;
        if let Some(id) = &draft.committed_request_id {
            let receipt = self
                .commits
                .get(id)
                .context("DERIVATION_PLAN_COMMIT_MISSING")?;
            ensure!(
                receipt.draft_id == draft.id
                    && receipt.draft_revision + 1 == draft.revision
                    && receipt.previous_plan_revision == draft.plan_revision
                    && receipt.task_ids
                        == draft
                            .plan
                            .tasks
                            .iter()
                            .map(|t| t.id.clone())
                            .collect::<Vec<_>>()
                    && receipt.object_ids
                        == draft
                            .plan
                            .objects
                            .iter()
                            .map(|o| o.id.clone())
                            .collect::<Vec<_>>(),
                "DERIVATION_PLAN_DRAFT_COMMIT_MISMATCH"
            );
        }
        Ok(())
    }

    pub fn task_boundaries(
        &self,
        task: &crate::object_task_types::TaskRecord,
    ) -> Result<(u64, Option<u64>)> {
        let origin = self
            .commits
            .values()
            .filter(|c| c.task_ids.contains(&task.id))
            .map(|c| c.plan_revision)
            .min()
            .context("DERIVATION_PLAN_TASK_COMMIT_MISSING")?;
        let mut ancestors = BTreeSet::from([task.id.as_str()]);
        let mut next = task.parent_task_id.as_deref();
        while let Some(id) = next {
            ensure!(ancestors.insert(id), "DERIVATION_PLAN_PARENT_CYCLE");
            next = self.task(id)?.parent_task_id.as_deref();
        }
        let cancellation = self
            .cancels
            .values()
            .filter(|c| ancestors.contains(c.task_id.as_str()))
            .map(|c| c.plan_revision)
            .min();
        Ok((origin, cancellation))
    }

    fn task_history(&self) -> Result<()> {
        for task in self.tasks.values() {
            let (origin, cancellation) = self.task_boundaries(task)?;
            ensure!(
                (task.status == "cancelled") == cancellation.is_some()
                    && cancellation.is_none_or(|r| r > origin),
                "DERIVATION_PLAN_CANCEL_HISTORY_MISSING"
            );
            let mut revisions: Vec<_> = self
                .revisions
                .values()
                .filter(|r| r.task_id == task.id)
                .collect();
            revisions.sort_by_key(|r| r.task_revision);
            let mut definition = revisions
                .first()
                .map(|r| r.before.clone())
                .unwrap_or_else(|| TaskDefinition::from_task(task));
            let mut last_plan = origin;
            for (i, receipt) in revisions.iter().enumerate() {
                ensure!(
                    receipt.previous_task_revision == i as u64
                        && receipt.before == definition
                        && receipt.plan_revision > last_plan
                        && cancellation.is_none_or(|r| receipt.plan_revision < r),
                    "DERIVATION_PLAN_DEFINITION_HISTORY_GAP"
                );
                definition = receipt.after.clone();
                last_plan = receipt.plan_revision;
            }
            ensure!(
                definition == TaskDefinition::from_task(task)
                    && task.revision == revisions.len() as u64 + u64::from(cancellation.is_some()),
                "DERIVATION_PLAN_TASK_HISTORY_MISMATCH"
            );
        }
        Ok(())
    }
}

fn unique(ids: &[String]) -> Result<()> {
    ensure!(
        ids.iter().collect::<BTreeSet<_>>().len() == ids.len(),
        "DERIVATION_PLAN_DUPLICATE_REFERENCE"
    );
    Ok(())
}
