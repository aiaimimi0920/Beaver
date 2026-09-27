//! Explicit cancellation journals optional workspace removal and retains HOME and frozen content.
use super::{files, storage as verification, transact, Report, Target, View};
use crate::object_recovery_resource;
use crate::{object_attempt::Attempt, object_attempt_view, project_runtime::ProjectRuntime};
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[path = "object_recovery_cancel.rs"]
mod cancel;
#[path = "object_recovery_disposition_store.rs"]
mod storage;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Choice {
    CancelAndKeep,
    CancelAndRemoveWorkspace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
    pub verification_request_id: String,
    pub choice: Choice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Retention {
    pub workspace: String,
    pub attempt_id: String,
    pub fine_task_id: String,
    pub output_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Outcome {
    CancelledAndRetained {
        report: Report,
        retained: Retention,
        task_revision: u64,
        run_revision: u64,
        plan_revision: u64,
    },
    CancelledAndWorkspaceRemoved {
        report: Report,
        removed: Retention,
        task_revision: u64,
        run_revision: u64,
        plan_revision: u64,
    },
    Blocked {
        report: Report,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub schema_version: u32,
    pub request: Request,
    pub result: Option<Outcome>,
}

impl Operation {
    fn retained(&self) -> bool {
        matches!(
            self.result,
            Some(
                Outcome::CancelledAndRetained { .. } | Outcome::CancelledAndWorkspaceRemoved { .. }
            )
        )
    }
}

pub(super) fn permits_new(previous: Option<&Operation>, generation: u64) -> bool {
    previous.is_none_or(|previous| {
        matches!(previous.result, Some(Outcome::Blocked { .. }))
            && previous.request.target.recovery_generation < generation
    })
}

pub(super) fn latest(
    connection: &Connection,
    project: &str,
    task: &str,
) -> Result<Option<Operation>> {
    Ok(storage::latest(connection, project, task)?.map(|saved| saved.operation))
}

pub(super) fn require_verifiable(connection: &Connection, project: &str, task: &str) -> Result<()> {
    if let Some(operation) = latest(connection, project, task)? {
        ensure!(
            operation.result.is_some(),
            "OBJECT_RECOVERY_DISPOSITION_PENDING"
        );
        ensure!(!operation.retained(), "OBJECT_RECOVERY_ALREADY_DISPOSED");
    }
    Ok(())
}

pub(crate) fn dispose(
    runtime: &ProjectRuntime,
    request: &Request,
    live: bool,
) -> Result<Operation> {
    dispose_with(runtime, request, live, || Ok(()), || Ok(()))
}

fn validate(request: &Request) -> Result<()> {
    verification::validate_target(&request.project_id, &request.request_id, &request.target)?;
    ensure!(
        crate::object_task_types::valid_id(&request.verification_request_id)
            && request.target.recovery_generation > 0,
        "INVALID_OBJECT_RECOVERY_REQUEST"
    );
    Ok(())
}

pub(crate) fn dispose_with(
    runtime: &ProjectRuntime,
    request: &Request,
    live: bool,
    before_files: impl FnOnce() -> Result<()>,
    after_files: impl FnOnce() -> Result<()>,
) -> Result<Operation> {
    ensure!(
        runtime.project_id() == request.project_id,
        "PROJECT_RUNTIME_MISMATCH"
    );
    validate(request)?;
    let _guard = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("OBJECT_RECOVERY_BUSY"))?;
    let saved = transact(runtime, |connection| storage::begin(connection, request))?;
    if saved.operation.result.is_some() {
        return Ok(saved.operation);
    }
    let mut saved = saved;
    before_files()?;
    let mut report = files::verify(runtime, &saved.verification.records, live);
    if let Err(error) = transact(runtime, |connection| {
        cancel::validate_current(connection, &saved)
    }) {
        report.records_current = false;
        report.issues.push(format!(
            "OBJECT_RECOVERY_DISPOSITION_RECORDS_CHANGED: {error:#}"
        ));
    }
    // Running is persisted only after checkpoint and ownership verification.
    if saved.resource.as_ref().is_some_and(|journal| {
        matches!(
            journal.state,
            object_recovery_resource::State::Running | object_recovery_resource::State::Completed
        )
    }) && report.workspace_status == super::WorkspaceStatus::Missing
    {
        report.workspace_status = super::WorkspaceStatus::MatchesCheckpoint;
    }
    if report.can_dispose() && matches!(request.choice, Choice::CancelAndRemoveWorkspace) {
        let previous = saved.clone();
        let journal = saved
            .resource
            .as_mut()
            .expect("resource journal initialized");
        if matches!(
            journal.state,
            object_recovery_resource::State::Pending | object_recovery_resource::State::Blocked
        ) {
            journal.start(&request.target.owner)?;
            let snapshot = saved.clone();
            transact(runtime, |connection| {
                storage::persist_pending(connection, &previous, &snapshot)
            })?;
        }
    }
    if report.can_dispose() && matches!(request.choice, Choice::CancelAndRemoveWorkspace) {
        let previous = saved.clone();
        let journal = saved
            .resource
            .as_ref()
            .expect("resource journal initialized");
        let removal = if journal.state == object_recovery_resource::State::Completed {
            ensure!(
                !journal.resolve_workspace(runtime.project_root())?.exists(),
                "OBJECT_RECOVERY_WORKSPACE_RECREATED"
            );
            Ok(())
        } else {
            remove_workspace(runtime, journal)
        };
        match removal {
            Ok(()) if journal.state == object_recovery_resource::State::Completed => {}
            Ok(()) => saved
                .resource
                .as_mut()
                .unwrap()
                .complete(&request.target.owner)?,
            Err(error) => {
                saved
                    .resource
                    .as_mut()
                    .unwrap()
                    .block(&request.target.owner, format!("{error:#}"))?;
                report.issues.push(format!(
                    "OBJECT_RECOVERY_WORKSPACE_REMOVE_FAILED: {error:#}"
                ));
            }
        }
        transact(runtime, |connection| {
            storage::persist_pending(connection, &previous, &saved)
        })?;
    }
    after_files()?;
    transact(runtime, |connection| {
        storage::finish(connection, saved, report)
    })
}

pub(super) fn retained_view(
    connection: &Connection,
    project: &str,
    task: &str,
) -> Result<Option<View>> {
    let Some(saved) =
        storage::latest(connection, project, task)?.filter(|saved| saved.operation.retained())
    else {
        return Ok(None);
    };
    cancel::validate_retained(connection, &saved)?;
    let records = &saved.verification.records;
    Ok(Some(View {
        target: records.target(saved.verification.operation.generation),
        preparation_state: records.preparation.state.clone(),
        paused: records.control.paused,
        operation: Some(saved.verification.operation),
        disposition: Some(saved.operation),
        report_matches_records: false,
        can_dispose: false,
        resume: super::resume::latest(connection, project, task)?,
        can_resume: false,
    }))
}

/// Only query paths use retained history. Writer validation still requires an owned claim.
pub(crate) fn retained_attempt(
    connection: &Connection,
    attempt: &Attempt,
) -> Result<Option<object_attempt_view::View>> {
    let prepared = &attempt.preparation;
    let Some(saved) = storage::latest(connection, &prepared.project_id, &prepared.medium.id)?
        .filter(|saved| saved.operation.retained())
    else {
        return Ok(None);
    };
    cancel::validate_retained(connection, &saved)?;
    ensure!(
        saved.verification.records.attempt.as_ref() == Some(attempt),
        "OBJECT_RECOVERY_RETAINED_HISTORY_MISMATCH"
    );
    Ok(Some(object_attempt_view::View {
        project_id: prepared.project_id.clone(),
        target: object_attempt_view::Target::from_record(attempt),
        task_revision: saved.verification.records.medium.revision + 1,
        state: attempt.state.clone(),
        output_captured: attempt.output.is_some(),
        error: attempt.error.clone(),
    }))
}

fn remove_workspace(
    runtime: &ProjectRuntime,
    journal: &object_recovery_resource::Journal,
) -> Result<()> {
    let path = journal.resolve_workspace(runtime.project_root())?;
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}
