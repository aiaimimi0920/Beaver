use crate::{
    asset_delivery_files as artifacts,
    asset_task::State,
    files::{Files, Snapshot},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Source,
    Dependency,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declaration {
    pub path: String,
    pub role: Role,
    #[serde(default)]
    pub expected_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captured {
    pub path: String,
    pub role: Role,
    pub sha256: String,
}

pub fn capture(files: &Files, workspace: &Path, inputs: &[Declaration]) -> Result<Vec<Captured>> {
    ensure!(inputs.len() <= 32, "Declare at most 32 input files");
    let mut seen = HashSet::new();
    for input in inputs {
        ensure!(
            seen.insert(input.path.to_ascii_lowercase()),
            "Duplicate input path"
        );
        if let Some(hash) = &input.expected_sha256 {
            ensure!(
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                "expectedSha256 must be a lowercase SHA-256"
            );
        }
    }
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let paths = inputs.iter().map(|i| i.path.clone()).collect::<Vec<_>>();
    let snapshot =
        artifacts::capture(files, workspace, &paths).context("Cannot capture work inputs")?;
    inputs
        .iter()
        .map(|input| {
            let hash = snapshot
                .get(&input.path)
                .context("Missing captured input")?;
            ensure!(
                input
                    .expected_sha256
                    .as_ref()
                    .is_none_or(|expected| expected == hash),
                "Input version changed: {}",
                input.path
            );
            Ok(Captured {
                path: input.path.clone(),
                role: input.role.clone(),
                sha256: hash.clone(),
            })
        })
        .collect()
}

/// Source bytes are frozen evidence but may be edited. Dependencies must remain unchanged.
pub fn verify(files: &Files, workspace: &Path, inputs: &[Captured]) -> Result<()> {
    for input in inputs {
        let snapshot = Snapshot::from([(input.path.clone(), input.sha256.clone())]);
        if input.role == Role::Dependency {
            artifacts::verify_workspace(files, workspace, &snapshot).with_context(|| {
                format!("Work dependency changed or unavailable: {}", input.path)
            })?;
        } else {
            artifacts::verify(files, &snapshot)
                .with_context(|| format!("Frozen work source is invalid: {}", input.path))?;
        }
    }
    Ok(())
}

/// Preserve separate versions when a path appears in several attempts; never last-write-win.
pub fn for_attempts(state: &State, ids: &[String]) -> Result<Vec<Captured>> {
    let mut inputs = Vec::new();
    for id in ids {
        let attempt = state
            .work
            .attempts
            .iter()
            .find(|a| &a.id == id)
            .context("Missing execution attempt")?;
        if let Some(manifest) = &attempt.input_files {
            inputs.extend(manifest.iter().cloned());
        }
    }
    Ok(inputs)
}
