use super::*;
use std::{
    sync::{mpsc, TryLockError},
    thread,
    time::{Duration, Instant},
};

#[test]
fn task_create_releases_store_before_indexing_during_registry_refresh() -> anyhow::Result<()> {
    create_during_registry_refresh(false)
}

#[test]
fn feature_add_releases_store_before_indexing_during_registry_refresh() -> anyhow::Result<()> {
    create_during_registry_refresh(true)
}

fn create_during_registry_refresh(feature: bool) -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    let other_root = project_root(temp.path(), "project-b")?;
    let other = register_project(&host, "project-b", &other_root)?;
    initialize_project_store(&other_root, "project-b", &other)?;
    let router = Arc::new(router);
    let store_handle = runtime.store();
    let (created_tx, created_rx) = mpsc::channel();
    let (registry_tx, registry_rx) = mpsc::channel();
    let (refreshed_tx, refreshed_rx) = mpsc::channel();
    let timeout = Duration::from_secs(10);

    // Hold a separate SQLite write transaction so task creation cannot finish
    // before the registry-owning refresh reaches the project Store lock.
    let mut blocker = Store::open(&temp.path().join("sqlite-blocker"))?;
    blocker.transaction(|connection| {
        connection.execute(
            "ATTACH DATABASE ?1 AS project",
            [root
                .join(".beaver/project.sqlite")
                .to_string_lossy()
                .as_ref()],
        )?;
        connection.execute("UPDATE project.entities SET value = value WHERE 0", [])?;
        let writer_router = router.clone();
        let writer_host = host.clone();
        let writer_root = host_root(temp.path());
        thread::spawn(move || {
            let result = if feature {
                feature_add_operation(
                    &writer_router,
                    writer_host,
                    &writer_root,
                    Some(json!({"projectId":"project-a","featureId":"crafting"})),
                )
            } else {
                create_project_task(
                    &writer_router,
                    Some(json!({"projectId":"project-a","prompt":"并发创建任务"})),
                )
            };
            let _ = created_tx.send(result);
        });
        let deadline = Instant::now() + timeout;
        loop {
            match store_handle.try_lock() {
                Err(TryLockError::WouldBlock) => break,
                Err(TryLockError::Poisoned(_)) => anyhow::bail!("project Store poisoned"),
                Ok(guard) => drop(guard),
            }
            anyhow::ensure!(Instant::now() < deadline, "creator did not acquire Store");
            thread::sleep(Duration::from_millis(1));
        }
        let refresh_router = router.clone();
        let refresh_store = store_handle.clone();
        thread::spawn(move || {
            let result = refresh_router.open_registered_with("project-b", |_, _| {
                registry_tx.send(())?;
                let store = refresh_store.lock().unwrap();
                // Match registry -> Store ordering used by open_registered/read_tasks.
                assert_eq!(store.list::<Value>("task")?.len(), 1);
                Ok(())
            });
            let _ = refreshed_tx.send(result.map(|_| ()));
        });
        registry_rx.recv_timeout(timeout)?;
        Ok(())
    })?;

    let task = created_rx
        .recv_timeout(timeout)
        .expect("task creation deadlocked against registry refresh")?;
    refreshed_rx
        .recv_timeout(timeout)
        .expect("registry refresh did not finish")?;
    let task_id = task["id"].as_str().unwrap();
    assert_eq!(task["status"], "queued");
    assert_eq!(router.runtime_for_task(task_id)?.project_id(), "project-a");
    assert_eq!(store_handle.lock().unwrap().list::<Value>("task")?.len(), 1);
    assert!(host.lock().unwrap().list::<Value>("task")?.is_empty());
    Ok(())
}
