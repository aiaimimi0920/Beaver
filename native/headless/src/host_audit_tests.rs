use super::Host;
use anyhow::Result;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repeated_shutdown_preserves_the_first_cleanup_failure() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::open(&temp.path().join("host"))?;
    let project = host
        .call(
            "project.create",
            json!({
                "parent":temp.path(), "name":"HeldRuntime", "template":"blank"
            }),
        )
        .await?;
    let lease = host
        .router
        .runtime_for_project(project["id"].as_str().unwrap())?;
    let first = host
        .shutdown()
        .await
        .expect_err("An externally held runtime cannot be closed");
    drop(lease);
    let second = host
        .shutdown()
        .await
        .expect_err("A repeated shutdown must not hide its earlier failure");
    assert_eq!(format!("{first:#}"), format!("{second:#}"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_update_failure_cannot_turn_committed_create_into_failure() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::open(&temp.path().join("host"))?;
    host.store.lock().unwrap().transaction(|db| {
        db.execute_batch("CREATE TRIGGER reject_audit_update BEFORE UPDATE ON calls BEGIN SELECT RAISE(FAIL, 'audit update unavailable'); END;")?;
        Ok(())
    })?;
    let project = host
        .call(
            "project.create",
            json!({
                "parent":temp.path(), "name":"Committed", "template":"blank"
            }),
        )
        .await?;
    assert!(temp.path().join("Committed/project.godot").is_file());
    let state = host.call("state", json!({})).await?;
    assert_eq!(state["projects"][0]["id"], project["id"]);
    host.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_library_caller_keeps_admitted_mutation_ahead_of_shutdown() -> Result<()> {
    use beaver_core::{call_log, project_storage::ProjectStore};
    use serde_json::Value;
    use std::time::Duration;

    let temp = tempfile::tempdir()?;
    let host = Host::open(&temp.path().join("host"))?;
    let project = host
        .call(
            "project.create",
            json!({
                "parent":temp.path(), "name":"CancelledCaller", "template":"blank"
            }),
        )
        .await?;
    let id = project["id"].as_str().unwrap();
    let runtime = host.router.runtime_for_project(id)?;
    let handle = runtime.store();
    // Block only the project mutation. The host audit insert can still establish
    // that the task.create request has been admitted and entered its API call.
    let blocked_project = handle.lock().unwrap();
    let caller_host = host.clone();
    let input = json!({"projectId":id,"prompt":"Preserve admitted work","decompose":false});
    let caller = tokio::spawn(async move { caller_host.call("task.create", input).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let logged = {
                let store = host.store.lock().unwrap();
                call_log::query(&store, &json!({"method":"task.create"}))?["records"]
                    .as_array()
                    .is_some_and(|records| !records.is_empty())
            };
            if logged {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(
        host.admission.try_lock().is_err(),
        "Cancelled caller released admitted work before its mutation finished"
    );
    drop(blocked_project);
    drop((handle, runtime));
    tokio::time::timeout(Duration::from_secs(10), host.shutdown()).await??;
    let local = ProjectStore::open(std::path::Path::new(project["path"].as_str().unwrap()), id)?;
    let tasks = local.store().list::<Value>("task")?;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["prompt"], "Preserve admitted work");
    assert!(!matches!(
        tasks[0]["status"].as_str(),
        Some("queued" | "running")
    ));
    Ok(())
}
