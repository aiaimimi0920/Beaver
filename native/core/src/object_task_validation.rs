use crate::{
    object_catalog,
    object_task_storage::{self, RUN_KIND, TASK_KIND},
    object_task_types::{self, Granularity, PlanProposal, RunRecord, TaskProposal, TaskRecord},
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) fn validate_plan(
    connection: &Connection,
    project_id: &str,
    plan: &PlanProposal,
    allow_empty: bool,
) -> Result<()> {
    object_task_types::validate_plan(plan, allow_empty)?;
    let objects: HashSet<_> = plan.objects.iter().map(|item| item.id.as_str()).collect();
    let tasks: HashMap<_, _> = plan
        .tasks
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    for id in &objects {
        ensure!(!tasks.contains_key(id), "OBJECT_TASK_ID_COLLISION: {id}");
        if let Some(record) = object_catalog::read(connection, id)? {
            ensure!(
                record.project_id == project_id,
                "OBJECT_TASK_OBJECT_PROJECT_MISMATCH"
            );
            let proposal = plan.objects.iter().find(|item| item.id == *id).unwrap();
            ensure!(
                record.name == proposal.name && record.category == proposal.category,
                "OBJECT_TASK_OBJECT_ID_CONFLICT: {id}"
            );
        }
        ensure!(
            object_task_storage::read::<TaskRecord>(connection, TASK_KIND, id)?.is_none(),
            "OBJECT_TASK_ID_COLLISION: {id}"
        );
    }
    for task in &plan.tasks {
        ensure!(
            object_catalog::read(connection, &task.id)?.is_none(),
            "OBJECT_TASK_ID_COLLISION: {}",
            task.id
        );
        if read_task(connection, project_id, &task.id)?.is_none() {
            validate_available_references(connection, project_id, task)?;
        }
        validate_identity(connection, project_id, task, &tasks, &objects)?;
        for dependency in &task.depends_on {
            ensure!(
                dependency != &task.id,
                "OBJECT_TASK_SELF_DEPENDENCY: {}",
                task.id
            );
            ensure!(
                tasks.contains_key(dependency.as_str())
                    || read_task(connection, project_id, dependency)?.is_some(),
                "OBJECT_TASK_DEPENDENCY_NOT_FOUND: {dependency}"
            );
        }
    }
    validate_dependency_graph(connection, project_id, plan)?;
    Ok(())
}

pub(crate) fn validate_available_references(
    connection: &Connection,
    project_id: &str,
    task: &TaskProposal,
) -> Result<()> {
    if let Some(parent_id) = &task.parent_task_id {
        if let Some(parent) = read_task(connection, project_id, parent_id)? {
            ensure!(
                parent.status != "cancelled",
                "OBJECT_TASK_PARENT_CANCELLED: {parent_id}"
            );
            if let Some(run_id) = &parent.run_id {
                let run = object_task_storage::read::<RunRecord>(connection, RUN_KIND, run_id)?
                    .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
                ensure!(
                    run.status != "cancelled",
                    "OBJECT_TASK_MEDIUM_RUN_CANCELLED: {run_id}"
                );
            }
        }
    }
    for dependency in &task.depends_on {
        if let Some(record) = read_task(connection, project_id, dependency)? {
            ensure!(
                record.status != "cancelled",
                "OBJECT_TASK_DEPENDENCY_CANCELLED: {dependency}"
            );
        }
    }
    Ok(())
}

fn validate_identity(
    connection: &Connection,
    project_id: &str,
    task: &TaskProposal,
    planned_tasks: &HashMap<&str, &TaskProposal>,
    planned_objects: &HashSet<&str>,
) -> Result<()> {
    match &task.granularity {
        Granularity::Coarse => ensure!(
            task.parent_task_id.is_none(),
            "INVALID_OBJECT_TASK_IDENTITY: coarse parent"
        ),
        Granularity::Medium => {
            let object_id = task.object_id.as_deref().context("medium object missing")?;
            ensure!(
                planned_objects.contains(object_id)
                    || object_catalog::read(connection, object_id)?
                        .is_some_and(|record| record.project_id == project_id),
                "OBJECT_TASK_OBJECT_NOT_FOUND: {object_id}"
            );
            if let Some(crate::object_framework::Baseline::PinnedVersion {
                selected_version_id,
            }) = &task.baseline
            {
                let object =
                    object_catalog::read(connection, object_id)?.context("OBJECT_NOT_FOUND")?;
                ensure!(
                    object.project_id == project_id,
                    "OBJECT_TASK_OBJECT_PROJECT_MISMATCH"
                );
                let version = object
                    .versions
                    .iter()
                    .find(|version| &version.version_id == selected_version_id)
                    .context("OBJECT_VERSION_NOT_FOUND")?;
                crate::object_version_manifest::read(&object, version)?;
            }
            if let Some(parent_id) = &task.parent_task_id {
                match planned_tasks.get(parent_id.as_str()) {
                    Some(parent) => ensure!(
                        parent.granularity == Granularity::Coarse,
                        "OBJECT_TASK_PARENT_GRANULARITY: {}",
                        task.id
                    ),
                    None => {
                        let parent = read_task(connection, project_id, parent_id)?
                            .context("OBJECT_TASK_PARENT_NOT_FOUND")?;
                        ensure!(
                            parent.granularity == Granularity::Coarse,
                            "OBJECT_TASK_PARENT_GRANULARITY: {}",
                            task.id
                        );
                    }
                }
            }
        }
        Granularity::Fine => {
            let parent_id = task
                .parent_task_id
                .as_deref()
                .context("fine parent missing")?;
            let object_id = task.object_id.as_deref().context("fine object missing")?;
            if let Some(parent) = planned_tasks.get(parent_id) {
                ensure!(
                    parent.granularity == Granularity::Medium
                        && parent.object_id.as_deref() == Some(object_id),
                    "OBJECT_TASK_FINE_PARENT_MISMATCH: {}",
                    task.id
                );
            } else {
                let parent = read_task(connection, project_id, parent_id)?
                    .context("OBJECT_TASK_PARENT_NOT_FOUND")?;
                ensure!(
                    parent.granularity == Granularity::Medium
                        && parent.object_id.as_deref() == Some(object_id),
                    "OBJECT_TASK_FINE_PARENT_MISMATCH: {}",
                    task.id
                );
                let run_id = parent
                    .run_id
                    .as_deref()
                    .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
                let run = object_task_storage::read::<RunRecord>(connection, RUN_KIND, run_id)?
                    .context("OBJECT_TASK_MEDIUM_RUN_MISSING")?;
                ensure!(
                    run.project_id == project_id
                        && run.object_id == object_id
                        && run.medium_task_id == parent_id,
                    "OBJECT_TASK_MEDIUM_RUN_MISMATCH"
                );
            }
            ensure!(
                planned_objects.contains(object_id)
                    || object_catalog::read(connection, object_id)?
                        .is_some_and(|record| record.project_id == project_id),
                "OBJECT_TASK_OBJECT_NOT_FOUND: {object_id}"
            );
        }
    }
    Ok(())
}

fn read_task(connection: &Connection, project_id: &str, id: &str) -> Result<Option<TaskRecord>> {
    let task = object_task_storage::read::<TaskRecord>(connection, TASK_KIND, id)?;
    if let Some(task) = &task {
        ensure!(task.id == id, "OBJECT_TASK_IDENTITY_MISMATCH");
        ensure!(
            task.project_id == project_id,
            "OBJECT_TASK_PROJECT_MISMATCH"
        );
    }
    Ok(task)
}

fn validate_dependency_graph(
    connection: &Connection,
    project_id: &str,
    plan: &PlanProposal,
) -> Result<()> {
    let mut dependencies: HashMap<String, Vec<String>> =
        object_task_storage::all_tasks(connection)?
            .into_iter()
            .filter(|task| task.project_id == project_id)
            .map(|task| (task.id, task.depends_on))
            .collect();
    for task in &plan.tasks {
        dependencies.insert(task.id.clone(), task.depends_on.clone());
    }
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut remaining = HashMap::new();
    for (id, required) in &dependencies {
        remaining.insert(id.as_str(), required.len());
        for dependency in required {
            ensure!(
                dependencies.contains_key(dependency),
                "OBJECT_TASK_DEPENDENCY_NOT_FOUND: {dependency}"
            );
            dependents.entry(dependency).or_default().push(id);
        }
    }
    let mut ready: VecDeque<_> = remaining
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        for dependent in dependents.get(id).into_iter().flatten() {
            let count = remaining.get_mut(dependent).expect("known dependent");
            *count -= 1;
            if *count == 0 {
                ready.push_back(dependent);
            }
        }
    }
    ensure!(
        visited == dependencies.len(),
        "OBJECT_TASK_DEPENDENCY_CYCLE"
    );
    Ok(())
}
