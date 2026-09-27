use super::{Request, Rule, Source};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileChange {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub owners: Vec<String>,
    pub reference: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stage {
    pub task_id: String,
    pub title: String,
    pub revision: u64,
    pub status: String,
    pub acceptance: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub object_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub schema_version: u32,
    pub request: Request,
    pub time: String,
    pub source_digest: String,
    pub output_digest: String,
    pub baseline_version_id: Option<String>,
    pub accepted_version_id_at_claim: Option<String>,
    pub accepted_version_id_at_review: Option<String>,
    pub object_revision_at_claim: u64,
    pub object_revision_at_review: u64,
    pub stages: Vec<Stage>,
    pub references: Vec<Reference>,
    pub files: Vec<FileChange>,
    pub rules: Vec<Rule>,
    pub blockers: Vec<String>,
}

pub(super) fn build(
    source: &Source,
    request: &Request,
    rules: Vec<Rule>,
    time: String,
    schema_version: u32,
) -> Result<Report> {
    ensure!(
        matches!(schema_version, 1 | 2),
        "OBJECT_CANDIDATE_SCHEMA_UNSUPPORTED"
    );
    let records = &source.records;
    let baseline = records
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_BASELINE_REQUIRED")?;
    let before = crate::object_run_baseline::snapshot(&baseline.versions)?;
    let attempt = records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    let after = attempt
        .output
        .as_ref()
        .context("OBJECT_CHECK_OUTPUT_REQUIRED")?;
    // Reconstruct v1 exactly when verifying immutable historical receipts.
    let mut blockers = if schema_version == 1 {
        vec![
            "FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED".into(),
            "FEEDBACK_REVIEW_UNAVAILABLE".into(),
            "PUBLICATION_NOT_IMPLEMENTED".into(),
        ]
    } else {
        vec![
            "FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED".into(),
            "PUBLICATION_CONFIRMATION_REQUIRED".into(),
        ]
    };
    if rules.iter().any(|rule| !rule.passed) {
        blockers.push("TECHNICAL_CHECK_FAILED".into());
    }
    if baseline.accepted_version_id_at_claim != records.accepted_version_id {
        blockers.push("ACCEPTED_VERSION_DRIFT".into());
    }
    if baseline.object_revision_at_claim != records.object.revision {
        blockers.push("OBJECT_REVISION_DRIFT".into());
    }
    let references: Vec<_> = baseline
        .versions
        .iter()
        .filter(|version| version.object_id != records.object.id)
        .map(|version| Reference {
            object_id: version.object_id.clone(),
            version_id: version.version_id.clone(),
        })
        .collect();
    if records.object.references.iter().any(|reference| {
        !references.iter().any(|frozen| {
            frozen.object_id == reference.object_id
                && Some(&frozen.version_id) == reference.version_id.as_ref()
        })
    }) {
        blockers.push("REFERENCE_CLOSURE_REVIEW_REQUIRED".into());
    }
    let mut files = Vec::new();
    for path in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let owners: Vec<_> = source
            .objects
            .iter()
            .filter(|object| {
                object
                    .files
                    .iter()
                    .any(|file| file.path.to_lowercase() == path.to_lowercase())
            })
            .map(|object| object.id.clone())
            .collect();
        let reference = baseline.versions.iter().any(|version| {
            version.object_id != records.object.id
                && version
                    .files
                    .iter()
                    .any(|file| file.path.to_lowercase() == path.to_lowercase())
        });
        // Show unchanged files too: ownership and reference closure cover the whole candidate.
        if !reference && (owners.len() != 1 || owners[0] != records.object.id) {
            blockers.push(format!("FILE_OWNERSHIP_REVIEW_REQUIRED: {path}"));
        }
        if reference && before.get(path) != after.get(path) {
            blockers.push(format!("FIXED_REFERENCE_CHANGED: {path}"));
        }
        files.push(FileChange {
            path: path.clone(),
            before: before.get(path).cloned(),
            after: after.get(path).cloned(),
            owners,
            reference,
        });
    }
    Ok(Report {
        schema_version,
        request: request.clone(),
        time,
        source_digest: crate::framework_checks::digest(source)?,
        output_digest: crate::framework_checks::digest(after)?,
        baseline_version_id: baseline.resolved_version_id.clone(),
        accepted_version_id_at_claim: baseline.accepted_version_id_at_claim.clone(),
        accepted_version_id_at_review: records.accepted_version_id.clone(),
        object_revision_at_claim: baseline.object_revision_at_claim,
        object_revision_at_review: records.object.revision,
        stages: source
            .fines
            .iter()
            .map(|fine| Stage {
                task_id: fine.id.clone(),
                title: fine.title.clone(),
                revision: fine.revision,
                status: fine.status.clone(),
                acceptance: fine.acceptance.clone(),
            })
            .collect(),
        references,
        files,
        rules,
        blockers,
    })
}
