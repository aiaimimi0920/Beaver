//! Adjacent-anchor move with immutable receipt and transaction-local CAS.
use crate::{
    object_task_queue::{self as queue, QUEUE_KIND},
    object_task_queue_view::{self as view, View, ORDER_KIND},
    object_task_storage as storage,
    object_task_types::valid_id,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const RECEIPT_KIND: &str = "object_task_queue_reorder_receipt";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub expected_version: String,
    pub task_id: String,
    pub previous_task_id: Option<String>,
    pub next_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request: Request,
    pub result: View,
}

pub(crate) fn move_ids(view: &View, request: &Request) -> Result<Vec<String>> {
    let queued: Vec<_> = view
        .items
        .iter()
        .filter(|item| item.state == "queued")
        .collect();
    ensure!(
        queued.iter().any(|item| item.task_id == request.task_id),
        "OBJECT_TASK_QUEUE_NOT_REORDERABLE"
    );
    let mut ids: Vec<_> = queued
        .iter()
        .filter(|item| item.task_id != request.task_id)
        .map(|item| item.task_id.clone())
        .collect();
    let index = match &request.previous_task_id {
        None => 0,
        Some(id) => {
            ids.iter()
                .position(|current| current == id)
                .context("OBJECT_TASK_QUEUE_ANCHOR_CONFLICT")?
                + 1
        }
    };
    ensure!(
        ids.get(index) == request.next_task_id.as_ref(),
        "OBJECT_TASK_QUEUE_ANCHOR_CONFLICT"
    );
    ids.insert(index, request.task_id.clone());
    let mut objects = HashSet::new();
    for head in queued {
        if !objects.insert(&head.object_id) || head.blockers.is_empty() {
            continue;
        }
        let first = ids.iter().find(|id| {
            view.items
                .iter()
                .any(|item| item.task_id == **id && item.object_id == head.object_id)
        });
        ensure!(
            first == Some(&head.task_id),
            "OBJECT_TASK_QUEUE_BLOCKED_HEAD"
        );
    }
    Ok(ids)
}

pub fn reorder(runtime: &ProjectRuntime, request: &Request) -> Result<Receipt> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        [&request.project_id, &request.request_id, &request.task_id]
            .into_iter()
            .chain(request.previous_task_id.iter())
            .chain(request.next_task_id.iter())
            .all(|id| valid_id(id)),
        "INVALID_OBJECT_TASK_QUEUE_ID"
    );
    ensure!(
        request.expected_version.len() == 64
            && request
                .expected_version
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "INVALID_OBJECT_TASK_QUEUE_VERSION"
    );
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            &request.project_id,
            &request.request_id
        ))?)
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = storage::read::<Receipt>(connection, RECEIPT_KIND, &key)? {
            ensure!(
                receipt.request == *request,
                "OBJECT_TASK_QUEUE_REQUEST_CONFLICT"
            );
            let ids: Vec<_> = receipt
                .result
                .items
                .iter()
                .filter(|item| item.state == "queued")
                .map(|item| item.task_id.clone())
                .collect();
            ensure!(
                receipt.result.project_id == request.project_id
                    && receipt.result.version != request.expected_version
                    && move_ids(&receipt.result, request)? == ids,
                "OBJECT_TASK_QUEUE_RECEIPT_MISMATCH"
            );
            return Ok(receipt);
        }
        let current = view::read_in(connection, &request.project_id)?;
        ensure!(
            current.version == request.expected_version,
            "OBJECT_TASK_QUEUE_VERSION_CONFLICT"
        );
        let ids = move_ids(&current, request)?;
        for (position, task_id) in ids.iter().enumerate() {
            let mut entry = queue::read_entry(connection, &request.project_id, task_id)?
                .context("OBJECT_TASK_QUEUE_ENTRY_NOT_FOUND")?;
            entry.position = position as u64;
            storage::replace(connection, QUEUE_KIND, &entry.id, &entry)?;
        }
        let revision = storage::read::<u64>(connection, ORDER_KIND, &request.project_id)?
            .unwrap_or(0)
            .checked_add(1)
            .context("OBJECT_TASK_QUEUE_REVISION_EXHAUSTED")?;
        storage::replace(connection, ORDER_KIND, &request.project_id, &revision)?;
        let receipt = Receipt {
            request: request.clone(),
            result: view::read_in(connection, &request.project_id)?,
        };
        storage::insert(connection, RECEIPT_KIND, &key, &receipt)?;
        Ok(receipt)
    })
}
