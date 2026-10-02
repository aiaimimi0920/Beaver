//! Bounded operational metadata only: never persist provider text, commands or stderr.
use crate::{
    object_attempt::Lease, object_attempt_view, object_task_storage as storage,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const KIND: &str = "object_attempt_trace";
const LIMIT: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub run_id: String,
    pub attempt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub sequence: usize,
    pub operation: String,
    pub phase: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Trace {
    pub request: Request,
    pub entries: Vec<Entry>,
    pub truncated: bool,
}

fn identity(record: &crate::object_attempt::Attempt) -> Request {
    Request {
        project_id: record.preparation.project_id.clone(),
        run_id: record.preparation.run.id.clone(),
        attempt_id: record.id.clone(),
    }
}

pub fn read(runtime: &ProjectRuntime, request: &Request) -> Result<Trace> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("attempt trace store lock poisoned"))?;
    let record =
        object_attempt_view::read(&store.connection, &request.project_id, &request.attempt_id)?;
    ensure!(
        identity(&record) == *request,
        "OBJECT_ATTEMPT_TRACE_IDENTITY_MISMATCH"
    );
    load(&store.connection, request)
}

fn load(connection: &rusqlite::Connection, request: &Request) -> Result<Trace> {
    let trace = storage::read::<Trace>(connection, KIND, &request.attempt_id)?.unwrap_or(Trace {
        request: request.clone(),
        entries: Vec::new(),
        truncated: false,
    });
    validate_trace(&trace, request)?;
    Ok(trace)
}

pub(crate) fn validate_trace(trace: &Trace, request: &Request) -> Result<()> {
    ensure!(
        trace.request == *request
            && trace.entries.len() <= LIMIT
            && trace
                .entries
                .iter()
                .enumerate()
                .all(|(index, entry)| entry.sequence == index + 1),
        "OBJECT_ATTEMPT_TRACE_IDENTITY_MISMATCH"
    );
    Ok(())
}

pub(crate) fn append(
    connection: &rusqlite::Connection,
    lease: &Lease,
    mut entry: Entry,
) -> Result<()> {
    let request = identity(lease.record());
    let mut trace = load(connection, &request)?;
    if trace.truncated {
        return Ok(());
    }
    if trace.entries.len() == LIMIT {
        trace.truncated = true;
    } else {
        entry.sequence = trace.entries.len() + 1;
        trace.entries.push(entry);
    }
    storage::replace(connection, KIND, &request.attempt_id, &trace)
}

pub(crate) fn notification(lease: &Lease, method: &str, params: &Value) -> Option<Entry> {
    let record = lease.record();
    let thread = record.thread_id.as_deref()?;
    let turn = record.turn_id.as_deref()?;
    if params["threadId"] != thread {
        return None;
    }
    let (operation, phase, status) = match method {
        "item/started" | "item/completed" if params["turnId"] == turn => {
            let operation = match params["item"]["type"].as_str()? {
                "commandExecution" => "commandExecution",
                "fileChange" => "fileChange",
                "mcpToolCall" => "mcpToolCall",
                "dynamicToolCall" => "dynamicToolCall",
                "webSearch" => "webSearch",
                _ => return None,
            };
            let status = match params["item"]["status"].as_str() {
                Some("completed") => "completed",
                Some("failed") => "failed",
                Some("declined") => "declined",
                Some("inProgress") => "inProgress",
                _ => "unknown",
            };
            (
                operation,
                if method == "item/started" {
                    "started"
                } else {
                    "completed"
                },
                status,
            )
        }
        "turn/completed" if params["turn"]["id"] == turn => {
            let status = match params["turn"]["status"].as_str() {
                Some("completed") => "completed",
                Some("interrupted") => "interrupted",
                Some("failed") => "failed",
                _ => "unknown",
            };
            ("turn", "completed", status)
        }
        "error" if params["turnId"] == turn => (
            "provider",
            "error",
            if params["willRetry"] == true {
                "retrying"
            } else {
                "failed"
            },
        ),
        _ => return None,
    };
    Some(Entry {
        sequence: 0,
        operation: operation.into(),
        phase: phase.into(),
        status: status.into(),
    })
}
