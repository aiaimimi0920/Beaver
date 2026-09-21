#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/framework.rs"]
mod framework_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{
    framework,
    framework_contract::Job,
    framework_operations::{self as operations, Operation},
};
use delivery_support::Fixture;
use serde_json::{json, Value};

#[tokio::test]
async fn durable_callback_retry_survives_parking_and_is_identity_scoped() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    std::fs::create_dir(f.workspace.join("docs"))?;
    std::fs::write(f.workspace.join("docs/design.md"), "frozen")?;
    let request = json!({"operation":"start","requestId":"submit-operation","job":{"kind":"callback","request":f.request("submit", &["docs/design.md"])?}});
    let op = f.framework(true, request.clone()).await?;
    let completed = f.terminal(op["id"].as_str().unwrap()).await?;
    assert_eq!(completed["status"], "succeeded");
    assert_eq!(completed["paused"], true);
    assert_eq!(f.framework(true, request.clone()).await?, completed);
    assert_eq!(f.state()?.delivery.unwrap().history.len(), 1);
    let mut conflict = request.clone();
    conflict["job"]["request"]["summary"] = json!("Changed");
    assert!(f.framework(true, conflict).await.is_err());
    assert!(f.framework(false, request.clone()).await.is_err());
    f.store
        .lock()
        .unwrap()
        .put("task", "other", &json!({"id":"other"}))?;
    assert!(framework::call(
        f.store.clone(),
        f.files(),
        "other".into(),
        None,
        json!({"operation":"inspect","operationId":op["id"]})
    )
    .await
    .is_err());
    f.resume("new-turn")?;
    assert!(f.framework(true, request).await.is_err());
    Ok(())
}

#[tokio::test]
async fn cancel_is_queryable_and_interrupts_a_real_child_without_replaying() -> Result<()> {
    let f = Fixture::new()?;
    f.configure(
        json!([framework_support::plugin("slow")?]),
        json!([]),
        false,
    )
    .await?;
    let op = f
        .start(
            true,
            json!({"kind":"plugin","plugin":"slow","action":"install"}),
        )
        .await?;
    framework_support::started(&f.workspace.join("adapter-started")).await?;
    assert!(f.configure(json!([]), json!([]), false).await.is_err());
    assert!(f
        .start(
            true,
            json!({"kind":"plugin","plugin":"slow","action":"probe"})
        )
        .await
        .is_err());
    let cancelled = f
        .framework(false, json!({"operation":"cancel","operationId":op["id"]}))
        .await?;
    assert_eq!(cancelled["status"], "cancelRequested");
    assert_eq!(
        f.terminal(op["id"].as_str().unwrap()).await?["status"],
        "cancelled"
    );
    assert!(!f.workspace.join("installed").exists());
    Ok(())
}

#[tokio::test]
async fn ending_the_model_turn_cancels_its_operations() -> Result<()> {
    let f = Fixture::new()?;
    f.configure(
        json!([framework_support::plugin("slow")?]),
        json!([]),
        false,
    )
    .await?;
    let op = f
        .start(
            true,
            json!({"kind":"plugin","plugin":"slow","action":"install"}),
        )
        .await?;
    framework_support::started(&f.workspace.join("adapter-started")).await?;
    f.resume("replacement")?;
    assert_eq!(
        f.terminal(op["id"].as_str().unwrap()).await?["status"],
        "cancelled"
    );
    assert!(!f.workspace.join("installed").exists());
    framework::shutdown(&f.store).await?;
    Ok(())
}

#[tokio::test]
async fn startup_repairs_committed_callback_receipts_but_never_replays_unfinished_jobs(
) -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    std::fs::create_dir(f.workspace.join("docs"))?;
    std::fs::write(f.workspace.join("docs/design.md"), "frozen")?;
    let request = f.request("submit", &["docs/design.md"])?;
    let response = f.call(request.clone()).await?;
    let op = Operation {
        id: "repair".into(),
        task_id: "task".into(),
        request_id: "durable".into(),
        job: Job::Callback { request },
        source: "model".into(),
        thread_id: Some("thread".into()),
        turn_id: Some("turn".into()),
        status: "running".into(),
        created_at: "now".into(),
        ended_at: None,
        result: None,
        error: None,
    };
    let db = f.store.lock().unwrap();
    db.put(operations::KIND, &op.id, &op)?;
    let mut unknown = op.clone();
    unknown.id = "unknown".into();
    unknown.job = Job::Plugin {
        plugin: "unknown".into(),
        action: "install".into(),
    };
    db.put(operations::KIND, &unknown.id, &unknown)?;
    operations::recover(&db)?;
    assert_eq!(
        operations::get(&db, "task", "repair")?.result,
        Some(response)
    );
    assert_eq!(operations::get(&db, "task", "repair")?.status, "succeeded");
    assert_eq!(
        operations::get(&db, "task", "unknown")?.status,
        "interrupted"
    );
    let evidence = db.list::<Value>("framework-recovery/task")?;
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0]["replayed"], false);
    operations::recover(&db)?;
    assert_eq!(db.list::<Value>("framework-recovery/task")?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn failed_operation_insert_preserves_previous_check_and_retry_key() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    let db = rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?;
    f.store.lock().unwrap().put(
        "framework-check/task",
        &candidate,
        &json!({"passed":true,"proof":"old"}),
    )?;
    db.execute_batch("CREATE TRIGGER reject_operation BEFORE INSERT ON entities WHEN NEW.kind='framework-operation' BEGIN SELECT RAISE(ABORT,'operation insert failure'); END;")?;
    let request = json!({"operation":"start","requestId":"atomic","job":{"kind":"check","candidateId":candidate}});
    assert!(f.framework(false, request.clone()).await.is_err());
    assert_eq!(
        f.store
            .lock()
            .unwrap()
            .get::<Value>("framework-check/task", &candidate)?
            .unwrap()["proof"],
        "old"
    );
    assert!(f
        .store
        .lock()
        .unwrap()
        .list::<Operation>(operations::KIND)?
        .is_empty());
    db.execute_batch("DROP TRIGGER reject_operation;")?;
    let op = f.framework(false, request).await?;
    assert_eq!(
        f.terminal(op["id"].as_str().unwrap()).await?["status"],
        "succeeded"
    );
    Ok(())
}
