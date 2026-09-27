//! Deliver verified frozen PNG bytes as a model image, without copying base64 into text.
use crate::object_attempt_file::{Content, Response};
use anyhow::{ensure, Context, Result};
use base64::Engine;
use serde_json::{json, Value};

pub(super) fn archived(data_url: &str) -> Result<Value> {
    let encoded = data_url
        .strip_prefix("data:image/png;base64,")
        .context("PREVIEW_FRAME_PNG_REQUIRED")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    if bytes.len() > crate::object_attempt_file::INLINE_LIMIT {
        return Ok(crate::asset_tool::text_result(
            json!({"content":{"kind":"tooLarge"}}),
        ));
    }
    // The archive resolver has verified the digest, bounded PNG decode and frame identity.
    Ok(json!({"success":true,"contentItems":[
        {"type":"inputText","text":"Inspect this frozen historical Godot frame and its numbered regions. Optional engine receipts describe historical static mesh rays or through-frustum box node sets, including occluded geometry and explicit truncation. Image-only regions assert no geometry. Re-locate against current content before editing; do not reuse historical topology."},
        {"type":"inputImage","imageUrl":data_url}
    ]}))
}

pub(super) fn result(response: Response) -> Result<Value> {
    let Content::Image { mime, base64 } = &response.content else {
        return Ok(crate::asset_tool::text_result(serde_json::to_value(
            response,
        )?));
    };
    // Only PNG has a bounded decoder in this host. Other preview formats retain
    // their existing metadata response rather than claiming visual delivery.
    if mime != "image/png" {
        return Ok(crate::asset_tool::text_result(serde_json::to_value(
            response,
        )?));
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64)?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .context("OBJECT_ATTEMPT_IMAGE_DECODE_FAILED")?;
    ensure!(
        u64::from(decoded.width()) * u64::from(decoded.height()) <= 16_777_216,
        "OBJECT_ATTEMPT_IMAGE_TOO_LARGE"
    );
    Ok(json!({"success":true,"contentItems":[
        {"type":"inputText","text":json!({
            "request":response.request,"byteCount":response.byte_count,
            "content":{"kind":"image","mime":mime,"width":decoded.width(),"height":decoded.height()},
            "instruction":"Inspect the attached frozen input image. Feedback region numbers follow array order, with normalized top-left coordinates. Re-locate against current content before editing; image regions do not assert geometric hits."
        }).to_string()},
        {"type":"inputImage","imageUrl":format!("data:{mime};base64,{base64}")}
    ]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_attempt_file::{Checkpoint, Request};

    #[test]
    fn frozen_png_delivery_rejects_invalid_png_and_keeps_size_refusal_textual() -> Result<()> {
        let response = |content| Response {
            request: Request {
                project_id: "p".into(),
                run_id: "r".into(),
                attempt_id: "a".into(),
                checkpoint: Checkpoint::Input,
                path: "image.png".into(),
                sha256: "a".repeat(64),
            },
            byte_count: 9,
            content,
        };
        let invalid = response(Content::Image {
            mime: "image/png".into(),
            base64: base64::engine::general_purpose::STANDARD.encode(b"not a PNG"),
        });
        assert!(result(invalid)
            .unwrap_err()
            .to_string()
            .contains("DECODE_FAILED"));
        let refusal = result(response(Content::TooLarge))?;
        assert_eq!(refusal["contentItems"].as_array().unwrap().len(), 1);
        let metadata: Value =
            serde_json::from_str(refusal["contentItems"][0]["text"].as_str().unwrap())?;
        assert_eq!(metadata["content"]["kind"], "tooLarge");
        Ok(())
    }
}
