use crate::{
    files::Files,
    scheduler::ParallelLimit,
    scheduler_runtime::{RuntimeOwner, TaskRuntime},
    store::Store,
};
use serde_json::{json, Value};
use std::collections::HashSet;
#[cfg(test)]
use std::sync::Mutex;

#[derive(Clone)]
pub(crate) struct ClaimedTask {
    pub(crate) task: Value,
    pub(crate) runtime: TaskRuntime,
}

#[cfg(test)]
pub(crate) fn claim_next(
    store: &Mutex<Store>,
    files: &Files,
    active: usize,
    parallel_limit: &ParallelLimit,
) -> Result<Vec<Value>, String> {
    let limit = parallel_limit()?.clamp(1, 6);
    let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
    claim(&mut store, files, active, limit).map_err(|error| error.to_string())
}

#[cfg(test)]
fn claim(
    store: &mut Store,
    files: &Files,
    active: usize,
    limit: usize,
) -> anyhow::Result<Vec<Value>> {
    crate::task_plan::reconcile(store, files)?;
    let tasks: Vec<Value> = store.list("task")?;
    let count = tasks
        .iter()
        .filter(|task| task["status"] == "running")
        .count()
        .max(active);
    let selected = tasks
        .iter()
        .rev()
        .filter(|task| {
            task["status"] == "queued"
                && !task["waitingDelivery"].is_string()
                && crate::task_plan::eligible(task, &tasks)
        })
        .take(limit.saturating_sub(count))
        .cloned()
        .collect();
    prepare_claims(store, files, selected)
}

#[cfg(test)]
pub(crate) fn claim_next_runtimes(
    runtimes: &[TaskRuntime],
    active: usize,
    parallel_limit: &ParallelLimit,
) -> Result<Vec<ClaimedTask>, String> {
    claim_batch(runtimes, active, parallel_limit).map(|(claimed, _)| claimed)
}

pub(crate) fn claim_batch(
    runtimes: &[TaskRuntime],
    active: usize,
    parallel_limit: &ParallelLimit,
) -> Result<(Vec<ClaimedTask>, usize), String> {
    let limit = parallel_limit()?.clamp(1, 6);
    let mut snapshots = Vec::new();
    let mut object_running = 0;
    let mut object_projects = HashSet::new();
    for runtime in runtimes {
        let store = runtime.store();
        let store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
        if let Some(project) = &runtime.project {
            if object_projects.insert(project.project_id().to_owned()) {
                object_running += store
                    .list::<crate::object_task_queue::QueueEntry>(
                        crate::object_task_queue::QUEUE_KIND,
                    )
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .filter(|entry| {
                        entry.project_id == project.project_id() && entry.state == "running"
                    })
                    .count();
            }
        }
        let tasks: Vec<Value> = store.list("task").map_err(|error| error.to_string())?;
        snapshots.push((runtime.clone(), tasks));
    }
    snapshots.sort_by_key(|(runtime, _)| matches!(runtime.owner(), RuntimeOwner::Host));
    let project_ids = project_ids(&snapshots);
    let project_task_ids = project_task_ids(&snapshots);
    for (runtime, tasks) in &mut snapshots {
        let permit = runtime
            .work_gate
            .acquire()
            .map_err(|error| error.to_string())?;
        let store = runtime.store();
        let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
        let registered: HashSet<String> = store
            .list_with_ids::<Value>("project")
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let owns = |task: &Value| {
            authoritative_task(runtime, task, &project_ids, &project_task_ids)
                && (matches!(runtime.owner(), RuntimeOwner::Project(_))
                    || task["projectId"]
                        .as_str()
                        .is_some_and(|id| registered.contains(id)))
        };
        if !runtime.draining && permit.is_some() {
            crate::task_plan::reconcile_matching(&mut store, &runtime.files(), owns)
                .map_err(|error| error.to_string())?;
        }
        *tasks = store
            .list::<Value>("task")
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(owns)
            .collect();
    }
    let running = snapshots
        .iter()
        .map(|(runtime, tasks)| {
            tasks
                .iter()
                .filter(|task| {
                    authoritative_task(runtime, task, &project_ids, &project_task_ids)
                        && task["status"] == "running"
                })
                .count()
        })
        .sum::<usize>();
    let mut remaining = limit.saturating_sub((running + object_running).max(active));
    let mut claimed = Vec::new();
    let mut claimed_ids = HashSet::new();
    for (runtime, tasks) in snapshots {
        if runtime.draining {
            continue;
        }
        if remaining == 0 {
            break;
        }
        let mut selected = Vec::new();
        for task in tasks.iter().rev() {
            if selected.len() >= remaining {
                break;
            }
            let Some(id) = task["id"].as_str() else {
                continue;
            };
            if claimed_ids.contains(id) {
                continue;
            }
            if !authoritative_task(&runtime, task, &project_ids, &project_task_ids) {
                continue;
            }
            if task["status"] == "queued"
                && !task["waitingDelivery"].is_string()
                && crate::task_plan::eligible(task, &tasks)
            {
                claimed_ids.insert(id.to_owned());
                selected.push(task.clone());
            }
        }
        if selected.is_empty() {
            continue;
        }
        let Some(_permit) = runtime
            .work_gate
            .acquire()
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        let store = runtime.store();
        let files = runtime.files();
        let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
        let selected = revalidate_claims(&store, runtime.owner(), selected)
            .map_err(|error| error.to_string())?;
        let prepared =
            prepare_claims(&mut store, &files, selected).map_err(|error| error.to_string())?;
        remaining = remaining.saturating_sub(prepared.len());
        for task in prepared {
            if let Some(id) = task["id"].as_str() {
                claimed_ids.insert(id.to_owned());
            }
            claimed.push(ClaimedTask {
                task,
                runtime: runtime.clone(),
            });
        }
    }
    Ok((claimed, remaining))
}

fn project_ids(snapshots: &[(TaskRuntime, Vec<Value>)]) -> HashSet<String> {
    snapshots
        .iter()
        .filter_map(|(runtime, _)| match runtime.owner() {
            RuntimeOwner::Project(project_id) => Some(project_id.clone()),
            RuntimeOwner::Host => None,
        })
        .collect()
}

fn project_task_ids(snapshots: &[(TaskRuntime, Vec<Value>)]) -> HashSet<String> {
    snapshots
        .iter()
        .filter(|(runtime, _)| matches!(runtime.owner(), RuntimeOwner::Project(_)))
        .flat_map(|(_, tasks)| tasks.iter())
        .filter_map(|task| task["id"].as_str().map(str::to_owned))
        .collect()
}

fn authoritative_task(
    runtime: &TaskRuntime,
    task: &Value,
    project_ids: &HashSet<String>,
    project_task_ids: &HashSet<String>,
) -> bool {
    match runtime.owner() {
        RuntimeOwner::Project(_) => true,
        RuntimeOwner::Host => {
            !task["projectId"]
                .as_str()
                .is_some_and(|project_id| project_ids.contains(project_id))
                && !task["id"]
                    .as_str()
                    .is_some_and(|task_id| project_task_ids.contains(task_id))
        }
    }
}

fn revalidate_claims(
    store: &Store,
    owner: &RuntimeOwner,
    selected: Vec<Value>,
) -> anyhow::Result<Vec<Value>> {
    let tasks: Vec<Value> = store.list("task")?;
    let mut current = Vec::new();
    for snapshot in selected {
        let Some(task) = tasks.iter().find(|task| task["id"] == snapshot["id"]) else {
            continue;
        };
        if task["projectId"] != snapshot["projectId"]
            || task["status"] != "queued"
            || task["waitingDelivery"].is_string()
            || !crate::task_plan::eligible(task, &tasks)
        {
            continue;
        }
        let Some(project_id) = task["projectId"].as_str() else {
            continue;
        };
        match owner {
            RuntimeOwner::Host if store.get::<Value>("project", project_id)?.is_none() => continue,
            RuntimeOwner::Project(id) if id != project_id => continue,
            _ => {}
        }
        current.push(task.clone());
    }
    Ok(current)
}

fn prepare_claims(
    store: &mut Store,
    files: &Files,
    selected: Vec<Value>,
) -> anyhow::Result<Vec<Value>> {
    let mut prepared = Vec::new();
    for mut task in selected {
        if let Err(error) = crate::task_plan::prepare(store, files, &mut task) {
            task["status"] = json!("failed");
            task["error"] = json!(error.to_string());
            store.put("task", task["id"].as_str().unwrap(), &task)?;
        } else {
            prepared.push(task);
        }
    }
    store.transaction(|db| {
        let mut result = Vec::new();
        for mut task in prepared {
            let id = task["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("任务标识无效"))?
                .to_string();
            task["status"] = json!("running");
            task.as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("任务格式无效"))?
                .remove("error");
            task["updatedAt"] =
                json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            db.execute(
                "UPDATE entities SET value=? WHERE kind='task' AND id=?",
                rusqlite::params![task.to_string(), id],
            )?;
            result.push(task);
        }
        Ok(result)
    })
}

fn interrupt_queued(
    store: &mut Store,
    id: Option<&str>,
    blocked_projects: &HashSet<String>,
) -> Result<(), String> {
    let tasks: Vec<Value> = store.list("task").map_err(|error| error.to_string())?;
    for mut task in tasks {
        let task_id = task["id"].as_str().ok_or("任务标识无效")?.to_string();
        let blocked = task["projectId"]
            .as_str()
            .is_some_and(|project_id| blocked_projects.contains(project_id));
        if !blocked && task["status"] == "queued" && id.is_none_or(|expected| expected == task_id) {
            task["status"] = json!("interrupted");
            store
                .put("task", &task_id, &task)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn runtime_for_existing_task(
    runtimes: &[TaskRuntime],
    id: &str,
) -> Result<Option<TaskRuntime>, String> {
    let project_ids = runtimes
        .iter()
        .filter_map(|runtime| match runtime.owner() {
            RuntimeOwner::Project(project_id) => Some(project_id.clone()),
            RuntimeOwner::Host => None,
        })
        .collect::<HashSet<_>>();
    for runtime in runtimes
        .iter()
        .filter(|runtime| matches!(runtime.owner(), RuntimeOwner::Project(_)))
    {
        let store = runtime.store();
        let found = store
            .lock()
            .map_err(|_| "数据库锁不可用".to_string())?
            .get::<Value>("task", id)
            .map_err(|error| error.to_string())?
            .is_some();
        if found {
            return Ok(Some(runtime.clone()));
        }
    }
    for runtime in runtimes
        .iter()
        .filter(|runtime| matches!(runtime.owner(), RuntimeOwner::Host))
    {
        let store = runtime.store();
        let task = store
            .lock()
            .map_err(|_| "数据库锁不可用".to_string())?
            .get::<Value>("task", id)
            .map_err(|error| error.to_string())?;
        if let Some(task) = task {
            if task["projectId"]
                .as_str()
                .is_some_and(|project_id| project_ids.contains(project_id))
            {
                continue;
            }
            return Ok(Some(runtime.clone()));
        }
    }
    Ok(None)
}

pub(crate) fn interrupt_task(
    runtimes: &[TaskRuntime],
    id: &str,
) -> Result<Option<TaskRuntime>, String> {
    let Some(runtime) = runtime_for_existing_task(runtimes, id)? else {
        return Ok(None);
    };
    let store = runtime.store();
    let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
    interrupt_queued(&mut store, Some(id), &HashSet::new())?;
    Ok(Some(runtime))
}

pub(crate) fn interrupt_all(runtimes: &[TaskRuntime]) -> Result<(), String> {
    let project_ids = runtimes
        .iter()
        .filter_map(|runtime| match runtime.owner() {
            RuntimeOwner::Project(project_id) => Some(project_id.clone()),
            RuntimeOwner::Host => None,
        })
        .collect::<HashSet<_>>();
    let empty_projects = HashSet::new();
    for runtime in runtimes {
        let store = runtime.store();
        let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
        let blocked_projects = if matches!(runtime.owner(), RuntimeOwner::Host) {
            &project_ids
        } else {
            &empty_projects
        };
        interrupt_queued(&mut store, None, blocked_projects)?;
    }
    Ok(())
}

pub(crate) async fn close_sessions(sessions: &crate::asset_sessions::Sessions) {
    for id in sessions.ids() {
        sessions.close(&id).await;
    }
}

#[cfg(test)]
#[path = "scheduler_draining_tests.rs"]
mod draining_tests;

#[cfg(test)]
#[path = "scheduler_claim_revalidation_tests.rs"]
mod claim_revalidation_tests;

#[cfg(test)]
#[path = "scheduler_registration_gate_tests.rs"]
mod registration_gate_tests;
