use super::{
    check, is_source, measure, read, vendor, Baseline, Exceptions, Report, EXCEPTIONS_FILE,
    MAX_SOURCE_BYTES,
};
use crate::files::{Change, Files, Snapshot};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// Read source blobs only. Large game assets never enter the line checker.
pub fn snapshot_baseline(files: &Files, snapshot: &Snapshot) -> Result<Baseline> {
    let mut baseline = Baseline::empty();
    for (path, hash) in snapshot.iter().filter(|(path, _)| is_source(path)) {
        let bytes = read(&files.blob(hash)?, MAX_SOURCE_BYTES)?;
        let stamp = measure(path, &bytes)?;
        if stamp.effective_lines > 500 && !vendor::immutable(path, &bytes) {
            baseline.files.insert(path.clone(), stamp);
        }
    }
    Ok(baseline)
}

/// Finalization and merge retry both inspect the captured bytes, not live workspace files.
pub fn check_changes(files: &Files, baseline: &Snapshot, changes: &[Change]) -> Result<Report> {
    let mut after = baseline.clone();
    for change in changes {
        if let Some(hash) = &change.after {
            after.insert(change.path.clone(), hash.clone());
        } else {
            after.remove(&change.path);
        }
    }
    let exceptions = match after.get(EXCEPTIONS_FILE) {
        Some(hash) => Exceptions::parse(&read(&files.blob(hash)?, 64 * 1024)?)
            .with_context(|| format!("Invalid {EXCEPTIONS_FILE}"))?,
        None => Exceptions::default(),
    };
    let mut sources = BTreeMap::new();
    let mut debt = Baseline::empty();
    let paths: std::collections::BTreeSet<_> = changes
        .iter()
        .filter(|change| change.after.is_some() && is_source(&change.path))
        .map(|change| change.path.as_str())
        .chain(exceptions.files.keys().map(String::as_str))
        .collect();
    for path in paths {
        let Some(hash) = after.get(path) else {
            continue;
        };
        if !is_source(path) {
            continue;
        }
        let bytes = read(&files.blob(hash)?, MAX_SOURCE_BYTES)?;
        let stamp = measure(path, &bytes)?;
        if stamp.effective_lines > 500 {
            if let Some(old) = baseline.get(path) {
                debt.files.insert(
                    path.to_owned(),
                    measure(path, &read(&files.blob(old)?, MAX_SOURCE_BYTES)?)?,
                );
            }
        }
        sources.insert(path.to_owned(), (stamp, vendor::immutable(path, &bytes)));
    }
    Ok(check(sources, &debt, &exceptions, false))
}
