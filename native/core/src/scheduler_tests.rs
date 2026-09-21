use super::*;
use crate::{
    project_storage::ProjectStore, scheduler_runtime::RuntimeOwner,
    scheduler_runtime_ops::claim_next,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn dropping_last_handle_interrupts_unstarted_queue() {
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(Mutex::new(Store::open(temp.path()).unwrap()));
    store
        .lock()
        .unwrap()
        .put("task", "t", &json!({"id":"t","status":"queued"}))
        .unwrap();
    let scheduler = Scheduler::start(
        store.clone(),
        Arc::new(Files::new(temp.path().into())),
        Arc::new(|_| panic!("closed scheduler must not start tasks")),
        Arc::new(|| {}),
        Arc::new(|| Ok(2)),
    );
    drop(scheduler);
    tokio::task::yield_now().await;
    assert_eq!(
        store
            .lock()
            .unwrap()
            .get::<Value>("task", "t")
            .unwrap()
            .unwrap()["status"],
        "interrupted"
    );
}

fn queued_store(path: &std::path::Path) -> Result<Arc<Mutex<Store>>> {
    let store = Store::open(path)?;
    for id in ["first", "second", "third"] {
        store.put(
            "task",
            id,
            &json!({"id":id,"status":"queued","workspacePrepared":true}),
        )?;
    }
    Ok(Arc::new(Mutex::new(store)))
}

fn project_root(parent: &Path, project_id: &str) -> Result<PathBuf> {
    let root = parent.join(project_id);
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("project.godot"), "config_version=5\n")?;
    Ok(root)
}

fn project_runtime(parent: &Path, project_id: &str) -> Result<TaskRuntime> {
    let root = project_root(parent, project_id)?;
    let runtime = ProjectStore::initialize(&root, project_id)?.into_runtime();
    Ok(TaskRuntime::project(
        project_id,
        runtime.store(),
        runtime.files(),
    ))
}

fn host_runtime(root: &Path) -> Result<TaskRuntime> {
    Ok(TaskRuntime::host(
        Arc::new(Mutex::new(Store::open(root)?)),
        Arc::new(Files::new(root.to_path_buf())),
    ))
}

fn failed_launch() -> Launch {
    Launch {
        command: None,
        model: String::new(),
        prompt: String::new(),
        ask_user_tool: Value::Null,
        secrets: vec![],
        max_minutes: 1,
        blender: None,
        godot: None,
    }
}

fn queued_project_task(id: &str, project_id: &str) -> Value {
    json!({
        "id": id,
        "projectId": project_id,
        "status": "queued",
        "workspacePrepared": true
    })
}

async fn wait_for_status(store: Arc<Mutex<Store>>, id: &str, expected: &str) -> Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let task = store.lock().unwrap().get::<Value>("task", id).unwrap();
            if let Some(task) = task {
                if task["status"] == expected {
                    break task;
                }
            }
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("scheduler did not finish task")
}

#[test]
fn host_capacity_wins_and_includes_active_workers() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"maxParallel":2}))?;
    let tasks = queued_store(&temp.path().join("tasks"))?;
    tasks
        .lock()
        .unwrap()
        .put("settings", "main", &json!({"maxParallel":6}))?;
    let source = host.clone();
    let limit: ParallelLimit = Arc::new(move || {
        crate::execution_settings::parallel_limit(&source.lock().unwrap())
            .map_err(|e| e.to_string())
    });
    let files = Files::new(temp.path().join("tasks"));
    // One worker is still finalizing even though its task is no longer marked running.
    let claimed = claim_next(&tasks, &files, 1, &limit).map_err(anyhow::Error::msg)?;
    assert_eq!(claimed.len(), 1);
    assert!(claim_next(&tasks, &files, 2, &limit)
        .map_err(anyhow::Error::msg)?
        .is_empty());
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"maxParallel":1}))?;
    assert!(claim_next(&tasks, &files, 0, &limit)
        .map_err(anyhow::Error::msg)?
        .is_empty());
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"maxParallel":3}))?;
    assert_eq!(
        claim_next(&tasks, &files, 0, &limit)
            .map_err(anyhow::Error::msg)?
            .len(),
        2
    );
    Ok(())
}

#[test]
fn capacity_is_resolved_before_locking_task_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let tasks = queued_store(temp.path())?;
    let shared = tasks.clone();
    let limit: ParallelLimit = Arc::new(move || {
        let store = shared
            .try_lock()
            .map_err(|_| "task store is already locked".to_string())?;
        crate::execution_settings::parallel_limit(&store).map_err(|e| e.to_string())
    });
    let claimed = claim_next(&tasks, &Files::new(temp.path().into()), 0, &limit)
        .map_err(anyhow::Error::msg)?;
    assert_eq!(claimed.len(), 2);
    Ok(())
}

#[test]
fn capacity_failure_does_not_claim_or_modify_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let tasks = queued_store(temp.path())?;
    let before: Vec<Value> = tasks.lock().unwrap().list("task")?;
    let limit: ParallelLimit = Arc::new(|| Err("host unavailable".into()));
    assert_eq!(
        claim_next(&tasks, &Files::new(temp.path().into()), 0, &limit).unwrap_err(),
        "host unavailable"
    );
    assert_eq!(tasks.lock().unwrap().list::<Value>("task")?, before);
    Ok(())
}

#[path = "scheduler_claim_tests.rs"]
mod claims;

#[test]
fn interrupt_queued_project_task_before_matching_host_duplicate() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = project_runtime(temp.path(), "project-a")?;
    project.store().lock().unwrap().put(
        "task",
        "shared",
        &json!({"id":"shared","projectId":"project-a","status":"queued"}),
    )?;
    let host = host_runtime(&temp.path().join("host"))?;
    let host_store = host.store();
    host_store.lock().unwrap().put(
        "task",
        "shared",
        &json!({"id":"shared","projectId":"legacy","status":"queued"}),
    )?;

    let interrupted = interrupt_task(&[project.clone(), host], "shared")
        .map_err(anyhow::Error::msg)?
        .expect("project task should be found first");

    assert!(matches!(
        interrupted.owner(),
        RuntimeOwner::Project(project_id) if project_id == "project-a"
    ));
    assert_eq!(
        project
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "shared")?
            .unwrap()["status"],
        "interrupted"
    );
    assert_eq!(
        host_store
            .lock()
            .unwrap()
            .get::<Value>("task", "shared")?
            .unwrap()["status"],
        "queued"
    );
    Ok(())
}

#[test]
fn missing_task_does_not_select_a_runtime_for_session_cleanup() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_runtime(&temp.path().join("host"))?;
    assert!(
        crate::scheduler_runtime_ops::runtime_for_existing_task(&[host], "missing")
            .map_err(anyhow::Error::msg)?
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn scheduler_discovers_project_runtime_after_wake_without_host_fallback() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_runtime(&temp.path().join("host"))?;
    let project = project_runtime(temp.path(), "project-a")?;
    let project_store = project.store();
    project_store.lock().unwrap().put(
        "task",
        "project-task",
        &queued_project_task("project-task", "project-a"),
    )?;
    let visible = Arc::new(Mutex::new(vec![host.clone()]));
    let source = visible.clone();
    let owners = Arc::new(Mutex::new(Vec::<RuntimeOwner>::new()));
    let factory_owners = owners.clone();
    let scheduler = Scheduler::start_with_runtimes(
        Arc::new(move || Ok(source.lock().unwrap().clone())),
        Arc::new(move |_, runtime| {
            factory_owners.lock().unwrap().push(runtime.owner().clone());
            Ok(failed_launch())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    );

    host.store().lock().unwrap().put(
        "task",
        "project-task",
        &json!({
            "id": "project-task",
            "projectId": "project-a",
            "status": "queued",
            "workspacePrepared": true
        }),
    )?;
    visible.lock().unwrap().push(project.clone());
    scheduler.wake().map_err(anyhow::Error::msg)?;
    let task = wait_for_status(project_store.clone(), "project-task", "failed").await;
    assert_eq!(task["status"], "failed");
    assert_eq!(
        host.store()
            .lock()
            .unwrap()
            .get::<Value>("task", "project-task")?
            .unwrap()["status"],
        "queued"
    );
    assert!(owners
        .lock()
        .unwrap()
        .iter()
        .any(|owner| matches!(owner, RuntimeOwner::Project(id) if id == "project-a")));
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

#[tokio::test]
async fn completed_project_task_finishes_only_in_project_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_runtime(&temp.path().join("host"))?;
    let project = project_runtime(temp.path(), "project-a")?;
    let project_root = temp.path().join("project-a");
    project.store().lock().unwrap().put(
        "project",
        "project-a",
        &json!({"id":"project-a","path":project_root}),
    )?;
    let project_store = project.store();
    project_store.lock().unwrap().put(
        "task",
        "project-task",
        &json!({
            "id":"project-task","projectId":"project-a","status":"queued",
            "workspacePrepared":true,"validationOnly":true,"changes":[],"baseline":{},
            "capability":"code","validationVersion":1,"autoAccept":false
        }),
    )?;
    let host_store = host.store();
    host_store.lock().unwrap().put(
        "task",
        "project-task",
        &json!({
            "id":"project-task","projectId":"project-a","status":"queued",
            "workspacePrepared":true
        }),
    )?;
    let visible = vec![host.clone(), project.clone()];
    let scheduler = Scheduler::start_with_runtimes(
        Arc::new(move || Ok(visible.clone())),
        Arc::new(move |_, _| Ok(failed_launch())),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    );

    scheduler.wake().map_err(anyhow::Error::msg)?;
    let task = wait_for_status(project_store.clone(), "project-task", "completed").await;
    assert_eq!(task["projectId"], "project-a");
    assert_eq!(
        host_store
            .lock()
            .unwrap()
            .get::<Value>("task", "project-task")?
            .unwrap()["status"],
        "queued"
    );
    let events = project_store.lock().unwrap().events("project-task")?;
    assert!(events.iter().any(|event| event.kind == "status"));
    assert!(project_store
        .lock()
        .unwrap()
        .list::<Value>("operation")?
        .iter()
        .all(|operation| operation["projectId"] == "project-a"));
    assert!(host_store
        .lock()
        .unwrap()
        .list::<Value>("operation")?
        .is_empty());
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

#[tokio::test]
async fn active_project_task_finishes_in_original_store_after_runtime_disappears() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_runtime(&temp.path().join("host"))?;
    let project = project_runtime(temp.path(), "project-a")?;
    let project_store = project.store();
    project_store.lock().unwrap().put(
        "task",
        "project-task",
        &queued_project_task("project-task", "project-a"),
    )?;
    let visible = Arc::new(Mutex::new(vec![host, project.clone()]));
    let source = visible.clone();
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let factory_entered = entered.clone();
    let factory_release = release.clone();
    let scheduler = Scheduler::start_with_runtimes(
        Arc::new(move || Ok(source.lock().unwrap().clone())),
        Arc::new(move |_, _| {
            factory_entered.store(true, Ordering::SeqCst);
            while !factory_release.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
            Ok(failed_launch())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    );

    scheduler.wake().map_err(anyhow::Error::msg)?;
    timeout(Duration::from_secs(5), async {
        while !entered.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("scheduler did not start project task");
    visible.lock().unwrap().retain(
        |runtime| !matches!(runtime.owner(), RuntimeOwner::Project(id) if id == "project-a"),
    );
    release.store(true, Ordering::SeqCst);

    let task = wait_for_status(project_store.clone(), "project-task", "failed").await;
    assert_eq!(task["status"], "failed");
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    Ok(())
}

#[tokio::test]
async fn runtime_source_failure_does_not_leave_active_worker_running() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_runtime(&temp.path().join("host"))?;
    let project = project_runtime(temp.path(), "project-a")?;
    let project_store = project.store();
    project_store.lock().unwrap().put(
        "task",
        "project-task",
        &queued_project_task("project-task", "project-a"),
    )?;
    let visible = Arc::new(Mutex::new(vec![host, project.clone()]));
    let source_state = Arc::new((AtomicBool::new(false), AtomicBool::new(false)));
    let source = visible.clone();
    let source_state_for_runtime = source_state.clone();
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let factory_entered = entered.clone();
    let factory_release = release.clone();
    let scheduler = Scheduler::start_with_runtimes(
        Arc::new(move || {
            if source_state_for_runtime.0.load(Ordering::SeqCst) {
                source_state_for_runtime.1.store(true, Ordering::SeqCst);
                Err("runtime source failed".into())
            } else {
                Ok(source.lock().unwrap().clone())
            }
        }),
        Arc::new(move |_, _| {
            factory_entered.store(true, Ordering::SeqCst);
            while !factory_release.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
            Ok(failed_launch())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    );

    scheduler.wake().map_err(anyhow::Error::msg)?;
    timeout(Duration::from_secs(5), async {
        while !entered.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("scheduler did not start project task");
    source_state.0.store(true, Ordering::SeqCst);
    let shutdown = {
        let scheduler = scheduler.clone();
        tokio::spawn(async move { scheduler.shutdown().await })
    };
    timeout(Duration::from_secs(5), async {
        while !source_state.1.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("scheduler did not observe runtime source failure");
    release.store(true, Ordering::SeqCst);
    let result = shutdown.await?;
    assert_eq!(result.unwrap_err(), "runtime source failed");
    let task = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", "project-task")?
        .unwrap();
    assert_ne!(task["status"], "running");
    Ok(())
}
