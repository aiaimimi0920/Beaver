#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_work_inputs::Role, task_callback};
use delivery_support::Fixture;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn freezes_real_source_version_and_allows_intended_edits() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Edit model").await?;
    let path = f.workspace.join("model.blend");
    std::fs::write(&path, b"original model")?;
    let hash = format!("{:x}", Sha256::digest(b"original model"));
    let result = f
        .work(
            json!({"action":"begin","subtaskId":id,"inputs":{"sha256":"model claim"},
        "inputFiles":[{"path":"model.blend","role":"source","expectedSha256":hash}]}),
        )
        .await?;
    std::fs::write(&path, b"edited model")?;
    f.finish(result["attemptId"].as_str().unwrap(), "completed")
        .await?;
    let state = f.state()?;
    let input = &state.work.attempts[0].input_files.as_ref().unwrap()[0];
    assert_eq!(input.sha256, hash);
    assert_eq!(input.role, Role::Source);
    assert_eq!(std::fs::read(f.files().blob(&hash)?)?, b"original model");
    assert_eq!(state.work.attempts[0].inputs["sha256"], "model claim");
    let candidate = f.submit().await?;
    f.decide(f.decision(&candidate, "approve")?)?;
    Ok(())
}

#[tokio::test]
async fn invalid_input_declarations_leave_no_attempt_receipt_or_revision() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Capture").await?;
    std::fs::write(f.workspace.join("ref.png"), b"reference")?;
    std::fs::write(f.workspace.join("empty.png"), b"")?;
    let item = json!({"path":"ref.png","role":"dependency"});
    let mut manifests = vec![
        json!(null),
        json!([{"path":"missing.png","role":"source"}]),
        json!([{"path":"empty.png","role":"dependency"}]),
        json!([{"path":"../ref.png","role":"source"}]),
        json!([{"path":"C:/ref.png","role":"source"}]),
        json!([{"path":".beaver/secret","role":"source"}]),
        json!([{"path":"ref.png","role":"plugin"}]),
        json!([{"path":"ref.png","role":"source","sha256":"forged"}]),
        json!([{"path":"ref.png","role":"dependency","expectedSha256":"0".repeat(64)}]),
        json!([{"path":"ref.png","role":"dependency","expectedSha256":"A".repeat(64)}]),
        json!([item.clone(), item.clone()]),
        json!([item.clone(), {"path":"REF.PNG","role":"source"}]),
    ];
    manifests.push(json!(vec![item; 33]));
    let before = serde_json::to_value(f.state()?)?;
    let revision =
        task_callback::inspect(&f.store.lock().unwrap(), "task", None)?["revision"].clone();
    for manifest in manifests {
        let request = f.work_request(
            json!({"action":"begin","subtaskId":id,"inputs":{},"inputFiles":manifest}),
        )?;
        assert!(f.call(request.clone()).await.is_err(), "{request}");
        assert_eq!(serde_json::to_value(f.state()?)?, before);
        let view = task_callback::inspect(
            &f.store.lock().unwrap(),
            "task",
            request["requestId"].as_str(),
        )?;
        assert_eq!(view["revision"], revision);
        assert!(view["receipt"].is_null());
    }
    assert!(f
        .work(json!({"action":"begin","subtaskId":id,"inputs":{}}))
        .await
        .is_err());
    Ok(())
}

#[tokio::test]
async fn exact_retries_retain_capture_but_sync_calls_cannot_skip_verification() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Use reference").await?;
    std::fs::write(f.workspace.join("ref.png"), b"version one")?;
    let request = f.work_request(json!({"action":"begin","subtaskId":id,"inputs":{},
        "inputFiles":[{"path":"ref.png","role":"dependency"}]}))?;
    assert!(task_callback::call(
        &mut f.store.lock().unwrap(),
        "task",
        "thread",
        "turn",
        &request
    )
    .is_err());
    let result = f.call(request.clone()).await?;
    let finish = f.work_request(json!({"action":"finish","attemptId":result["attemptId"],
        "outcome":"completed","summary":"Finished","outputs":{}}))?;
    assert!(task_callback::call(
        &mut f.store.lock().unwrap(),
        "task",
        "thread",
        "turn",
        &finish
    )
    .is_err());
    let finished = f.call(finish.clone()).await?;
    std::fs::write(f.workspace.join("ref.png"), b"version two")?;
    assert_eq!(f.call(request).await?, result);
    assert_eq!(f.call(finish).await?, finished);
    assert_eq!(f.state()?.work.attempts.len(), 1);
    assert!(f.submit().await.is_err());
    Ok(())
}

#[tokio::test]
async fn captured_inputs_and_revisions_roll_back_with_failed_receipt_commit() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("Atomic capture").await?;
    std::fs::write(f.workspace.join("ref.png"), b"reference")?;
    let request = f.work_request(json!({"action":"begin","subtaskId":id,"inputs":{},
        "inputFiles":[{"path":"ref.png","role":"source"}]}))?;
    let before = serde_json::to_value(f.state()?)?;
    let db = rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?;
    db.execute_batch("CREATE TRIGGER reject_capture BEFORE INSERT ON entities WHEN NEW.kind='task-callback/task' BEGIN SELECT RAISE(ABORT, 'receipt failure'); END;")?;
    assert!(f.call(request.clone()).await.is_err());
    assert_eq!(serde_json::to_value(f.state()?)?, before);
    let view = task_callback::inspect(
        &f.store.lock().unwrap(),
        "task",
        request["requestId"].as_str(),
    )?;
    assert_eq!(view["revision"], request["expectedRevision"]);
    assert!(view["receipt"].is_null());
    db.execute_batch("DROP TRIGGER reject_capture;")?;
    f.call(request).await?;
    assert_eq!(f.state()?.work.attempts.len(), 1);
    Ok(())
}

#[tokio::test]
async fn old_attempts_remain_distinct_from_explicit_empty_manifests() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let id = f.create("No input files").await?;
    let attempt = f.begin(&id, "").await?;
    assert!(f.state()?.work.attempts[0]
        .input_files
        .as_ref()
        .unwrap()
        .is_empty());
    let mut legacy: Value = serde_json::to_value(f.state()?)?;
    legacy["work"]["attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("inputFiles");
    f.store.lock().unwrap().put("asset-task", "task", &legacy)?;
    assert!(f.state()?.work.attempts[0].input_files.is_none());
    f.finish(&attempt, "completed").await?;
    assert!(f.state()?.work.attempts[0].input_files.is_none());
    Ok(())
}
