//! Materialize one new run workspace without overwriting retained work.
use super::Preparation;
use crate::{files, object_import_content, object_run_baseline, project_runtime::ProjectRuntime};
use anyhow::{ensure, Context, Result};
use std::{fs, io::ErrorKind};

pub(super) fn create(runtime: &ProjectRuntime, record: &Preparation) -> Result<()> {
    let baseline = record
        .baseline
        .as_ref()
        .context("OBJECT_RUN_BASELINE_MISSING")?;
    let files = runtime.files();
    ensure!(
        files.workspace_location(&record.run.id)? == record.workspace,
        "OBJECT_RUN_WORKSPACE_MISMATCH"
    );
    let workspace = files.workspace(&record.run.id)?;
    let snapshot = object_run_baseline::snapshot(&baseline.versions)?;
    object_import_content::verify(runtime.project_root(), &baseline.versions)?;
    fs::create_dir_all(
        workspace
            .parent()
            .context("OBJECT_RUN_WORKSPACE_PARENT_MISSING")?,
    )?;
    fs::create_dir(&workspace).map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            anyhow::anyhow!("OBJECT_RUN_WORKSPACE_ALREADY_EXISTS")
        } else {
            anyhow::anyhow!(error).context("OBJECT_RUN_WORKSPACE_CREATE_FAILED")
        }
    })?;
    files.restore_copy(&snapshot, &workspace)?;
    // Verify the copied bytes too, including source changes during the copy.
    for version in &baseline.versions {
        for file in &version.files {
            let path = files::safe_path(&workspace, &file.path)?;
            ensure!(
                fs::metadata(&path)?.len() == file.bytes
                    && files::file_hash_limited(&path, Some(file.bytes))?.as_deref()
                        == Some(file.sha256.as_str()),
                "OBJECT_RUN_WORKSPACE_CONTENT_MISMATCH: {}",
                file.path
            );
        }
    }
    Ok(())
}
