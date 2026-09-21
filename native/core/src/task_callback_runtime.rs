use crate::{
    asset_task,
    files::Files,
    store::Store,
    task_callback,
    task_callback_contract::Request,
    task_callback_prepare::{self, Prepared},
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub async fn dynamic(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: &str,
    params: &Value,
) -> Result<Value> {
    call(
        store,
        files,
        id.into(),
        params["threadId"]
            .as_str()
            .context("Missing callback threadId")?
            .into(),
        params["turnId"]
            .as_str()
            .context("Missing callback turnId")?
            .into(),
        params
            .get("arguments")
            .context("Missing callback arguments")?
            .clone(),
    )
    .await
}

/// Hash/copy outside the database mutex; recheck identity and revisions on commit.
pub async fn call(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: String,
    thread: String,
    turn: String,
    input: Value,
) -> Result<Value> {
    tokio::task::spawn_blocking(move || blocking(store, files, id, thread, turn, input, None))
        .await?
}

pub(crate) fn operation(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    operation: &crate::framework_operations::Operation,
    input: Value,
) -> Result<()> {
    blocking(
        store,
        files,
        operation.task_id.clone(),
        operation
            .thread_id
            .clone()
            .context("Missing operation thread")?,
        operation
            .turn_id
            .clone()
            .context("Missing operation turn")?,
        input,
        Some(operation),
    )?;
    Ok(())
}

fn commit(
    db: &mut Store,
    id: &str,
    thread: &str,
    turn: &str,
    input: &Value,
    captured: Option<&Prepared>,
    operation: Option<&crate::framework_operations::Operation>,
) -> Result<Value> {
    if let Some(operation) = operation {
        crate::framework_operations::require_running(db, id, &operation.id)?;
    } else {
        crate::framework_operations::idle(db, id)?;
    }
    let result = task_callback::call_prepared(db, id, thread, turn, input, captured)?;
    if let Some(operation) = operation {
        let mut saved = operation.clone();
        saved.status = "succeeded".into();
        saved.ended_at = Some(asset_task::now());
        saved.result = Some(result.clone());
        db.put(crate::framework_operations::KIND, &operation.id, &saved)?;
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn blocking(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: String,
    thread: String,
    turn: String,
    input: Value,
    operation: Option<&crate::framework_operations::Operation>,
) -> Result<Value> {
    ensure!(
        serde_json::to_vec(&input)?.len() <= 64 * 1024,
        "Callback input exceeds 64 KiB"
    );
    let request: Request =
        serde_json::from_value(input.clone()).context("Invalid task callback request")?;
    if !task_callback_prepare::required(&request) {
        let mut db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        if matches!(request, Request::State | Request::Receipt { .. }) {
            return task_callback::call(&mut db, &id, &thread, &turn, &input);
        }
        return commit(&mut db, &id, &thread, &turn, &input, None, operation);
    }
    let (request_id, expected_revision) = match &request {
        Request::Work {
            request_id,
            expected_revision,
            ..
        }
        | Request::SubmitDelivery {
            request_id,
            expected_revision,
            ..
        } => (request_id, *expected_revision),
        _ => unreachable!(),
    };
    let (workspace, state) = {
        let mut db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        if let Some(operation) = operation {
            crate::framework_operations::require_running(&db, &id, &operation.id)?;
        }
        if task_callback::inspect(&db, &id, Some(request_id))?["receipt"].is_object() {
            return commit(&mut db, &id, &thread, &turn, &input, None, operation);
        }
        let task = task_callback::active(&db.connection, &id, &thread, &turn)?;
        ensure!(
            task_callback::inspect(&db, &id, None)?["revision"] == expected_revision,
            "Callback revision changed"
        );
        let state = asset_task::get(&db, &id)?;
        task_callback_prepare::preflight(&state, &request)?;
        (
            files.resolve_workspace(
                &id,
                Path::new(task["workspace"].as_str().context("Missing workspace")?),
            )?,
            state,
        )
    };
    let captured = Prepared::capture(&files, &workspace, &state, &request, &input)?;
    let mut db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    commit(
        &mut db,
        &id,
        &thread,
        &turn,
        &input,
        Some(&captured),
        operation,
    )
}
