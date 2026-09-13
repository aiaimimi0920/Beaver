use super::{model::Run, repository};
use crate::files::{file_hash, safe_path, Files, Snapshot};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

pub fn freeze_feedback(files: &Files, workspace: &Path, run: &Run, record: &Value) -> Result<()> {
    let directory = safe_path(workspace, ".beaver-context/validation")?;
    freeze(files, &directory, run, "feedback.json", record)
}

pub fn freeze_repair(files: &Files, workspace: &Path, run: &Run, record: &Value) -> Result<()> {
    uuid::Uuid::parse_str(&run.id).context("Invalid repair run ID")?;
    let directory = safe_path(
        workspace,
        &format!(".beaver-context/validation/repairs/{}", run.id),
    )?;
    freeze(files, &directory, run, "repair.json", record)
}

fn freeze(files: &Files, directory: &Path, run: &Run, name: &str, record: &Value) -> Result<()> {
    fs::create_dir_all(&directory)?;
    fs::write(
        safe_path(directory, name)?,
        serde_json::to_vec_pretty(record)?,
    )?;
    fs::write(
        safe_path(directory, "run.json")?,
        serde_json::to_vec_pretty(run)?,
    )?;
    fs::write(safe_path(directory, "run.log")?, &run.log)?;
    let media = safe_path(directory, "media")?;
    fs::create_dir_all(&media)?;
    let original = repository::run_dir(files.root(), &run.id)?;
    for evidence in &run.evidence {
        let path = safe_path(&original, &evidence.file)?;
        ensure!(
            file_hash(&path)?.as_ref() == Some(&evidence.sha256),
            "Evidence changed before feedback was captured"
        );
        let suffix = if evidence.kind == "image" {
            "png"
        } else {
            "webm"
        };
        uuid::Uuid::parse_str(&evidence.id).context("Invalid evidence ID")?;
        fs::copy(
            path,
            safe_path(&media, &format!("{}.{suffix}", evidence.id))?,
        )?;
    }
    let mut sources = Snapshot::new();
    let refs = run
        .evidence
        .iter()
        .flat_map(|e| &e.references)
        .chain(run.flow.iter().flat_map(|f| &f.definition.references));
    for reference in refs {
        let path = super::flow::relative(&reference.path)?;
        if let Some(hash) = run.snapshot.get(path) {
            sources.insert(path.into(), hash.clone());
        }
    }
    for case in run.code.iter().flat_map(|c| &c.cases) {
        if let Ok(path) = super::flow::relative(&case.file) {
            if let Some(hash) = run.snapshot.get(path) {
                sources.insert(path.into(), hash.clone());
            }
        }
    }
    // Failed integration tests also need their candidate, not only the older task workspace.
    if run.kind == "code" {
        sources = run.snapshot.clone();
    }
    files.restore_copy(&sources, &safe_path(directory, "source")?)?;
    Ok(())
}
