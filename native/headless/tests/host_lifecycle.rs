use anyhow::Result;
use beaver_core::{project_storage::ProjectStore, store::Store, validation};
use beaver_headless::Host;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, sync::Arc, time::Duration};

async fn project(host: &Arc<Host>, parent: &Path) -> Result<Value> {
    host.call(
        "project.create",
        json!({"parent":parent,"name":"Blank","template":"blank"}),
    )
    .await
}

async fn terminal(host: &Arc<Host>, id: &Value) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state = host.call("state", json!({})).await?;
            let task = state["tasks"]
                .as_array()
                .unwrap()
                .iter()
                .find(|task| task["id"] == *id)
                .unwrap();
            if !matches!(task["status"].as_str(), Some("queued" | "running")) {
                return Ok(task.clone());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_scheduler_fails_without_provider_in_project_store_only() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Host::open(&data)?;
    let project = project(&host, temp.path()).await?;
    let root = Path::new(project["path"].as_str().unwrap());
    let original = fs::read(root.join("project.godot"))?;
    let task = tokio::time::timeout(Duration::from_secs(10), host.call("task.create", json!({
        "projectId":project["id"], "prompt":"Create a character", "assetTask":true, "decompose":false
    }))).await??;
    let finished = terminal(&host, &task["id"]).await?;
    assert_eq!(finished["status"], "failed");
    assert!(finished["threadId"].is_null());
    assert!(finished["error"].as_str().unwrap().contains("服务地址"));
    assert!(finished["retainedBlenderPort"].is_null());
    assert_eq!(fs::read(root.join("project.godot"))?, original);
    let workspace = task["workspace"].as_str().unwrap();
    assert_eq!(
        workspace,
        format!(".beaver/workspaces/{}", task["id"].as_str().unwrap())
    );
    assert!(root.join(workspace).is_dir());
    assert!(Store::open(&data)?.list::<Value>("task")?.is_empty());
    let events = host.call("task.events", json!({"id":task["id"]})).await?;
    assert!(events.as_array().is_some_and(|events| !events.is_empty()));
    let logs = host
        .call("logs.query", json!({"taskId":task["id"]}))
        .await?;
    assert!(logs["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["method"] == "task.create"));
    assert!(host
        .call(
            "task.answer",
            json!({"id":task["id"],"questionId":"absent","answers":{}})
        )
        .await
        .is_err());
    host.call("task.continue", json!({"id":task["id"],"text":"Retry"}))
        .await?;
    assert_eq!(terminal(&host, &task["id"]).await?["status"], "failed");
    host.call("task.interrupt", json!({"id":task["id"]}))
        .await?;
    host.shutdown().await?;
    drop(host);
    let local = ProjectStore::open(root, project["id"].as_str().unwrap())?;
    assert!(local
        .store()
        .get::<Value>("task", task["id"].as_str().unwrap())?
        .is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_recovers_once_without_resuming_and_instance_lock_is_exclusive() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Host::open(&data)?;
    assert!(Host::open(&data).is_err());
    let project = project(&host, temp.path()).await?;
    let root = Path::new(project["path"].as_str().unwrap());
    let id = project["id"].as_str().unwrap();
    assert!(ProjectStore::open(root, id).is_err());
    host.shutdown().await?;
    drop(host);
    let local = ProjectStore::open(root, id)?;
    for (task_id, status) in [("queued-task", "queued"), ("running-task", "running")] {
        local.store().put(
            "task",
            task_id,
            &json!({"id":task_id,"projectId":id,"status":status}),
        )?;
    }
    let run = validation::repository::new_run(
        local.store(),
        id,
        BTreeMap::new(),
        None,
        Some("running-task".into()),
        None,
    )?;
    drop(local);
    let recovered = Host::open(&data)?;
    for _ in 0..2 {
        let state = recovered.call("state", json!({})).await?;
        assert_eq!(state["tasks"].as_array().unwrap().len(), 2);
        assert!(state["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|task| task["status"] == "interrupted" && task["threadId"].is_null()));
    }
    recovered.shutdown().await?;
    drop(recovered);
    let local = ProjectStore::open(root, id)?;
    let run = local
        .store()
        .get::<validation::model::Run>("validationRun", &run.id)?
        .unwrap();
    assert_eq!(run.status, "interrupted");
    assert_eq!(run.verdict, "needsReview");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_free_settings_and_legacy_boundary_are_enforced_before_mutation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::open(temp.path())?;
    let mut settings = host.call("settings.get", json!({})).await?;
    settings["maxParallel"] = json!(1);
    let saved = host
        .call("settings.save", json!({"settings":settings,"keys":{}}))
        .await?;
    assert_eq!(saved["maxParallel"], 1);
    settings["maxParallel"] = json!(2);
    assert!(host
        .call(
            "settings.save",
            json!({"settings":settings,"keys":{"code":"do-not-save"}})
        )
        .await
        .is_err());
    assert_eq!(
        host.call("settings.get", json!({})).await?["maxParallel"],
        1
    );
    assert!(host
        .call(
            "task.create",
            json!({"projectId":"absent","prompt":"x","objectFramework":{}})
        )
        .await
        .is_err());
    assert!(host.call("objectTask.run", json!({})).await.is_err());
    assert!(host
        .call("workflow.list", json!({"id":"absent","extra":true}))
        .await
        .is_err());
    host.shutdown().await?;
    drop(host);
    assert!(Store::open(temp.path())?
        .list::<Value>("secret")?
        .is_empty());
    Ok(())
}
