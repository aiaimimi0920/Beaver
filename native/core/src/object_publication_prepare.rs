use super::{Feedback, Preview, Request, ReviewTarget};
use crate::{
    files::{Change, Snapshot},
    object_catalog::ObjectFile,
    object_task_types::valid_id,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::BTreeSet;

pub(super) fn validate_target(
    runtime: &crate::project_runtime::ProjectRuntime,
    request: &ReviewTarget,
) -> Result<()> {
    ensure!(
        runtime.project_id() == request.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.review_request_id) && valid_id(&request.target.attempt_id),
        "INVALID_OBJECT_PUBLICATION_REQUEST"
    );
    Ok(())
}

pub(super) fn load(
    runtime: &crate::project_runtime::ProjectRuntime,
    db: &Connection,
    request: &ReviewTarget,
) -> Result<(super::Source, super::Report, Preview)> {
    let saved: super::super::Stored =
        crate::object_task_storage::read(db, super::super::KIND, &request.review_request_id)?
            .context("OBJECT_CANDIDATE_REVIEW_REQUIRED")?;
    ensure!(
        saved.report.request.project_id == request.project_id
            && saved.report.request.target == request.target,
        "OBJECT_PUBLICATION_REVIEW_MISMATCH"
    );
    let source = super::super::read(db, &saved.report.request)?;
    ensure!(source == saved.source, "OBJECT_CANDIDATE_RECORD_CHANGED");
    let report = super::super::checked(saved)?;
    super::super::super::resume::require_idle(db, &request.project_id, &request.target.task_id)?;
    super::super::super::disposition::require_verifiable(
        db,
        &request.project_id,
        &request.target.task_id,
    )?;
    ensure!(
        !source.records.control.paused
            && !crate::object_task_coarse_dispatch::parent_paused(db, &source.records.medium)?,
        "OBJECT_TASK_PAUSED"
    );
    if let Some(verification) = super::super::super::storage::latest(
        db,
        &request.project_id,
        &request.target.task_id,
        &request.target.run_id,
    )? {
        ensure!(
            verification.operation.result.is_some(),
            "OBJECT_RECOVERY_PENDING"
        );
    }
    let mut feedback = super::super::super::resume::rework::feedback(
        db,
        &request.project_id,
        &request.target.task_id,
    )?;
    feedback.extend(super::deferred::feedback(
        db,
        &request.project_id,
        &request.target.task_id,
    )?);
    feedback.extend(super::feedback::inherited(db, request)?);
    feedback.sort_by(|a, b| a.request_id.cmp(&b.request_id));
    for item in &mut feedback {
        super::feedback::prepare(runtime, db, request, item).map_err(|error| {
            anyhow::anyhow!(
                "OBJECT_PUBLICATION_FEEDBACK_EVIDENCE: {}: {error:#}",
                item.request_id
            )
        })?;
    }
    let preview = build(&source, &report, feedback)?;
    Ok((source, report, preview))
}

pub(super) fn build(
    source: &super::Source,
    report: &super::Report,
    feedback: Vec<Feedback>,
) -> Result<Preview> {
    let records = &source.records;
    ensure!(
        feedback
            .iter()
            .map(|f| &f.request_id)
            .collect::<BTreeSet<_>>()
            .len()
            == feedback.len(),
        "OBJECT_PUBLICATION_FEEDBACK_ID_CONFLICT"
    );
    let object = &records.object;
    let baseline = records
        .preparation
        .baseline
        .as_ref()
        .context("OBJECT_BASELINE_REQUIRED")?;
    ensure!(
        baseline.object_revision_at_claim == object.revision
            && baseline.accepted_version_id_at_claim == records.accepted_version_id,
        "OBJECT_PUBLICATION_BASELINE_DRIFT"
    );
    ensure!(
        report.rules.iter().all(|rule| rule.passed),
        "OBJECT_PUBLICATION_CHECK_FAILED"
    );
    let attempt = records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    let output = attempt
        .output
        .as_ref()
        .context("OBJECT_CHECK_OUTPUT_REQUIRED")?;
    let references: Vec<_> = baseline
        .versions
        .iter()
        .filter(|v| v.object_id != object.id)
        .cloned()
        .collect();
    let reference_files = crate::object_run_baseline::snapshot(&references)?;
    for (path, hash) in &reference_files {
        ensure!(
            output.get(path) == Some(hash),
            "OBJECT_PUBLICATION_FIXED_REFERENCE_CHANGED: {path}"
        );
    }
    ensure!(
        object.references.iter().all(|reference| references
            .iter()
            .any(|v| v.object_id == reference.object_id
                && Some(&v.version_id) == reference.version_id.as_ref())),
        "OBJECT_PUBLICATION_REFERENCE_CLOSURE_CHANGED"
    );
    let mut after = Snapshot::new();
    let mut files = Vec::new();
    let mut folded = BTreeSet::new();
    for (path, hash) in output {
        ensure!(
            crate::object_version_manifest::valid_asset_path(path)
                && folded.insert(path.to_lowercase()),
            "OBJECT_PUBLICATION_INVALID_PATH: {path}"
        );
        if reference_files.contains_key(path) {
            continue;
        }
        ensure!(
            !source.objects.iter().any(|owner| owner.id != object.id
                && owner
                    .files
                    .iter()
                    .any(|f| f.path.to_lowercase() == path.to_lowercase())),
            "OBJECT_PUBLICATION_FOREIGN_FILE: {path}"
        );
        let role = object
            .files
            .iter()
            .find(|f| f.path.to_lowercase() == path.to_lowercase())
            .map(|f| f.role.clone())
            .or_else(|| {
                baseline
                    .versions
                    .iter()
                    .filter(|v| v.object_id == object.id)
                    .flat_map(|v| &v.files)
                    .find(|f| f.path == *path)
                    .map(|f| f.role.clone())
            })
            .unwrap_or_else(|| "resource".into());
        files.push(ObjectFile {
            path: path.clone(),
            role,
        });
        after.insert(path.clone(), hash.clone());
    }
    ensure!(files.len() <= 1024, "OBJECT_PUBLICATION_TOO_MANY_FILES");
    let before: Snapshot = match &records.accepted_version_id {
        Some(id) => {
            let version = object
                .versions
                .iter()
                .find(|v| &v.version_id == id)
                .context("OBJECT_VERSION_NOT_FOUND")?;
            crate::object_version_manifest::read(object, version)?
                .files
                .into_iter()
                .map(|f| (f.path, f.sha256))
                .collect()
        }
        None => Snapshot::new(),
    };
    let paths = before
        .keys()
        .chain(after.keys())
        .chain(object.files.iter().map(|f| &f.path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| Change {
            path: path.clone(),
            before: before.get(path).cloned(),
            after: after.get(path).cloned(),
        })
        .collect::<Vec<_>>();
    let mut path_owners = BTreeSet::new();
    for path in &paths {
        ensure!(
            path_owners.insert(path.path.to_lowercase()),
            "OBJECT_PUBLICATION_PATH_CASE_CONFLICT"
        );
        ensure!(
            !source.objects.iter().any(|owner| owner.id != object.id
                && owner
                    .files
                    .iter()
                    .any(|f| f.path.to_lowercase() == path.path.to_lowercase())),
            "OBJECT_PUBLICATION_FOREIGN_FILE: {}",
            path.path
        );
    }
    let mut preview = Preview {
        review: ReviewTarget {
            project_id: report.request.project_id.clone(),
            target: report.request.target.clone(),
            review_request_id: report.request.request_id.clone(),
        },
        digest: String::new(),
        output_digest: report.output_digest.clone(),
        baseline_version_id: baseline.resolved_version_id.clone(),
        accepted_version_id: records.accepted_version_id.clone(),
        object_revision: object.revision,
        replacement_required: baseline.resolved_version_id != records.accepted_version_id,
        files,
        paths,
        feedback,
    };
    preview.digest = digest(&preview, &report.source_digest)?;
    Ok(preview)
}

fn digest(preview: &Preview, source_digest: &str) -> Result<String> {
    let mut value = preview.clone();
    value.digest.clear();
    crate::framework_checks::digest(&(value, source_digest))
}

pub(super) fn approve(request: &Request, preview: &Preview) -> Result<()> {
    ensure!(
        valid_id(&request.request_id)
            && request.review() == preview.review
            && request.preview_digest == preview.digest,
        "OBJECT_PUBLICATION_PREVIEW_CHANGED"
    );
    ensure!(
        !request.acceptance_note.trim().is_empty() && request.acceptance_note.len() <= 4000,
        "OBJECT_PUBLICATION_OWNER_ACCEPTANCE_REQUIRED"
    );
    ensure!(
        request.confirm_files && (!preview.replacement_required || request.confirm_replacement),
        "OBJECT_PUBLICATION_CONFIRMATION_REQUIRED"
    );
    let ids: BTreeSet<_> = request.feedback.iter().map(|f| &f.request_id).collect();
    for decision in &request.feedback {
        let later = preview
            .feedback
            .iter()
            .find(|f| f.request_id == decision.request_id)
            .is_some_and(|f| f.later.is_some());
        ensure!(
            later == (decision.resolution == super::Resolution::Deferred),
            "OBJECT_PUBLICATION_FEEDBACK_ROUTE_MISMATCH"
        );
    }
    ensure!(
        ids.len() == request.feedback.len()
            && ids == preview.feedback.iter().map(|f| &f.request_id).collect()
            && request
                .feedback
                .iter()
                .all(|f| !f.note.trim().is_empty() && f.note.len() <= 4000),
        "OBJECT_PUBLICATION_FEEDBACK_UNRESOLVED"
    );
    super::relocation::approve(request, preview)?;
    Ok(())
}
