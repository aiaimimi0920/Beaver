//! Point copied Codex indexes at their partition; JSONL history stays untouched.
use crate::{
    files::safe_path,
    migration_bundle::{RestoreReceipt, RestoredProject},
    project_migration_plan::{relocate, ProjectPlan},
    project_migration_sessions::PathRecord,
};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::{collections::BTreeMap, path::PathBuf};

const DATA: &str = "application/data/";

fn target_path(
    record: &PathRecord,
    receipt: &RestoreReceipt,
    project: &RestoredProject,
    plan: &ProjectPlan,
) -> Result<PathBuf> {
    let target = &record.target_archive_path;
    if let Some(relative) = target.strip_prefix(DATA) {
        ensure!(
            plan.files.iter().any(|file| file.source == relative),
            "session target was not moved into project {}: {relative}",
            project.id
        );
        let located = relocate(relative).context("session target has no project location")?;
        return safe_path(&project.restored_path, &located);
    }
    let rest = target
        .strip_prefix("projects/")
        .context("session target is outside the archive")?;
    let (id, relative) = rest.split_once('/').unwrap_or((rest, ""));
    let owner = receipt
        .projects
        .iter()
        .find(|candidate| candidate.id == id)
        .context("session target names an unarchived project")?;
    if relative.is_empty() {
        Ok(owner.restored_path.clone())
    } else {
        safe_path(&owner.restored_path, relative)
    }
}

/// Only the rows the inventory already verified are rewritten, one index at a time.
pub(crate) fn rewrite(
    records: &[PathRecord],
    receipt: &RestoreReceipt,
    project: &RestoredProject,
    plan: &ProjectPlan,
) -> Result<usize> {
    let mut by_index: BTreeMap<&str, Vec<&PathRecord>> = BTreeMap::new();
    for record in records {
        let Some(relative) = record.index_path.strip_prefix(DATA) else {
            continue;
        };
        if plan.files.iter().any(|file| file.source == relative) {
            by_index.entry(relative).or_default().push(record);
        }
    }
    let mut changed = 0;
    for (relative, records) in by_index {
        let located = relocate(relative).context("session index has no project location")?;
        let database = safe_path(&project.restored_path, &located)?;
        let mut connection =
            Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        connection.execute_batch("PRAGMA trusted_schema=OFF;")?;
        let transaction = connection.transaction()?;
        for record in records {
            let next = target_path(record, receipt, project, plan)?;
            ensure!(
                if record.column == "rollout_path" {
                    next.is_file()
                } else {
                    next.is_dir()
                },
                "session target missing after partitioning: {}",
                record.target_archive_path
            );
            let updated = transaction.execute(
                &format!(
                    "UPDATE {} SET {}=? WHERE rowid=?",
                    record.table, record.column
                ),
                params![next.to_string_lossy().into_owned(), record.rowid],
            )?;
            ensure!(
                updated == 1,
                "session index row vanished during partitioning"
            );
            changed += 1;
        }
        transaction.commit()?;
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    }
    Ok(changed)
}
