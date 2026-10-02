//! Typed plan references; project-local draft, stage and assumption keys keep their meaning.
use crate::{
    object_framework::{Baseline, Identity},
    object_task_types::{Draft, PlanProposal, RunRecord, TaskRecord},
    project_derivation_copy::Request,
    project_derivation_identity::generated,
};
use anyhow::Result;

pub(crate) struct PlanIds<'a>(pub &'a Request);

impl PlanIds<'_> {
    pub fn id(&self, id: &mut String, kind: &str) -> Result<()> {
        *id = generated(self.0, kind, id)?;
        Ok(())
    }

    pub fn optional(&self, id: &mut Option<String>, kind: &str) -> Result<()> {
        if let Some(id) = id {
            self.id(id, kind)?;
        }
        Ok(())
    }

    pub fn list(&self, ids: &mut [String], kind: &str) -> Result<()> {
        for id in ids {
            self.id(id, kind)?;
        }
        Ok(())
    }

    pub fn baseline(&self, baseline: &mut Baseline) -> Result<()> {
        if let Baseline::PinnedVersion {
            selected_version_id,
        } = baseline
        {
            self.id(selected_version_id, "object_version")?;
        }
        Ok(())
    }

    pub fn provenance(&self, detail: &mut Option<String>) -> Result<()> {
        let Some(link) = detail.as_deref().and_then(|s| s.strip_prefix("planning:")) else {
            return Ok(());
        };
        let (session, suffix) = link
            .split_once("/decision:")
            .map_or((link, String::new()), |(id, decision)| {
                (id, format!("/decision:{decision}"))
            });
        *detail = Some(format!(
            "planning:{}{suffix}",
            generated(self.0, "object_planning_request", session)?
        ));
        Ok(())
    }

    pub fn plan(&self, plan: &mut PlanProposal) -> Result<()> {
        // Proposals may introduce IDs which do not yet have an entity-map entry.
        // Use exactly the same namespace as committed objects and tasks.
        for object in &mut plan.objects {
            self.id(&mut object.id, "object")?;
        }
        for task in &mut plan.tasks {
            self.id(&mut task.id, "object_task")?;
            self.optional(&mut task.object_id, "object")?;
            self.optional(&mut task.parent_task_id, "object_task")?;
            self.list(&mut task.depends_on, "object_task")?;
            if let Some(baseline) = &mut task.baseline {
                self.baseline(baseline)?;
            }
        }
        for assumption in &mut plan.assumptions {
            self.provenance(&mut assumption.source_detail)?;
        }
        Ok(())
    }

    pub fn draft(&self, draft: &mut Draft) -> Result<()> {
        draft.project_id = self.0.target_project_id.clone();
        self.optional(&mut draft.committed_request_id, "object_task_request")?;
        self.plan(&mut draft.plan)
    }

    pub fn run(&self, run: &mut RunRecord) -> Result<()> {
        run.project_id = self.0.target_project_id.clone();
        self.id(&mut run.id, "object_run")?;
        self.id(&mut run.object_id, "object")?;
        self.id(&mut run.medium_task_id, "object_task")?;
        self.optional(&mut run.baseline_version_id, "object_version")
    }

    pub fn task(&self, task: &mut TaskRecord) -> Result<()> {
        task.project_id = self.0.target_project_id.clone();
        self.id(&mut task.id, "object_task")?;
        self.optional(&mut task.object_id, "object")?;
        self.optional(&mut task.parent_task_id, "object_task")?;
        self.optional(&mut task.run_id, "object_run")?;
        self.list(&mut task.depends_on, "object_task")?;
        match &mut task.identity {
            Identity::Coarse { .. } => {}
            Identity::Medium {
                object_id,
                baseline,
                ..
            } => {
                self.id(object_id, "object")?;
                self.baseline(baseline)?;
            }
            Identity::Fine {
                object_id,
                medium_task_id,
                run_id,
                ..
            } => {
                self.id(object_id, "object")?;
                self.id(medium_task_id, "object_task")?;
                self.id(run_id, "object_run")?;
            }
        }
        Ok(())
    }
}
