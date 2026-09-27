use crate::{
    object_framework::{Identity, VERSION},
    object_task_storage::{self, RUN_KIND, TASK_KIND},
    object_task_types::{Granularity, RunRecord, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::HashSet;

pub(crate) fn validate_and_describe(
    connection: &Connection,
    root: &TaskRecord,
) -> Result<Vec<String>> {
    let tasks = object_task_storage::all_tasks(connection)?;
    let owned = closure(&tasks, &root.id, false);
    for task in tasks.iter().filter(|task| owned.contains(&task.id)) {
        ensure!(
            task.project_id == root.project_id,
            "OBJECT_TASK_PROJECT_MISMATCH"
        );
        ensure!(
            task.status == "planned" || task.status == "cancelled",
            "OBJECT_TASK_NOT_PLANNED: {}",
            task.id
        );
        if task.status == "planned" {
            validate_identity_and_run(connection, task)?;
        }
    }
    let affected = closure(&tasks, &root.id, true);
    let mut ids = tasks
        .iter()
        .filter(|task| affected.contains(&task.id) && task.status != "cancelled")
        .map(|task| {
            ensure!(
                task.project_id == root.project_id,
                "OBJECT_TASK_PROJECT_MISMATCH"
            );
            Ok(task.id.clone())
        })
        .collect::<Result<Vec<_>>>()?;
    ids.sort();
    Ok(ids)
}

fn closure(tasks: &[TaskRecord], root: &str, include_dependents: bool) -> HashSet<String> {
    let mut scope = HashSet::from([root.to_owned()]);
    let mut pending = vec![root.to_owned()];
    while let Some(id) = pending.pop() {
        for task in tasks {
            let related = task.parent_task_id.as_deref() == Some(id.as_str())
                || (include_dependents && task.depends_on.contains(&id));
            if related && scope.insert(task.id.clone()) {
                pending.push(task.id.clone());
            }
        }
    }
    scope
}

fn validate_identity_and_run(connection: &Connection, task: &TaskRecord) -> Result<()> {
    let matches_identity = match &task.identity {
        Identity::Coarse { schema_version } => {
            *schema_version == VERSION
                && task.granularity == Granularity::Coarse
                && task.object_id.is_none()
                && task.parent_task_id.is_none()
                && task.run_id.is_none()
                && task.stage_id.is_none()
        }
        Identity::Medium {
            schema_version,
            object_id,
            ..
        } => {
            *schema_version == VERSION
                && task.granularity == Granularity::Medium
                && task.object_id.as_ref() == Some(object_id)
                && task.stage_id.is_none()
        }
        Identity::Fine {
            schema_version,
            object_id,
            medium_task_id,
            run_id,
            stage_id,
        } => {
            *schema_version == VERSION
                && task.granularity == Granularity::Fine
                && task.object_id.as_ref() == Some(object_id)
                && task.parent_task_id.as_ref() == Some(medium_task_id)
                && task.run_id.as_ref() == Some(run_id)
                && task.stage_id.as_ref() == Some(stage_id)
        }
    };
    ensure!(matches_identity, "OBJECT_TASK_IDENTITY_MISMATCH");
    if task.granularity == Granularity::Fine {
        let parent_id = task
            .parent_task_id
            .as_deref()
            .expect("validated fine parent");
        let parent = object_task_storage::read::<TaskRecord>(connection, TASK_KIND, parent_id)?
            .context("OBJECT_TASK_PARENT_NOT_FOUND")?;
        ensure!(
            parent.id == parent_id
                && parent.project_id == task.project_id
                && parent.granularity == Granularity::Medium
                && parent.object_id == task.object_id
                && parent.run_id == task.run_id,
            "OBJECT_TASK_FINE_PARENT_MISMATCH"
        );
    }
    if task.granularity == Granularity::Coarse {
        return Ok(());
    }
    let run_id = task
        .run_id
        .as_deref()
        .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
    let run = object_task_storage::read::<RunRecord>(connection, RUN_KIND, run_id)?
        .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
    let medium_id = if task.granularity == Granularity::Medium {
        Some(task.id.as_str())
    } else {
        task.parent_task_id.as_deref()
    };
    ensure!(
        run.id == run_id
            && run.project_id == task.project_id
            && Some(&run.object_id) == task.object_id.as_ref()
            && Some(run.medium_task_id.as_str()) == medium_id,
        "OBJECT_TASK_MEDIUM_RUN_MISMATCH"
    );
    ensure!(
        run.status == "planned" && run.baseline_version_id.is_none(),
        "OBJECT_TASK_RUN_NOT_PLANNED: {run_id}"
    );
    Ok(())
}
