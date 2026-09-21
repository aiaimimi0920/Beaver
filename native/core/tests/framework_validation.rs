#[path = "support/asset_delivery.rs"]
mod delivery_support;
#[path = "support/framework.rs"]
mod framework_support;
#[path = "support/asset_work.rs"]
mod support;

use anyhow::Result;
use beaver_core::{asset_delivery_files as artifacts, framework_checks};
use delivery_support::Fixture;
use serde_json::{json, Value};

#[tokio::test]
async fn required_checks_gate_owner_approval_and_configuration_changes_invalidate_reports(
) -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    f.configure(json!([]), json!([framework_support::rule("pass")?]), false)
        .await?;
    assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    let checked = f.check(&candidate).await?;
    assert_eq!(checked["status"], "succeeded", "{checked}");
    assert_eq!(checked["result"]["value"]["passed"], true);
    let artifact = artifacts::get(&f.store.lock().unwrap(), "task", &candidate)?;
    framework_checks::gate(&f.store.lock().unwrap(), "task", &artifact)?;
    f.configure(json!([]), json!([framework_support::rule("fail")?]), false)
        .await?;
    assert!(framework_checks::gate(&f.store.lock().unwrap(), "task", &artifact).is_err());
    assert_eq!(
        f.check(&candidate).await?["result"]["value"]["passed"],
        false
    );
    assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    f.configure(json!([]), json!([framework_support::rule("pass")?]), false)
        .await?;
    assert_eq!(
        f.check(&candidate).await?["result"]["value"]["passed"],
        true
    );
    f.decide(f.decision(&candidate, "approve")?)?;
    Ok(())
}

#[tokio::test]
async fn exit_success_wrong_rule_version_and_input_tampering_cannot_pass() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    for id in ["wrong-rule-version", "malformed", "crash", "tamper"] {
        f.configure(json!([]), json!([framework_support::rule(id)?]), false)
            .await?;
        let op = f.check(&candidate).await?;
        assert_ne!(op["result"]["value"]["passed"], true, "{op}");
        assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    }
    assert_eq!(
        std::fs::read(f.workspace.join("docs/design.md"))?,
        b"# Design\nSaved output\n"
    );
    Ok(())
}

#[tokio::test]
async fn rule_resource_changes_block_check_and_approval() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    let script = f.temp.path().join("adapter.resource");
    std::fs::write(&script, "version one")?;
    let mut rule = framework_support::rule("pass")?;
    rule["checker"]["files"] = json!({script.to_string_lossy():framework_support::hash(&script)?});
    f.configure(json!([]), json!([rule]), false).await?;
    assert_eq!(
        f.check(&candidate).await?["result"]["value"]["passed"],
        true
    );
    std::fs::write(&script, "version two")?;
    assert!(f.decide(f.decision(&candidate, "approve")?).is_err());
    assert_eq!(
        f.check(&candidate).await?["result"]["value"]["passed"],
        false
    );
    Ok(())
}

#[tokio::test]
async fn a_check_cannot_publish_if_its_terminal_operation_write_fails() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    let candidate = f.submit().await?;
    let db = rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?;
    db.execute_batch("CREATE TRIGGER reject_terminal BEFORE UPDATE ON entities WHEN NEW.kind='framework-operation' AND json_extract(NEW.value,'$.status')='succeeded' BEGIN SELECT RAISE(ABORT,'terminal failure'); END;")?;
    let op = f.check(&candidate).await?;
    assert_eq!(op["status"], "failed", "{op}");
    assert_eq!(
        f.store
            .lock()
            .unwrap()
            .get::<Value>("framework-check/task", &candidate)?
            .unwrap()["passed"],
        false
    );
    Ok(())
}

#[tokio::test]
async fn plugins_require_probe_version_enabled_callable_and_no_restart() -> Result<()> {
    let f = Fixture::new()?;
    for id in ["ready", "wrong-version", "restart"] {
        f.configure(json!([framework_support::plugin(id)?]), json!([]), false)
            .await?;
        let operation = f
            .start(
                true,
                json!({"kind":"plugin","plugin":id,"action":"install"}),
            )
            .await?;
        let completed = f.terminal(operation["id"].as_str().unwrap()).await?;
        assert_eq!(completed["status"], "succeeded", "{completed}");
        let result = &completed["result"]["value"];
        assert_eq!(result["ready"], id == "ready");
        assert_eq!(result["evidence"].as_array().unwrap().len(), 2);
        assert!(!result.to_string().contains("private-output"));
    }
    assert_eq!(
        std::fs::read_to_string(f.workspace.join("installed"))?
            .lines()
            .count(),
        3
    );
    Ok(())
}

#[tokio::test]
async fn adapter_timeout_and_bad_executable_pin_fail_without_secret_output() -> Result<()> {
    let f = Fixture::new()?;
    let mut plugin = framework_support::plugin("slow")?;
    plugin["probe"]["timeoutSeconds"] = json!(1);
    for bad_pin in [false, true] {
        if bad_pin {
            plugin["probe"]["sha256"] = json!("0".repeat(64));
        }
        f.configure(json!([plugin]), json!([]), false).await?;
        let operation = f
            .start(
                true,
                json!({"kind":"plugin","plugin":"slow","action":"probe"}),
            )
            .await?;
        let completed = f.terminal(operation["id"].as_str().unwrap()).await?;
        assert_eq!(completed["status"], "failed", "{completed}");
        assert!(!completed.to_string().contains("private-output"));
        if bad_pin {
            assert!(completed.to_string().contains("Pinned"), "{completed}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn only_owner_visual_judgment_over_decodable_frozen_images_satisfies_semantic_gate(
) -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
        .save(f.workspace.join("preview.png"))?;
    std::fs::write(f.workspace.join("invalid.png"), "not an image")?;
    let result = f
        .call(f.request("submit", &["preview.png", "invalid.png"])?)
        .await?;
    let candidate = result["candidateId"].as_str().unwrap();
    f.configure(json!([]), json!([]), true).await?;
    f.resume("judge-turn")?;
    let request = json!({"operation":"judge","candidateId":candidate,"configurationRevision":1,"verdict":"pass","note":"Inspected silhouette","paths":["preview.png"]});
    let mut invalid = request.clone();
    invalid["paths"] = json!(["invalid.png"]);
    assert!(f.framework(false, invalid).await.is_err());
    assert_eq!(f.framework(true, request.clone()).await?["source"], "model");
    let artifact = artifacts::get(&f.store.lock().unwrap(), "task", candidate)?;
    assert!(framework_checks::gate(&f.store.lock().unwrap(), "task", &artifact).is_err());
    assert_eq!(
        f.framework(false, request.clone()).await?["source"],
        "owner"
    );
    framework_checks::gate(&f.store.lock().unwrap(), "task", &artifact)?;
    f.configure(json!([]), json!([]), true).await?;
    assert!(f.framework(false, request).await.is_err());
    assert!(framework_checks::gate(&f.store.lock().unwrap(), "task", &artifact).is_err());
    assert_eq!(
        f.store
            .lock()
            .unwrap()
            .list::<Value>("framework-judgment-history/task")?
            .len(),
        2
    );
    Ok(())
}
