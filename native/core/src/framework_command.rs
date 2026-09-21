use crate::{files::file_hash_limited, framework_contract::Command};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicBool, time::Duration};

/// Adapter stdout ends with BEAVER_RESULT=<JSON>. Other output is retained only as a digest.
pub fn run(
    command: &Command,
    workspace: &Path,
    context: &Value,
    cancelled: &AtomicBool,
) -> Result<Value> {
    verify(command)?;
    ensure!(
        !cancelled.load(std::sync::atomic::Ordering::SeqCst),
        "Operation cancelled"
    );
    let executable = Path::new(&command.executable);
    let directory = tempfile::tempdir()?;
    let context_path = directory.path().join("context.json");
    std::fs::write(&context_path, serde_json::to_vec(context)?)?;
    let environment = BTreeMap::from([(
        "BEAVER_CONTEXT".into(),
        context_path.to_string_lossy().into_owned(),
    )]);
    let args = command.args.iter().map(String::as_str).collect::<Vec<_>>();
    let result = crate::process::run_cancellable_env(
        executable,
        &args,
        Some(workspace),
        Duration::from_secs(command.timeout_seconds),
        cancelled,
        &environment,
    )?;
    ensure!(
        result.code == 0,
        "Adapter failed with exit code {} (output withheld; inspect the adapter locally)",
        result.code
    );
    let line = result
        .text
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("BEAVER_RESULT="))
        .context("Adapter did not return BEAVER_RESULT JSON")?;
    ensure!(line.len() <= 64 * 1024, "Adapter report exceeds 64 KiB");
    let report: Value = serde_json::from_str(line)?;
    ensure!(report.is_object(), "Adapter report must be an object");
    verify(command)?;
    Ok(
        json!({"report":report,"output":crate::call_log::summary(&json!(result.text)),"executableSha256":command.sha256,"resourceHashes":command.files,"source":"beaver-adapter","runnerVersion":1}),
    )
}

pub fn verify(command: &Command) -> Result<()> {
    command.validate()?;
    for (path, hash) in
        std::iter::once((&command.executable, &command.sha256)).chain(command.files.iter())
    {
        let path = Path::new(path);
        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            !crate::files::linked(&metadata) && metadata.is_file(),
            "Adapter files must be regular files"
        );
        ensure!(
            file_hash_limited(path, Some(512 * 1024 * 1024))?.as_ref() == Some(hash),
            "Pinned adapter file changed"
        );
    }
    Ok(())
}
