use super::attempt_fixture::{attempts, fixture, start, workspace};
use crate::{
    object_attempt::{self, State},
    object_attempt_file::{self as files, Checkpoint, Content, Request},
};
use anyhow::Result;
use std::sync::atomic::AtomicBool;

#[test]
fn frozen_media_survives_live_deletion_for_both_checkpoints_and_rejects_tampering() -> Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let f = fixture()?;
    let lease = start(&f)?;
    let root = workspace(&f, lease.record())?;
    let samples: &[(&str, &[u8], &str, &str)] = &[
        (
            "image.PNG",
            &[137, 80, 78, 71, 13, 10, 26, 10],
            "image",
            "image/png",
        ),
        ("sound.wav", b"RIFF\0\0\0\0WAVE", "audio", "audio/wav"),
    ];
    for (path, bytes, _, _) in samples {
        std::fs::write(root.join(path), bytes)?;
    }
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let mut attempt = attempts(&f, "head")?.remove(0);
    attempt.input = attempt.output.clone().unwrap();
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_attempt", &attempt.id, &attempt)?;
    for (path, bytes, kind, mime) in samples {
        std::fs::write(root.join(path), b"modified live content")?;
        std::fs::remove_file(root.join(path))?;
        let mut request = Request {
            project_id: "project-1".into(),
            run_id: attempt.preparation.run.id.clone(),
            attempt_id: attempt.id.clone(),
            checkpoint: Checkpoint::Input,
            path: (*path).into(),
            sha256: attempt.input[*path].clone(),
        };
        for checkpoint in [Checkpoint::Input, Checkpoint::Output] {
            request.checkpoint = checkpoint;
            let response = serde_json::to_value(files::read(&f.runtime, &request)?)?;
            assert_eq!(response["request"], serde_json::to_value(&request)?);
            assert_eq!(response["byteCount"], bytes.len());
            assert_eq!(
                response["content"],
                serde_json::json!({
                    "kind": kind, "mime": mime, "base64": STANDARD.encode(bytes)
                })
            );
        }
        std::fs::write(f.runtime.files().blob(&request.sha256)?, b"corrupt")?;
        assert!(files::read(&f.runtime, &request)
            .unwrap_err()
            .to_string()
            .contains("HASH_MISMATCH"));
    }
    Ok(())
}

#[test]
fn frozen_content_survives_workspace_removal_and_rejects_foreign_selections() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    let path = workspace(&f, lease.record())?;
    std::fs::write(path.join("hero.gd"), "extends Node\n")?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    let request = Request {
        project_id: "project-1".into(),
        run_id: attempt.preparation.run.id.clone(),
        attempt_id: attempt.id.clone(),
        checkpoint: Checkpoint::Output,
        path: "hero.gd".into(),
        sha256: attempt.output.as_ref().unwrap()["hero.gd"].clone(),
    };
    std::fs::write(path.join("hero.gd"), "changed")?;
    std::fs::remove_file(path.join("hero.gd"))?;
    assert_eq!(
        files::read(&f.runtime, &request)?.content,
        Content::Text {
            text: "extends Node\n".into()
        }
    );
    // A persisted input checkpoint uses the same content-addressed storage.
    let mut historical = attempt.clone();
    historical.input = attempt.output.clone().unwrap();
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_attempt", &historical.id, &historical)?;
    assert_eq!(
        files::read(
            &f.runtime,
            &Request {
                checkpoint: Checkpoint::Input,
                ..request.clone()
            }
        )?
        .content,
        Content::Text {
            text: "extends Node\n".into()
        }
    );
    for invalid in [
        Request {
            project_id: "other".into(),
            ..request.clone()
        },
        Request {
            run_id: "other".into(),
            ..request.clone()
        },
        Request {
            attempt_id: "other".into(),
            ..request.clone()
        },
        Request {
            path: "../hero.gd".into(),
            ..request.clone()
        },
        Request {
            sha256: "a".repeat(64),
            ..request.clone()
        },
    ] {
        assert!(files::read(&f.runtime, &invalid).is_err());
    }
    let blob = f.runtime.files().blob(&request.sha256)?;
    std::fs::write(&blob, "corrupt")?;
    assert!(files::read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("HASH_MISMATCH"));
    std::fs::remove_file(blob)?;
    assert!(files::read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("BLOB_UNAVAILABLE"));
    Ok(())
}

#[test]
fn binary_large_and_unfrozen_output_have_distinct_results() -> Result<()> {
    let f = fixture()?;
    let lease = start(&f)?;
    let path = workspace(&f, lease.record())?;
    let mut request = Request {
        project_id: "project-1".into(),
        run_id: lease.record().preparation.run.id.clone(),
        attempt_id: lease.record().id.clone(),
        checkpoint: Checkpoint::Output,
        path: "binary.bin".into(),
        sha256: "a".repeat(64),
    };
    assert!(files::read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OUTPUT_UNAVAILABLE"));
    std::fs::write(path.join("binary.bin"), [0, 255, 12])?;
    std::fs::write(path.join("large.txt"), vec![b'a'; files::INLINE_LIMIT + 1])?;
    std::fs::write(path.join("empty.txt"), "")?;
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let attempt = attempts(&f, "head")?.remove(0);
    for (path, expected) in [
        ("binary.bin", Content::Binary),
        ("large.txt", Content::TooLarge),
        (
            "empty.txt",
            Content::Text {
                text: String::new(),
            },
        ),
    ] {
        request.path = path.into();
        request.sha256 = attempt.output.as_ref().unwrap()[path].clone();
        assert_eq!(files::read(&f.runtime, &request)?.content, expected);
    }
    Ok(())
}
