//! Immutable, point-in-time evidence. These checks never grant execution or acceptance.
use crate::{
    object_attempt::{Attempt, State},
    object_attempt_view::{self as views, Target},
    object_task_storage as storage,
    object_task_types::valid_id,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[path = "object_attempt_check_engine.rs"]
mod engine;
pub(crate) use engine::verify as verify_snapshot;

const KIND: &str = "object_attempt_check_report";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub version: u32,
    pub passed: bool,
    pub issues: Vec<String>,
    pub files_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub request: Request,
    pub runner_version: u32,
    pub attempt_digest: String,
    pub input_digest: String,
    pub output_digest: String,
    pub time: String,
    pub passed: bool,
    pub rules: Vec<Rule>,
}

fn record(connection: &Connection, request: &Request) -> Result<Attempt> {
    let record = views::read(connection, &request.project_id, &request.target.attempt_id)?;
    ensure!(
        Target::from_record(&record) == request.target,
        "OBJECT_ATTEMPT_STALE_TARGET"
    );
    ensure!(
        record.state != State::Running && record.output.is_some(),
        "OBJECT_CHECK_OUTPUT_REQUIRED"
    );
    Ok(record)
}

fn validate(runtime: &ProjectRuntime, request: &Request) -> Result<()> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.request_id) && valid_id(&request.target.attempt_id),
        "INVALID_OBJECT_CHECK_REQUEST"
    );
    Ok(())
}

pub(crate) fn valid_rules(rules: &[Rule]) -> bool {
    rules.len() == 2
        && rules[0].id == "checkpoint-integrity"
        && rules[1].id == "code-structure"
        && rules
            .iter()
            .all(|rule| rule.version == 1 && (!rule.passed || rule.issues.is_empty()))
}

pub(crate) fn verify_report(report: &Report, record: &Attempt) -> Result<()> {
    ensure!(
        report.request.project_id == record.preparation.project_id
            && report.request.target == Target::from_record(record)
            && report.runner_version == 1
            && report.attempt_digest == crate::framework_checks::digest(record)?
            && report.input_digest == crate::framework_checks::digest(&record.input)?
            && report.output_digest == crate::framework_checks::digest(&record.output)?
            && valid_rules(&report.rules)
            && report.passed == report.rules.iter().all(|rule| rule.passed),
        "OBJECT_CHECK_REPORT_MISMATCH"
    );
    Ok(())
}

fn replay(connection: &Connection, request: &Request, attempt: &Attempt) -> Result<Option<Report>> {
    let report: Option<Report> = storage::read(connection, KIND, &request.request_id)?;
    if let Some(report) = &report {
        ensure!(report.request == *request, "OBJECT_CHECK_REQUEST_CONFLICT");
        verify_report(report, attempt)?;
    }
    Ok(report)
}

pub(crate) fn require_passed(connection: &Connection, attempt: &Attempt, id: &str) -> Result<()> {
    let request = Request {
        project_id: attempt.preparation.project_id.clone(),
        request_id: id.into(),
        target: Target::from_record(attempt),
    };
    ensure!(
        replay(connection, &request, attempt)?.is_some_and(|report| report.passed),
        "OBJECT_STAGE_PASSING_REPORT_REQUIRED"
    );
    Ok(())
}

pub(crate) fn recheck(runtime: &ProjectRuntime, attempt: &Attempt) -> Vec<Rule> {
    engine::run(runtime, attempt)
}

pub fn run(runtime: &ProjectRuntime, request: &Request) -> Result<Report> {
    validate(runtime, request)?;
    let handle = runtime.store();
    let attempt = {
        let store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("object check store lock poisoned"))?;
        let attempt = record(&store.connection, request)?;
        if let Some(report) = replay(&store.connection, request, &attempt)? {
            return Ok(report);
        }
        attempt
    };
    // File IO runs without the database lock. No workspace or external process is used.
    let rules = engine::run(runtime, &attempt);
    let report = Report {
        request: request.clone(),
        runner_version: 1,
        attempt_digest: crate::framework_checks::digest(&attempt)?,
        input_digest: crate::framework_checks::digest(&attempt.input)?,
        output_digest: crate::framework_checks::digest(&attempt.output)?,
        time: crate::asset_task::now(),
        passed: rules.iter().all(|rule| rule.passed),
        rules,
    };
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object check store lock poisoned"))?;
    store.transaction(|connection| {
        ensure!(
            record(connection, request)? == attempt,
            "OBJECT_CHECK_RECORD_CHANGED"
        );
        if let Some(previous) = replay(connection, request, &attempt)? {
            return Ok(previous);
        }
        storage::insert(connection, KIND, &request.request_id, &report)?;
        Ok(report)
    })
}

pub fn list(runtime: &ProjectRuntime, project: &str, attempt_id: &str) -> Result<Vec<Report>> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    ensure!(valid_id(attempt_id), "INVALID_OBJECT_ATTEMPT_ID");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object check store lock poisoned"))?;
    let attempt = views::read(&store.connection, project, attempt_id)?;
    let mut reports = Vec::new();
    for (id, report) in store.list_with_ids::<Report>(KIND)? {
        if report.request.target.attempt_id == attempt_id {
            ensure!(
                id == report.request.request_id,
                "OBJECT_CHECK_REPORT_MISMATCH"
            );
            verify_report(&report, &attempt)?;
            reports.push(report);
        }
    }
    reports.sort_by(|a, b| {
        a.time
            .cmp(&b.time)
            .then_with(|| a.request.request_id.cmp(&b.request.request_id))
    });
    Ok(reports)
}
