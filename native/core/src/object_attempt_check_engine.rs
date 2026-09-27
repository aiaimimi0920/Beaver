//! Built-in checks against frozen bytes and the original run baseline, including on retries.
use super::Rule;
use crate::{
    code_structure,
    files::{self, Change, Snapshot},
    object_attempt::Attempt,
    object_version_manifest::{valid_asset_path, valid_digest},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeSet, fs};

pub(crate) fn verify(runtime: &ProjectRuntime, snapshot: &Snapshot) -> Result<()> {
    let mut paths = BTreeSet::new();
    ensure!(
        snapshot.iter().all(|(path, hash)| valid_asset_path(path)
            && valid_digest(hash)
            && paths.insert(path.to_lowercase())),
        "OBJECT_CHECK_INVALID_MANIFEST"
    );
    for hash in snapshot.values().collect::<BTreeSet<_>>() {
        let path = runtime.files().blob(hash)?;
        let metadata = fs::symlink_metadata(&path).context("OBJECT_CHECK_CONTENT_MISSING")?;
        ensure!(
            metadata.is_file() && !files::linked(&metadata),
            "OBJECT_CHECK_INVALID_BLOB"
        );
        ensure!(
            files::file_hash_limited(&path, Some(512 * 1024 * 1024))?.as_deref() == Some(hash),
            "OBJECT_CHECK_HASH_MISMATCH"
        );
    }
    Ok(())
}

fn baseline(attempt: &Attempt) -> Result<Snapshot> {
    let baseline = attempt
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_RUN_BASELINE_MISSING")?;
    ensure!(
        crate::object_import_snapshot::digest(&baseline.versions)? == baseline.content_digest,
        "OBJECT_CHECK_BASELINE_DIGEST_MISMATCH"
    );
    crate::object_run_baseline::snapshot(&baseline.versions)
}

fn integrity(runtime: &ProjectRuntime, attempt: &Attempt, baseline: &Snapshot) -> Result<()> {
    verify(runtime, baseline)?;
    verify(runtime, &attempt.input)?;
    verify(
        runtime,
        attempt
            .output
            .as_ref()
            .context("OBJECT_CHECK_OUTPUT_REQUIRED")?,
    )
}

pub(super) fn run(runtime: &ProjectRuntime, attempt: &Attempt) -> Vec<Rule> {
    let mut integrity_rule = Rule {
        id: "checkpoint-integrity".into(),
        version: 1,
        passed: false,
        issues: vec![],
        files_checked: 0,
    };
    let mut structure_rule = Rule {
        id: "code-structure".into(),
        version: 1,
        passed: false,
        issues: vec![],
        files_checked: 0,
    };
    let outcome = (|| -> Result<()> {
        let baseline = baseline(attempt)?;
        integrity(runtime, attempt, &baseline)?;
        integrity_rule.passed = true;
        integrity_rule.files_checked =
            baseline.len() + attempt.input.len() + attempt.output.as_ref().unwrap().len();
        let output = attempt.output.as_ref().unwrap();
        let changes: Vec<_> = baseline
            .keys()
            .chain(output.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|path| baseline.get(*path) != output.get(*path))
            .map(|path| Change {
                path: path.clone(),
                before: baseline.get(path).cloned(),
                after: output.get(path).cloned(),
            })
            .collect();
        match code_structure::check_changes(&runtime.files(), &baseline, &changes) {
            Ok(report) => {
                structure_rule.passed = report.ok;
                structure_rule.files_checked = report.files.len();
                structure_rule.issues = report.violations;
            }
            Err(error) => structure_rule.issues.push(format!("{error:#}")),
        }
        // Detect mutations during the structure reader as well as before it.
        integrity(runtime, attempt, &baseline)
    })();
    if let Err(error) = outcome {
        integrity_rule.passed = false;
        integrity_rule.issues.push(format!("{error:#}"));
        if structure_rule.issues.is_empty() && !structure_rule.passed {
            structure_rule
                .issues
                .push("OBJECT_CHECK_INTEGRITY_REQUIRED".into());
        }
    }
    vec![integrity_rule, structure_rule]
}
