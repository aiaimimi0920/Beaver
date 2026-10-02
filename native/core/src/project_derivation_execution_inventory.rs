//! Consume the source-lock inventory, using the same visible-file rules as checkpoint capture.
use crate::{
    data_backup::Entry, object_attempt::Attempt, object_version_manifest::valid_digest,
    project_derivation_paths::valid_relative_path,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate(connection: &Connection, entries: &[Entry]) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut inventory = BTreeMap::new();
    for entry in entries {
        ensure!(
            valid_relative_path(&entry.path) && names.insert(entry.path.to_lowercase()),
            "DERIVATION_EXECUTION_INVENTORY_ALIAS"
        );
        inventory.insert(entry.path.as_str(), entry);
    }
    let mut statement =
        connection.prepare("SELECT value FROM entities WHERE kind='object_attempt'")?;
    let mut runs = BTreeMap::<String, Vec<Attempt>>::new();
    for row in statement.query_map([], |row| row.get::<_, String>(0))? {
        let attempt: Attempt = serde_json::from_str(&row?)?;
        for hash in attempt
            .input
            .values()
            .chain(attempt.output.iter().flat_map(|s| s.values()))
        {
            ensure!(valid_digest(hash), "OBJECT_RECOVERY_INVALID_CHECKPOINT");
            let path = format!(".beaver/content/blobs/{hash}");
            let blob = inventory
                .get(path.as_str())
                .context("DERIVATION_EXECUTION_BLOB_MISSING")?;
            ensure!(
                blob.sha256.as_ref() == Some(hash),
                "DERIVATION_EXECUTION_BLOB_MISMATCH"
            );
        }
        runs.entry(attempt.preparation.run.id.clone())
            .or_default()
            .push(attempt);
    }
    for attempts in runs.into_values() {
        // Revisions are local to a fine, not a clock for the entire run.
        let chain = crate::object_run_recovery::resume::chain(connection, attempts)?;
        let attempt = chain
            .last()
            .context("DERIVATION_EXECUTION_ATTEMPT_REQUIRED")?;
        let root = format!(".beaver/workspaces/{}", attempt.preparation.run.id);
        ensure!(
            attempt.preparation.workspace == root,
            "OBJECT_RUN_WORKSPACE_MISMATCH"
        );
        ensure!(
            inventory
                .get(root.as_str())
                .is_some_and(|e| e.sha256.is_none()),
            "DERIVATION_EXECUTION_WORKSPACE_MISSING"
        );
        let prefix = format!("{root}/");
        let mut visible = crate::files::Snapshot::new();
        for entry in entries.iter().filter(|e| e.sha256.is_some()) {
            let Some(relative) = entry.path.strip_prefix(&prefix) else {
                continue;
            };
            if !relative
                .split('/')
                .any(crate::files::excluded_snapshot_name)
            {
                visible.insert(relative.to_owned(), entry.sha256.clone().unwrap());
            }
        }
        ensure!(
            attempt.output.as_ref() == Some(&visible),
            "DERIVATION_EXECUTION_WORKSPACE_DRIFT"
        );
    }
    Ok(())
}
