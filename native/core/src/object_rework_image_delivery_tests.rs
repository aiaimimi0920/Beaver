use super::*;
use crate::{object_attempt_callback as callback, object_attempt_control};
use serde_json::{json, Value};

fn params(hash: &str) -> Value {
    json!({"tool":callback::TOOL,"threadId":"thread","turnId":"turn",
        "arguments":{"operation":"inputFile","path":"preview.png","sha256":hash}})
}

// Owned RPC execution requires the Windows-only process-tree fixture.
#[cfg(windows)]
#[tokio::test]
async fn frozen_png_rework_delivers_actual_image_over_model_rpc() -> Result<()> {
    use crate::object_attempt_worker;
    use base64::Engine;
    use std::sync::Arc;

    let f = fixture()?;
    let request = image_request(&f, true)?;
    let image = request.rework.as_ref().unwrap().image.as_ref().unwrap();
    let bytes = std::fs::read(f.runtime.files().blob(&image.sha256)?)?;
    let (_, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    let id = lease.record().id.clone();
    let root = f.temp.path().join("image-rpc");
    let launch_root = root.clone();
    let (_controls, receiver) = tokio::sync::mpsc::channel(16);
    object_attempt_worker::execute_lease(
        f.runtime.clone(),
        lease,
        Arc::new(move |attempt, runtime| {
            let cwd = runtime
                .files()
                .resolve_workspace(
                    &attempt.preparation.run.id,
                    std::path::Path::new(&attempt.preparation.workspace),
                )
                .map_err(|error| error.to_string())?;
            super::super::super::object_attempt_rpc_fixture::launch(
                &launch_root,
                &cwd,
                "image-callback",
            )
            .map_err(|error| error.to_string())
        }),
        Arc::new(AtomicBool::new(false)),
        receiver,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    super::super::super::object_attempt_rpc_fixture::assert_closed(&root)?;
    let attempt = attempts(&f, "head")?
        .into_iter()
        .find(|a| a.id == id)
        .unwrap();
    assert_eq!(attempt.state, State::AwaitingGate, "{:?}", attempt.error);
    let hash = &attempt.output.as_ref().unwrap()["image-delivered.json"];
    let reply: Value = serde_json::from_slice(&std::fs::read(f.runtime.files().blob(hash)?)?)?;
    let items = reply["contentItems"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[1]["imageUrl"],
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        )
    );
    let metadata: Value = serde_json::from_str(items[0]["text"].as_str().unwrap())?;
    assert_eq!(metadata["request"]["attemptId"], id);
    assert_eq!(metadata["request"]["sha256"], image.sha256);
    assert_eq!(metadata["content"]["width"], 20);
    assert_eq!(metadata["content"]["height"], 10);
    assert!(metadata["content"].get("base64").is_none());
    assert_eq!(
        std::fs::read(f.runtime.files().blob(&image.sha256)?)?,
        bytes
    );
    assert!(attempt.fine.prompt.contains("Brighten this area"));
    assert_eq!(task_record(&f, "head")?.status, "awaitingAcceptance");
    Ok(())
}

#[test]
fn frozen_png_delivery_rejects_stale_identity_corruption_and_interrupt() -> Result<()> {
    let f = fixture()?;
    let request = image_request(&f, true)?;
    let image = request.rework.as_ref().unwrap().image.as_ref().unwrap();
    let (_, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    let mut lease = lease.unwrap();
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let read = params(&image.sha256);
    for field in ["threadId", "turnId", "tool"] {
        let mut foreign = read.clone();
        foreign[field] = json!("foreign");
        assert!(callback::call(&f.runtime, &lease, &foreign).is_err());
    }
    assert!(callback::call(&f.runtime, &lease, &params(&"a".repeat(64))).is_err());
    let blob = f.runtime.files().blob(&image.sha256)?;
    let bytes = std::fs::read(&blob)?;
    std::fs::write(&blob, b"tampered")?;
    assert!(callback::call(&f.runtime, &lease, &read)
        .unwrap_err()
        .to_string()
        .contains("HASH_MISMATCH"));
    std::fs::write(&blob, bytes)?;
    assert_eq!(
        callback::call(&f.runtime, &lease, &read)?["contentItems"][1]["type"],
        "inputImage"
    );
    object_attempt_control::request(&f.runtime, &interrupt_request(&f, "head")?, true)?;
    assert!(callback::call(&f.runtime, &lease, &read)
        .unwrap_err()
        .to_string()
        .contains("INTERRUPTED"));
    Ok(())
}
