#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/framework.rs"]
mod framework_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::asset_delivery_files as artifacts;
use delivery_support::Fixture;
use serde_json::json;
use std::{fs, path::Path};

#[tokio::test]
async fn durable_callback_and_check_use_project_frozen_content() -> Result<()> {
    let f = Fixture::new_project()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "frozen design")?;
    let request = json!({"operation":"start","requestId":"submit-operation",
        "job":{"kind":"callback","request":f.request("submit", &["design.md"])?}});
    let op = f.framework(true, request.clone()).await?;
    let done = f.terminal(op["id"].as_str().unwrap()).await?;
    assert_eq!(done["status"], "succeeded", "{done}");
    assert_eq!(done["paused"], true);
    assert_eq!(f.framework(true, request).await?, done);
    let task = f.task()?;
    let candidate = task["waitingDelivery"].as_str().unwrap();
    let frozen = artifacts::get(&f.store.lock().unwrap(), "task", candidate)?;
    let hash = &frozen.files["design.md"];
    assert_eq!(
        f.files().blob(hash)?,
        fs::canonicalize(f.temp.path().join("project/.beaver/content/blobs"))?.join(hash)
    );
    fs::write(f.workspace.join("design.md"), "unreviewed edit")?;
    assert_eq!(
        artifacts::read(&f.files(), &frozen, "design.md")?,
        b"frozen design"
    );
    let checked = f.check(candidate).await?;
    assert_eq!(checked["status"], "succeeded", "{checked}");
    let value = &checked["result"]["value"];
    assert_eq!(value["passed"], true);
    let export = Path::new(value["exportPath"].as_str().unwrap());
    assert!(export.starts_with(fs::canonicalize(
        f.temp.path().join("project/.beaver/cache")
    )?));
    assert_eq!(
        fs::read_to_string(export.join("design.md"))?,
        "frozen design"
    );
    assert_no_legacy_blobs(&f);
    Ok(())
}

#[tokio::test]
async fn project_input_exports_keep_distinct_attempt_history() -> Result<()> {
    let f = Fixture::new_project()?;
    f.plan().await?;
    let mut attempts = Vec::new();
    for version in ["original model", "edited model"] {
        fs::write(f.workspace.join("model.blend"), version)?;
        let id = f.create(version).await?;
        let begun = f
            .work(json!({"action":"begin","subtaskId":id,"inputs":{},
            "inputFiles":[{"path":"model.blend","role":"source"}]}))
            .await?;
        let attempt = begun["attemptId"].as_str().unwrap().to_owned();
        let state = f.state()?;
        let captured = &state
            .work
            .attempts
            .iter()
            .find(|a| a.id == attempt)
            .unwrap()
            .input_files
            .as_ref()
            .unwrap()[0];
        assert_eq!(
            fs::read_to_string(f.files().blob(&captured.sha256)?)?,
            version
        );
        f.finish(&attempt, "completed").await?;
        attempts.push(attempt);
    }
    fs::write(f.workspace.join("model.blend"), "not a frozen input")?;
    let candidate = f.submit().await?;
    let checked = f.check(&candidate).await?;
    assert_eq!(checked["status"], "succeeded", "{checked}");
    let exports = checked["result"]["value"]["attemptInputs"]
        .as_array()
        .unwrap();
    assert_eq!(exports.len(), 2);
    assert_ne!(exports[0]["path"], exports[1]["path"]);
    let cache = fs::canonicalize(f.temp.path().join("project/.beaver/cache/delivery-exports"))?;
    for (attempt, expected) in attempts.iter().zip(["original model", "edited model"]) {
        let export = exports.iter().find(|e| e["attemptId"] == *attempt).unwrap();
        let path = Path::new(export["path"].as_str().unwrap());
        assert!(path.starts_with(&cache));
        assert_eq!(fs::read_to_string(path.join("model.blend"))?, expected);
        let op = f
            .start(false, json!({"kind":"inputExport","attemptId":attempt}))
            .await?;
        let done = f.terminal(op["id"].as_str().unwrap()).await?;
        assert_eq!(done["status"], "succeeded", "{done}");
        let value = &done["result"]["value"];
        assert_eq!(value["source"], "frozen-attempt-inputs");
        let path = Path::new(value["path"].as_str().unwrap());
        assert!(path.starts_with(&cache));
        assert_eq!(fs::read_to_string(path.join("model.blend"))?, expected);
    }
    assert_no_legacy_blobs(&f);
    Ok(())
}

#[tokio::test]
async fn judgment_reads_project_snapshot_and_never_falls_back_to_legacy_blob() -> Result<()> {
    let f = Fixture::new_project()?;
    f.plan().await?;
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
        .save(f.workspace.join("preview.png"))?;
    let result = f.call(f.request("submit", &["preview.png"])?).await?;
    let candidate = result["candidateId"].as_str().unwrap();
    let frozen = artifacts::get(&f.store.lock().unwrap(), "task", candidate)?;
    f.configure(json!([]), json!([]), true).await?;
    f.resume("judge-turn")?;
    fs::write(f.workspace.join("preview.png"), "no longer a PNG")?;
    let request = json!({"operation":"judge","candidateId":candidate,
        "configurationRevision":1,"verdict":"pass","note":"Inspected silhouette",
        "paths":["preview.png"]});
    assert_eq!(
        f.framework(false, request.clone()).await?["source"],
        "owner"
    );
    assert_no_legacy_blobs(&f);
    let hash = &frozen.files["preview.png"];
    let decoy = f.temp.path().join("project/.beaver/blobs");
    fs::create_dir(&decoy)?;
    fs::rename(f.files().blob(hash)?, decoy.join(hash))?;
    assert!(f.framework(false, request).await.is_err());
    Ok(())
}

#[tokio::test]
async fn legacy_exports_keep_the_existing_location() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    let frozen = artifacts::get(&f.store.lock().unwrap(), "task", &candidate)?;
    let export = artifacts::export(&f.files(), &frozen)?;
    assert!(export.starts_with(fs::canonicalize(f.temp.path().join("delivery-exports"))?));
    assert_eq!(
        fs::read_to_string(export.join("docs/design.md"))?,
        "# Design\nSaved output\n"
    );
    Ok(())
}

fn assert_no_legacy_blobs(f: &Fixture) {
    assert!(!f.temp.path().join("blobs").exists());
    assert!(!f.temp.path().join("project/.beaver/blobs").exists());
    assert!(!f
        .temp
        .path()
        .join("project/.beaver/delivery-exports")
        .exists());
}

#[tokio::test]
async fn foreign_workspace_cannot_enter_callback_or_framework_job() -> Result<()> {
    let f = Fixture::new_project()?;
    f.plan().await?;
    let outside = f.temp.path().join("foreign-workspace");
    fs::create_dir(&outside)?;
    fs::write(outside.join("design.md"), "foreign design")?;
    let mut task = f.task()?;
    task["workspace"] = json!(outside);
    f.store.lock().unwrap().put("task", "task", &task)?;
    let request = f.request("submit", &["design.md"])?;
    let before = beaver_core::task_callback::inspect(&f.store.lock().unwrap(), "task", None)?;
    let error = f.call(request.clone()).await.unwrap_err();
    assert!(
        error.to_string().contains("不属于当前项目或任务"),
        "{error:#}"
    );
    let error = f
        .start(true, json!({"kind":"callback","request":request}))
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("不属于当前项目或任务"),
        "{error:#}"
    );
    assert_eq!(
        beaver_core::task_callback::inspect(&f.store.lock().unwrap(), "task", None)?,
        before
    );
    assert!(f
        .store
        .lock()
        .unwrap()
        .list::<serde_json::Value>(beaver_core::framework_operations::KIND)?
        .is_empty());
    assert!(!f.temp.path().join("project/.beaver/content/blobs").exists());
    assert_eq!(fs::read(outside.join("design.md"))?, b"foreign design");
    assert!(!f.workspace.join("design.md").exists());
    Ok(())
}

#[tokio::test]
async fn approval_rejects_matching_files_in_a_foreign_workspace() -> Result<()> {
    let f = Fixture::new_project()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    let outside = f.temp.path().join("foreign-workspace");
    fs::create_dir_all(outside.join("docs"))?;
    fs::copy(
        f.workspace.join("docs/design.md"),
        outside.join("docs/design.md"),
    )?;
    let mut task = f.task()?;
    task["workspace"] = json!(outside);
    f.store.lock().unwrap().put("task", "task", &task)?;
    let before = beaver_core::asset_delivery_review::inspect(&f.store.lock().unwrap(), "task")?;
    let decision = f.decision(&candidate, "approve")?;
    let error = f.decide(decision.clone()).unwrap_err();
    assert!(
        error.to_string().contains("不属于当前项目或任务"),
        "{error:#}"
    );
    assert_eq!(
        beaver_core::asset_delivery_review::inspect(&f.store.lock().unwrap(), "task")?,
        before
    );
    assert_eq!(f.task()?, task);
    task["workspace"] = json!(f.files().workspace_location("task")?);
    f.store.lock().unwrap().put("task", "task", &task)?;
    assert_eq!(f.decide(decision)?["status"], "queued");
    Ok(())
}
