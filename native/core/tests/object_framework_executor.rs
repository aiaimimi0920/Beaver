use anyhow::Result;
use beaver_core::{
    executor::{Execution, Outcome},
    files::Files,
    store::Store,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::{process::Command, sync::mpsc};

#[tokio::test]
async fn disabled_identity_is_rejected_before_spawning_or_logging_a_model_call() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    let task = json!({"id":"task","projectId":"project","status":"running",
        "objectFramework":{"schemaVersion":1,"layer":"coarse"}});
    store.put("task", "task", &task)?;
    let store = Arc::new(Mutex::new(store));
    let execution = Execution {
        store: store.clone(),
        files: Arc::new(Files::new(temp.path().into())),
        task_id: "task".into(),
        model: "unused".into(),
        prompt: "Unused".into(),
        ask_user_tool: json!({}),
        max_minutes: 1,
        secrets: vec![],
    };
    let (_sender, receiver) = mpsc::channel(1);
    let outcome = execution
        .run(
            Command::new(temp.path().join("must-not-start.exe")),
            receiver,
        )
        .await;
    assert!(
        matches!(outcome, Outcome::Failed(ref message) if message.contains("OBJECT_FRAMEWORK_DISABLED"))
    );
    let store = store.lock().unwrap();
    assert_eq!(store.get::<Value>("task", "task")?, Some(task));
    assert!(store.events("task")?.is_empty());
    assert_eq!(
        beaver_core::call_log::query(&store, &json!({}))?["records"],
        json!([])
    );
    Ok(())
}
