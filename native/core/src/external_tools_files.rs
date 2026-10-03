use super::Context;
use anyhow::{ensure, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(super) const TEXT_LIMIT: u64 = 1024 * 1024;
pub(super) const ASSET_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Output {
    pub path: String,
    // Required JSON field: null explicitly approves creating a new file only.
    pub expected_sha256: Value,
}

pub(super) fn workspace(root: &Path) -> Result<PathBuf> {
    ensure!(
        root.is_absolute(),
        "Task workspace must be host-resolved and absolute"
    );
    let mut current = PathBuf::new();
    for part in root.components() {
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        ensure!(
            !crate::files::linked(&metadata),
            "Linked workspace ancestor rejected"
        );
    }
    ensure!(root.is_dir(), "Task workspace does not exist");
    Ok(fs::canonicalize(root)?)
}

pub(super) fn path(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(relative.len() <= 1024, "Task path is too long");
    ensure!(
        !relative.split('/').any(|part| {
            crate::files::excluded_snapshot_name(part)
                || part.starts_with('.')
                || part.ends_with(['.', ' '])
                || part.chars().any(char::is_control)
        }),
        "Reserved task path rejected"
    );
    let path = crate::files::safe_path(root, relative)?;
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        ensure!(metadata.is_file(), "Task path is not a regular file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(metadata.nlink() == 1, "Hard-linked task file rejected");
        }
    }
    Ok(path)
}

pub(super) fn expected(value: &Value) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    let hash = value
        .as_str()
        .context("expectedSha256 must be null or SHA-256")?;
    ensure!(
        hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Invalid expected SHA-256"
    );
    Ok(Some(hash.into()))
}

pub(super) fn verify(root: &Path, output: &Output) -> Result<PathBuf> {
    ensure!(
        !output.path.eq_ignore_ascii_case("beaver.runtime.json"),
        "The host-owned workflow runtime configuration is read-only"
    );
    let path = path(root, &output.path)?;
    ensure!(
        crate::files::file_hash_limited(&path, Some(ASSET_LIMIT))?
            == expected(&output.expected_sha256)?,
        "File changed or overwrite not approved: {}",
        output.path
    );
    Ok(path)
}

pub(super) fn publish(root: &Path, output: &Output, source: &Path) -> Result<Value> {
    let target = verify(root, output)?;
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        metadata.is_file() && !crate::files::linked(&metadata) && metadata.len() <= ASSET_LIMIT,
        "Invalid or oversized staged output"
    );
    let parent = target.parent().context("Missing output parent")?;
    fs::create_dir_all(parent)?;
    let target = verify(root, output)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".beaver-write-")
        .tempfile_in(parent)?;
    let bytes = std::io::copy(
        &mut fs::File::open(source)?.take(ASSET_LIMIT + 1),
        &mut temp,
    )?;
    ensure!(bytes <= ASSET_LIMIT, "Output exceeds size limit");
    temp.as_file().sync_all()?;
    let hash = crate::files::file_hash_limited(temp.path(), Some(ASSET_LIMIT))?;
    verify(root, output)?;
    if output.expected_sha256.is_null() {
        temp.persist_noclobber(&target)
            .map_err(|error| error.error)?;
    } else {
        temp.persist(&target).map_err(|error| error.error)?;
    }
    Ok(json!({"path":output.path,"sha256":hash,"bytes":bytes}))
}

pub(super) fn read(context: &Context, arguments: &Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Arguments {
        path: String,
    }
    let args: Arguments = serde_json::from_value(arguments.clone())?;
    let path = path(&context.workspace, &args.path)?;
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(TEXT_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= TEXT_LIMIT, "Read exceeds 1 MiB");
    let hash = super::blender::digest(&bytes);
    let content = String::from_utf8(bytes).context("file.read requires UTF-8 text")?;
    Ok(json!({"path":args.path,"content":content,"sha256":hash,"bytes":content.len()}))
}

pub(super) fn write(
    context: &Context,
    arguments: &Value,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Arguments {
        path: String,
        content: String,
        expected_sha256: Value,
    }
    ensure!(
        arguments.get("expectedSha256").is_some(),
        "expectedSha256 is required (null for a new file)"
    );
    let args: Arguments = serde_json::from_value(arguments.clone())?;
    ensure!(
        args.content.len() as u64 <= TEXT_LIMIT,
        "Write exceeds 1 MiB"
    );
    let output = Output {
        path: args.path,
        expected_sha256: args.expected_sha256,
    };
    verify(&context.workspace, &output)?;
    let parent = path(&context.workspace, &output.path)?
        .parent()
        .unwrap()
        .to_owned();
    fs::create_dir_all(&parent)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".beaver-write-")
        .tempfile_in(&parent)?;
    temp.write_all(args.content.as_bytes())?;
    ensure!(
        !cancelled.load(std::sync::atomic::Ordering::SeqCst),
        "File write cancelled before publication"
    );
    publish(&context.workspace, &output, temp.path())
}
