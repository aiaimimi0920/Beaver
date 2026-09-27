//! Read only immutable checkpoint members, never the mutable task workspace.
use crate::{object_attempt_view, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Read};

pub const INLINE_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Checkpoint {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub checkpoint: Checkpoint,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub request: Request,
    pub byte_count: u64,
    pub content: Content,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    Text { text: String },
    Image { mime: String, base64: String },
    Audio { mime: String, base64: String },
    Binary,
    TooLarge,
}

pub fn read(runtime: &ProjectRuntime, request: &Request) -> Result<Response> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let attempt = {
        let handle = runtime.store();
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("attempt file store lock poisoned"))?;
        object_attempt_view::read(&store.connection, &request.project_id, &request.attempt_id)?
    };
    ensure!(
        attempt.preparation.run.id == request.run_id,
        "OBJECT_ATTEMPT_FILE_RUN_MISMATCH"
    );
    let snapshot = match request.checkpoint {
        Checkpoint::Input => &attempt.input,
        Checkpoint::Output => attempt
            .output
            .as_ref()
            .context("OBJECT_ATTEMPT_FILE_OUTPUT_UNAVAILABLE")?,
    };
    let hash = snapshot
        .get(&request.path)
        .context("OBJECT_ATTEMPT_FILE_NOT_IN_CHECKPOINT")?;
    ensure!(hash == &request.sha256, "OBJECT_ATTEMPT_FILE_STALE_HASH");
    let blob = runtime.files().blob(hash)?;
    let metadata = fs::symlink_metadata(&blob).context("OBJECT_ATTEMPT_FILE_BLOB_UNAVAILABLE")?;
    ensure!(
        !crate::files::linked(&metadata) && metadata.is_file(),
        "OBJECT_ATTEMPT_FILE_UNSAFE_BLOB"
    );
    let content = if metadata.len() > INLINE_LIMIT as u64 {
        Content::TooLarge
    } else {
        let mut bytes = Vec::new();
        fs::File::open(&blob)?
            .take(INLINE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= INLINE_LIMIT,
            "OBJECT_ATTEMPT_FILE_GREW_TOO_LARGE"
        );
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == *hash,
            "OBJECT_ATTEMPT_FILE_HASH_MISMATCH"
        );
        match crate::frozen_media::classify(&request.path, &bytes) {
            Some(crate::frozen_media::Media::Image { mime, base64 }) => {
                Content::Image { mime, base64 }
            }
            Some(crate::frozen_media::Media::Audio { mime, base64 }) => {
                Content::Audio { mime, base64 }
            }
            None => match String::from_utf8(bytes) {
                Ok(text)
                    if !text
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) =>
                {
                    Content::Text { text }
                }
                _ => Content::Binary,
            },
        }
    };
    Ok(Response {
        request: request.clone(),
        byte_count: metadata.len(),
        content,
    })
}
