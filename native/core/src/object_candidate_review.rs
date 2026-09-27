//! Durable whole-output review evidence. Never accepts stages or materializes files.
use super::records::{self, Records};
use crate::{
    object_attempt::State,
    object_attempt_checks::{self as checks, Rule},
    object_attempt_view::Target,
    object_catalog::ObjectRecord,
    object_task_storage as storage,
    object_task_types::{valid_id, Granularity, TaskRecord},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[path = "object_publication.rs"]
pub mod publication;
#[path = "object_candidate_summary.rs"]
mod summary;
pub use summary::{FileChange, Reference, Report, Stage};
const KIND: &str = "object_candidate_review";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
    pub check_request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    records: Records,
    fines: Vec<TaskRecord>,
    objects: Vec<ObjectRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    source: Source,
    report: Report,
}

fn read(connection: &Connection, request: &Request) -> Result<Source> {
    let records = records::read(connection, &request.project_id, &request.target.task_id)?
        .context("OBJECT_CANDIDATE_PREPARATION_REQUIRED")?;
    let attempt = records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    ensure!(
        Target::from_record(attempt) == request.target,
        "OBJECT_ATTEMPT_STALE_TARGET"
    );
    ensure!(
        attempt.state == State::AwaitingGate,
        "OBJECT_CANDIDATE_SUCCESS_REQUIRED"
    );
    checks::require_passed(connection, attempt, &request.check_request_id)?;
    let mut fines: Vec<_> = storage::all_tasks(connection)?
        .into_iter()
        .filter(|task| {
            task.parent_task_id.as_deref() == Some(&records.medium.id)
                && task.granularity == Granularity::Fine
                && task.status != "cancelled"
        })
        .collect();
    fines.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
    ensure!(
        fines.last().is_some_and(|fine| fine.id == attempt.fine.id),
        "OBJECT_CANDIDATE_FINAL_FINE_REQUIRED"
    );
    ensure!(
        fines[..fines.len() - 1]
            .iter()
            .all(|fine| fine.status == "accepted"),
        "OBJECT_STAGE_ORDER_MISMATCH"
    );
    let mut statement =
        connection.prepare("SELECT value FROM entities WHERE kind='object' ORDER BY id")?;
    let json = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let objects = json
        .iter()
        .map(|text| serde_json::from_str::<ObjectRecord>(text))
        .collect::<serde_json::Result<Vec<_>>>()?
        .into_iter()
        .filter(|object| object.project_id == request.project_id)
        .collect();
    Ok(Source {
        records,
        fines,
        objects,
    })
}

fn checked(saved: Stored) -> Result<Report> {
    let expected = summary::build(
        &saved.source,
        &saved.report.request,
        saved.report.rules.clone(),
        saved.report.time.clone(),
        saved.report.schema_version,
    )?;
    ensure!(expected == saved.report, "OBJECT_CANDIDATE_REPORT_MISMATCH");
    Ok(saved.report)
}

fn replay(connection: &Connection, request: &Request) -> Result<Option<Report>> {
    let Some(saved) = storage::read::<Stored>(connection, KIND, &request.request_id)? else {
        return Ok(None);
    };
    ensure!(
        saved.report.request == *request,
        "OBJECT_CANDIDATE_REQUEST_CONFLICT"
    );
    Ok(Some(checked(saved)?))
}

pub(super) fn require_rework_source(
    connection: &Connection,
    project: &str,
    review_id: &str,
    records: &Records,
    current: bool,
) -> Result<()> {
    let saved = storage::read::<Stored>(connection, KIND, review_id)?
        .context("OBJECT_CANDIDATE_REVIEW_REQUIRED")?;
    ensure!(
        saved.report.request.project_id == project
            && saved.report.request.request_id == review_id
            && saved.source.records == *records,
        "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH"
    );
    if current {
        ensure!(
            read(connection, &saved.report.request)? == saved.source,
            "OBJECT_CANDIDATE_RECORD_CHANGED"
        );
    }
    checked(saved)?;
    Ok(())
}

pub fn prepare(runtime: &ProjectRuntime, request: &Request) -> Result<Report> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.request_id)
            && valid_id(&request.target.attempt_id)
            && valid_id(&request.check_request_id),
        "INVALID_OBJECT_CANDIDATE_REQUEST"
    );
    let handle = runtime.store();
    let source = {
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("candidate store lock poisoned"))?;
        if let Some(report) = replay(&store.connection, request)? {
            return Ok(report);
        }
        read(&store.connection, request)?
    };
    let attempt = source
        .records
        .attempt
        .as_ref()
        .context("OBJECT_ATTEMPT_NOT_FOUND")?;
    let rules = checks::recheck(runtime, attempt);
    let report = summary::build(&source, request, rules, crate::asset_task::now(), 2)?;
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("candidate store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(report) = replay(connection, request)? {
            return Ok(report);
        }
        ensure!(
            read(connection, request)? == source,
            "OBJECT_CANDIDATE_RECORD_CHANGED"
        );
        storage::insert(
            connection,
            KIND,
            &request.request_id,
            &Stored {
                source,
                report: report.clone(),
            },
        )?;
        Ok(report)
    })
}

pub fn list(runtime: &ProjectRuntime, project: &str, attempt_id: &str) -> Result<Vec<Report>> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    ensure!(valid_id(attempt_id), "INVALID_OBJECT_ATTEMPT_ID");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("candidate store lock poisoned"))?;
    crate::object_attempt_view::read(&store.connection, project, attempt_id)?;
    let mut reports = Vec::new();
    for (id, saved) in store.list_with_ids::<Stored>(KIND)? {
        if saved.report.request.target.attempt_id == attempt_id {
            ensure!(
                id == saved.report.request.request_id && saved.report.request.project_id == project,
                "OBJECT_CANDIDATE_REPORT_MISMATCH"
            );
            reports.push(checked(saved)?);
        }
    }
    reports.sort_by(|a, b| {
        a.time
            .cmp(&b.time)
            .then_with(|| a.request.request_id.cmp(&b.request.request_id))
    });
    Ok(reports)
}
