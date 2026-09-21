//! Resolve declared session paths using the immutable source inventory, not the old disk.
use crate::{migration_import, project_derivation_copy::Prepared, project_derivation_paths::Paths};
use anyhow::{ensure, Context, Result};
use std::path::{Path, PathBuf};

pub(crate) fn is_index(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 5
        && parts[..3] == [".beaver", "workspaces", ".codex"]
        && parts[4]
            .strip_prefix("state_")
            .and_then(|name| name.strip_suffix(".sqlite"))
            .is_some_and(|version| {
                !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit())
            })
}

pub(crate) fn target(
    prepared: &Prepared,
    database: &str,
    value: &str,
    rollout: bool,
    binding: &Path,
) -> Result<PathBuf> {
    let relative = migration_import::relative(&prepared.request.source, value)?
        .context("session path outside source project")?;
    let home = database
        .rsplit_once('/')
        .context("invalid session index")?
        .0;
    if rollout {
        ensure!(
            relative
                .to_ascii_lowercase()
                .starts_with(&format!("{}/", home.to_ascii_lowercase())),
            "session rollout outside its task home"
        );
    }
    if relative.is_empty() {
        ensure!(!rollout, "session rollout must be a file");
        return Ok(binding.into());
    }
    let mut entries = prepared
        .entries
        .iter()
        .filter(|entry| entry.path.eq_ignore_ascii_case(&relative));
    let entry = entries.next().context("session target missing from copy")?;
    ensure!(entries.next().is_none(), "ambiguous session target");
    ensure!(
        entry.sha256.is_some() == rollout,
        "session target type mismatch"
    );
    // A task may use its own workspace or project content, never a sibling task's storage.
    let parts: Vec<_> = entry.path.split('/').collect();
    if parts.starts_with(&[".beaver", "workspaces"]) && parts.len() > 2 {
        let owner = if parts[2] == ".codex" { 3 } else { 2 };
        ensure!(
            parts.get(owner) == database.split('/').nth(3).as_ref(),
            "session path belongs to another task"
        );
    }
    Ok(binding.join(Paths(&prepared.identities).relative(&entry.path)?))
}
