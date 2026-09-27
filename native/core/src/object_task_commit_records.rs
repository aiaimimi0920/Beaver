use crate::{
    object_catalog::{self, ObjectRecord},
    object_framework::{Baseline, Identity, VERSION},
    object_task_storage::{self, RUN_KIND, TASK_KIND},
    object_task_types::{Granularity, PlanProposal, RunRecord, TaskProposal, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap};

pub(crate) struct PreparedRecords {
    pub objects: Vec<ObjectRecord>,
    pub tasks: Vec<TaskRecord>,
    pub runs: Vec<RunRecord>,
    pub new_objects: Vec<ObjectRecord>,
    pub new_tasks: Vec<TaskRecord>,
    pub new_runs: Vec<RunRecord>,
}

impl PreparedRecords {
    pub fn changed(&self) -> bool {
        !self.new_objects.is_empty() || !self.new_tasks.is_empty() || !self.new_runs.is_empty()
    }
}

pub(crate) fn prepare(
    connection: &Connection,
    project_id: &str,
    plan: &PlanProposal,
) -> Result<PreparedRecords> {
    let objects = plan
        .objects
        .iter()
        .map(|item| ObjectRecord {
            id: item.id.clone(),
            project_id: project_id.to_owned(),
            name: item.name.clone(),
            category: item.category.clone(),
            tags: vec![],
            thumbnail_path: None,
            parent_object_id: None,
            revision: 0,
            components: vec![],
            files: vec![],
            references: vec![],
            versions: vec![],
        })
        .collect::<Vec<_>>();
    let mut new_objects = Vec::new();
    for object in &objects {
        if object_catalog::read(connection, &object.id)?.is_none() {
            new_objects.push(object.clone());
        }
    }

    let mut records = HashMap::new();
    let mut runs = HashMap::new();
    let mut new_tasks = Vec::new();
    let mut new_runs = Vec::new();
    for granularity in [Granularity::Coarse, Granularity::Medium, Granularity::Fine] {
        for proposal in plan
            .tasks
            .iter()
            .filter(|task| task.granularity == granularity)
        {
            let (identity, run_id) = match &proposal.granularity {
                Granularity::Coarse => (
                    Identity::Coarse {
                        schema_version: VERSION,
                    },
                    None,
                ),
                Granularity::Medium => {
                    let existing = object_task_storage::read::<TaskRecord>(
                        connection,
                        TASK_KIND,
                        &proposal.id,
                    )?;
                    let run_id = existing
                        .as_ref()
                        .and_then(|task| task.run_id.clone())
                        .unwrap_or_else(|| format!("run-{}", uuid::Uuid::new_v4()));
                    let object_id = proposal
                        .object_id
                        .as_deref()
                        .context("medium object missing")?;
                    let run =
                        object_task_storage::read::<RunRecord>(connection, RUN_KIND, &run_id)?;
                    let run = match run {
                        Some(run) => {
                            ensure!(
                                run.project_id == project_id
                                    && run.object_id == object_id
                                    && run.medium_task_id == proposal.id,
                                "OBJECT_TASK_MEDIUM_RUN_MISMATCH"
                            );
                            run
                        }
                        None => {
                            ensure!(existing.is_none(), "OBJECT_TASK_MEDIUM_RUN_MISSING");
                            let run = RunRecord {
                                id: run_id.clone(),
                                project_id: project_id.to_owned(),
                                object_id: object_id.to_owned(),
                                medium_task_id: proposal.id.clone(),
                                baseline_version_id: None,
                                status: "planned".into(),
                                revision: 0,
                            };
                            new_runs.push(run.clone());
                            run
                        }
                    };
                    runs.insert(run.id.clone(), run);
                    (
                        Identity::Medium {
                            schema_version: VERSION,
                            object_id: object_id.to_owned(),
                            baseline: proposal
                                .baseline
                                .clone()
                                .unwrap_or(Baseline::LatestAccepted {}),
                        },
                        Some(run_id),
                    )
                }
                Granularity::Fine => {
                    let parent_id = proposal
                        .parent_task_id
                        .as_deref()
                        .context("fine parent missing")?;
                    let parent = records
                        .get(parent_id)
                        .cloned()
                        .or(object_task_storage::read::<TaskRecord>(
                            connection, TASK_KIND, parent_id,
                        )?)
                        .context("OBJECT_TASK_PARENT_NOT_FOUND")?;
                    let run_id = parent
                        .run_id
                        .clone()
                        .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
                    let object_id = proposal
                        .object_id
                        .as_deref()
                        .context("fine object missing")?;
                    (
                        Identity::Fine {
                            schema_version: VERSION,
                            object_id: object_id.to_owned(),
                            medium_task_id: parent_id.to_owned(),
                            run_id: run_id.clone(),
                            stage_id: proposal.stage_id.clone().context("fine stage missing")?,
                        },
                        Some(run_id),
                    )
                }
            };
            let (record, created) = make_task(connection, project_id, proposal, identity, run_id)?;
            if created {
                new_tasks.push(record.clone());
            }
            records.insert(record.id.clone(), record);
        }
    }
    let tasks = plan
        .tasks
        .iter()
        .map(|task| {
            records
                .get(&task.id)
                .cloned()
                .context("OBJECT_TASK_RECORD_MISSING")
        })
        .collect::<Result<Vec<_>>>()?;
    let mut included_runs = BTreeMap::new();
    for task in &tasks {
        if let Some(run_id) = task.run_id.as_deref() {
            let run = runs
                .get(run_id)
                .cloned()
                .or(object_task_storage::read::<RunRecord>(
                    connection, RUN_KIND, run_id,
                )?)
                .context("OBJECT_TASK_RUN_NOT_FOUND")?;
            included_runs.insert(run.id.clone(), run);
        }
    }
    Ok(PreparedRecords {
        objects,
        tasks,
        runs: included_runs.into_values().collect(),
        new_objects,
        new_tasks,
        new_runs,
    })
}

fn make_task(
    connection: &Connection,
    project_id: &str,
    proposal: &TaskProposal,
    identity: Identity,
    run_id: Option<String>,
) -> Result<(TaskRecord, bool)> {
    if let Some(existing) =
        object_task_storage::read::<TaskRecord>(connection, TASK_KIND, &proposal.id)?
    {
        ensure!(
            existing.project_id == project_id
                && existing.granularity == proposal.granularity
                && existing.title == proposal.title
                && existing.prompt == proposal.prompt
                && existing.acceptance == proposal.acceptance
                && existing.requirement == proposal.requirement
                && existing.pending_planning == proposal.pending_planning
                && existing.object_id == proposal.object_id
                && existing.parent_task_id == proposal.parent_task_id
                && existing.depends_on == proposal.depends_on
                && existing.run_id == run_id
                && existing.stage_id == proposal.stage_id
                && existing.identity == identity,
            "OBJECT_TASK_ID_CONFLICT: {}",
            proposal.id
        );
        return Ok((existing, false));
    }
    Ok((
        TaskRecord {
            id: proposal.id.clone(),
            position: proposal.position,
            project_id: project_id.to_owned(),
            granularity: proposal.granularity.clone(),
            title: proposal.title.clone(),
            prompt: proposal.prompt.clone(),
            acceptance: proposal.acceptance.clone(),
            requirement: proposal.requirement,
            pending_planning: proposal.pending_planning.clone(),
            object_id: proposal.object_id.clone(),
            parent_task_id: proposal.parent_task_id.clone(),
            depends_on: proposal.depends_on.clone(),
            run_id,
            stage_id: proposal.stage_id.clone(),
            identity,
            status: "planned".into(),
            revision: 0,
        },
        true,
    ))
}
