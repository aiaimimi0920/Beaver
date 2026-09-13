use super::*;
use crate::{asset_task, scheduler::Scheduler};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn fixture() -> Result<(tempfile::TempDir, Store, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("data");
    let project = temp.path().join("project");
    fs::create_dir(&project)?;
    fs::write(project.join("project.godot"), "[application]\n")?;
    let store = Store::open(&root)?;
    store.put("project", "p", &json!({"id":"p","path":project}))?;
    Ok((temp, store, root))
}

#[tokio::test]
async fn queued_asset_state_survives_interruption_without_starting_blender() -> Result<()> {
    let (_temp, mut store, root) = fixture()?;
    let mut ordinary = create(
        &mut store,
        &root,
        json!({"projectId":"p","prompt":"Ordinary task","decompose":false}),
        &Value::Null,
        &Value::Null,
    )?;
    let ordinary_id = ordinary["id"].as_str().unwrap().to_owned();
    assert!(store.get::<Value>("asset-task", &ordinary_id)?.is_none());
    // Occupy the only execution slot so interruption exercises the queued path.
    ordinary["status"] = json!("running");
    store.put("task", &ordinary_id, &ordinary)?;
    store.put("settings", "main", &json!({"maxParallel":1}))?;
    let task = create(
        &mut store,
        &root,
        json!({"projectId":"p","prompt":"Wait for confirmation","assetTask":true}),
        &Value::Null,
        &Value::Null,
    )?;
    let id = task["id"].as_str().unwrap().to_owned();
    let initial = asset_task::get(&store, &id)?;
    assert_eq!(task["status"], "queued");
    assert_eq!(task["decompose"], false);
    assert_eq!(initial.task_id, id);
    assert_eq!(initial.project_id, "p");
    assert!(initial.session_id.is_none());

    let store = Arc::new(Mutex::new(store));
    let scheduler = Scheduler::start(
        store.clone(),
        Arc::new(Files::new(root.clone())),
        Arc::new(|_| panic!("An occupied scheduler must not launch queued work")),
        Arc::new(|| {}),
    );
    scheduler
        .interrupt(id.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(scheduler);
    drop(store);

    let reopened = Store::open(&root)?;
    assert_eq!(
        reopened.get::<Value>("task", &id)?.unwrap()["status"],
        "interrupted"
    );
    let retained = asset_task::get(&reopened, &id)?;
    assert_eq!(retained.task_id, initial.task_id);
    assert_eq!(retained.project_id, initial.project_id);
    assert_eq!(retained.round, initial.round);
    assert!(retained.session_id.is_none());
    assert!(retained.feedback.is_empty());
    assert!(reopened.get::<Value>("asset-task", &ordinary_id)?.is_none());
    Ok(())
}

#[test]
fn asset_state_failure_rolls_back_task_event_and_related_receipt() -> Result<()> {
    let (_temp, mut store, root) = fixture()?;
    store.connection.execute_batch(
        "CREATE TRIGGER reject_asset_state BEFORE INSERT ON entities
         WHEN NEW.kind = 'asset-task'
         BEGIN SELECT RAISE(ABORT, 'asset state unavailable'); END;",
    )?;
    let mut attempted_id = String::new();
    let result = create_recorded(
        &mut store,
        &root,
        json!({"projectId":"p","prompt":"Create asset","assetTask":true}),
        &Value::Null,
        &Value::Null,
        |_, _, task| {
            attempted_id = task["id"].as_str().unwrap().into();
            Ok(vec![(
                "receipt",
                "request".into(),
                json!({"taskId":attempted_id}),
            )])
        },
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("asset state unavailable"));
    assert!(!attempted_id.is_empty());
    assert!(store.list::<Value>("task")?.is_empty());
    assert!(store.events(&attempted_id)?.is_empty());
    assert!(store.list::<Value>("asset-task")?.is_empty());
    assert!(store.list::<Value>("receipt")?.is_empty());
    Ok(())
}
