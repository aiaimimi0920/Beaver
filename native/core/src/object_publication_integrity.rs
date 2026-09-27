//! Validate frozen journal data without re-entering the live writer query path.
use super::{commit, prepare, State, Stored};
use crate::{object_task_storage as store, object_version_manifest as manifest};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;

pub(super) fn validate(db: &Connection, saved: &Stored) -> Result<()> {
    let op = &saved.operation;
    let source = &saved.source;
    let object = &source.records.object;
    let review: super::super::Stored =
        store::read(db, super::super::KIND, &op.request.review_request_id)?
            .context("OBJECT_CANDIDATE_REVIEW_REQUIRED")?;
    ensure!(
        op.schema_version == 1
            && review.source == *source
            && review.report == saved.review
            && super::super::checked(review)? == saved.review
            && prepare::build(source, &saved.review, op.preview.feedback.clone())? == op.preview,
        "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
    );
    prepare::approve(&op.request, &op.preview)?;
    let version = crate::object_catalog::ObjectVersion {
        version_id: op.version_id.clone(),
        manifest: serde_json::to_value(&saved.manifest)?,
    };
    ensure!(
        manifest::read(object, &version)? == saved.manifest
            && saved.manifest.name == object.name
            && saved.manifest.components == object.components
            && saved.manifest.references == object.references
            && saved.manifest.files.len() == op.preview.files.len()
            && !object
                .versions
                .iter()
                .any(|v| v.version_id == op.version_id),
        "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
    );
    let mut total = 0_u64;
    for (file, own) in saved.manifest.files.iter().zip(&op.preview.files) {
        total = total
            .checked_add(file.bytes)
            .context("OBJECT_PUBLICATION_INVALID_BLOB")?;
        ensure!(
            file.path == own.path
                && file.role == own.role
                && op
                    .preview
                    .paths
                    .iter()
                    .any(|change| change.path == file.path
                        && change.after.as_ref() == Some(&file.sha256))
                && total <= 512 * 1024 * 1024,
            "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
        );
    }
    let changes: Vec<_> = op
        .preview
        .paths
        .iter()
        .filter(|c| c.before != c.after)
        .map(|c| c.path.clone())
        .collect();
    ensure!(
        changes.starts_with(&saved.writes),
        "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
    );
    match (&op.state, &op.result) {
        (State::Published, Some(result)) => {
            let object = commit::object(saved)?;
            let (_, medium, run, _) = commit::records(saved)?;
            ensure!(
                saved.writes == changes
                    && op.error.is_none()
                    && result.version_id == op.version_id
                    && result.object_revision == object.revision
                    && result.task_revision == medium.revision
                    && result.run_revision == run.revision
                    && result.plan_revision > 0
                    && result.plan_revision <= crate::object_command_receipt::MAX_REVISION,
                "OBJECT_PUBLICATION_RECEIPT_MISMATCH"
            );
            ensure!(
                store::read::<(String, crate::object_catalog::ObjectVersion)>(
                    db,
                    "object_version",
                    &op.version_id
                )? == Some((object.id.clone(), version)),
                "OBJECT_PUBLICATION_VERSION_MISMATCH"
            );
            crate::object_command_receipt::verify_acceptance(
                db,
                &commit::acceptance(saved),
                &object,
            )?;
        }
        (State::Applying | State::Aborting | State::Aborted, None) => {}
        _ => anyhow::bail!("OBJECT_PUBLICATION_RECEIPT_MISMATCH"),
    }
    Ok(())
}
