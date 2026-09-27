//! Bounded, verified reads of immutable version members, never live project files.
use crate::{
    object_catalog::{self, ObjectVersion},
    object_version_manifest,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Read};

pub const INLINE_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub object_id: String,
    pub version_id: String,
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
    Unsupported,
    TooLarge,
}

pub fn read(runtime: &ProjectRuntime, request: &Request) -> Result<Response> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let manifest = {
        let handle = runtime.store();
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("version file store lock poisoned"))?;
        let object =
            object_catalog::get(&store, &request.object_id)?.context("OBJECT_NOT_FOUND")?;
        ensure!(
            object.project_id == request.project_id,
            "PROJECT_RUNTIME_MISMATCH"
        );
        let mut versions = object
            .versions
            .iter()
            .filter(|v| v.version_id == request.version_id);
        let version = versions.next().context("OBJECT_VERSION_NOT_FOUND")?;
        ensure!(versions.next().is_none(), "DUPLICATE_OBJECT_VERSION");
        let saved: (String, ObjectVersion) = store
            .get("object_version", &request.version_id)?
            .context("OBJECT_VERSION_NOT_FOUND")?;
        ensure!(
            saved == (object.id.clone(), version.clone()),
            "OBJECT_VERSION_MISMATCH"
        );
        object_version_manifest::read_frozen_version(&object, version)?
    };
    let file = manifest
        .files
        .iter()
        .find(|file| file.path == request.path)
        .context("OBJECT_VERSION_FILE_NOT_IN_MANIFEST")?;
    ensure!(
        file.sha256 == request.sha256,
        "OBJECT_VERSION_FILE_STALE_HASH"
    );
    let blob = runtime.files().blob(&file.sha256)?;
    let metadata = fs::symlink_metadata(&blob).context("OBJECT_VERSION_FILE_BLOB_UNAVAILABLE")?;
    ensure!(
        !crate::files::linked(&metadata) && metadata.is_file(),
        "OBJECT_VERSION_FILE_UNSAFE_BLOB"
    );
    ensure!(
        metadata.len() == file.bytes,
        "OBJECT_VERSION_FILE_SIZE_MISMATCH"
    );
    let content = if file.bytes > INLINE_LIMIT as u64 {
        Content::TooLarge
    } else {
        let mut bytes = Vec::new();
        fs::File::open(&blob)?
            .take(INLINE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 == file.bytes,
            "OBJECT_VERSION_FILE_SIZE_MISMATCH"
        );
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == file.sha256,
            "OBJECT_VERSION_FILE_HASH_MISMATCH"
        );
        content(&request.path, bytes)
    };
    Ok(Response {
        request: request.clone(),
        byte_count: file.bytes,
        content,
    })
}

fn content(path: &str, bytes: Vec<u8>) -> Content {
    match crate::frozen_media::classify(path, &bytes) {
        Some(crate::frozen_media::Media::Image { mime, base64 }) => {
            return Content::Image { mime, base64 }
        }
        Some(crate::frozen_media::Media::Audio { mime, base64 }) => {
            return Content::Audio { mime, base64 }
        }
        None => {}
    }
    match String::from_utf8(bytes) {
        Ok(text)
            if !text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) =>
        {
            Content::Text { text }
        }
        _ => Content::Unsupported,
    }
}

#[cfg(test)]
#[path = "object_version_file_tests.rs"]
mod tests;
