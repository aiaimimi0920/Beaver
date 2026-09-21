use crate::{
    files::Files,
    framework_contract::{Configuration, Request},
    framework_operations::{self as operations, Operation},
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub fn configuration(store: &Store, id: &str) -> Result<Configuration> {
    Ok(store
        .get("framework-configuration", id)?
        .unwrap_or_default())
}

pub fn inspect(store: &Store, id: &str) -> Result<Value> {
    let asset = store.get::<crate::asset_task::State>("asset-task", id)?;
    let operations = store
        .list::<Operation>(operations::KIND)?
        .into_iter()
        .filter(|o| o.task_id == id)
        .take(30)
        .map(|o| operations::view(&o))
        .collect::<Vec<_>>();
    Ok(
        json!({"configuration":configuration(store,id)?,"operations":operations,
        "traces":store.list::<Value>(&format!("framework-trace/{id}"))?.into_iter().take(100).collect::<Vec<_>>(),
        "checks":store.list::<Value>(&format!("framework-check/{id}"))?,
        "judgments":store.list::<Value>(&format!("framework-judgment/{id}"))?,
        "observations":store.list::<Value>(&format!("framework-observation/{id}"))?.into_iter().take(20).collect::<Vec<_>>(),
        "recovery":store.list::<Value>(&format!("framework-recovery/{id}"))?.into_iter().take(20).collect::<Vec<_>>(),
        "currentRecovery":asset.as_ref().and_then(|s|s.recovery.as_ref()),
        "coverage":{"tools":"observed Codex events; request/result shape and digest only","skills":"model-declared use; no inferred execution","plugins":"registered task-scoped adapters; readiness expires with execution identity"}}),
    )
}

/// Owner authority is supplied by the host route, never by request JSON.
pub async fn call(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: String,
    identity: Option<(String, String)>,
    input: Value,
) -> Result<Value> {
    tokio::task::spawn_blocking(move || dispatch(store, files, id, identity, input)).await?
}

pub async fn dynamic(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: &str,
    params: &Value,
) -> Result<Value> {
    let identity = (
        params["threadId"]
            .as_str()
            .context("Missing framework threadId")?
            .into(),
        params["turnId"]
            .as_str()
            .context("Missing framework turnId")?
            .into(),
    );
    call(
        store,
        files,
        id.into(),
        Some(identity),
        params
            .get("arguments")
            .context("Missing framework arguments")?
            .clone(),
    )
    .await
}

fn dispatch(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: String,
    identity: Option<(String, String)>,
    input: Value,
) -> Result<Value> {
    ensure!(
        serde_json::to_vec(&input)?.len() <= 128 * 1024,
        "Framework input exceeds 128 KiB"
    );
    crate::asset_task::validate_id(&id)?;
    let request: Request = serde_json::from_value(input)?;
    let mut db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    let task: Value = db.get("task", &id)?.context("Task does not exist")?;
    if !matches!(
        request,
        Request::State | Request::Inspect { .. } | Request::Cancel { .. }
    ) {
        crate::object_framework::require_legacy(&task)?;
    }
    // Queries remain possible for the submitting turn after it parks for review.
    if let Some((thread, turn)) = &identity {
        ensure!(
            task["threadId"] == *thread && task["turnId"] == *turn,
            "Framework execution identity changed"
        );
        if !matches!(
            request,
            Request::State
                | Request::Inspect { .. }
                | Request::Cancel { .. }
                | Request::Start { .. }
        ) {
            crate::task_callback::active(&db.connection, &id, thread, turn)?;
        }
    }
    match request {
        Request::State => inspect(&db, &id),
        Request::Inspect { operation_id } => {
            Ok(operations::view(&operations::get(&db, &id, &operation_id)?))
        }
        Request::Cancel { operation_id } => {
            let mut op = operations::get(&db, &id, &operation_id)?;
            if !operations::terminal(&op.status) {
                op.status = "cancelRequested".into();
                db.put(operations::KIND, &op.id, &op)?;
            }
            Ok(operations::view(&op))
        }
        Request::Configure { mut configuration } => {
            ensure!(
                identity.is_none(),
                "Only the owner can configure plugins and acceptance rules"
            );
            operations::idle(&db, &id)?;
            configuration.validate()?;
            let current = self::configuration(&db, &id)?;
            ensure!(
                current.revision == configuration.revision,
                "Framework configuration revision changed"
            );
            configuration.revision += 1;
            db.put("framework-configuration", &id, &configuration)?;
            Ok(json!(configuration))
        }
        Request::Judge {
            candidate_id,
            configuration_revision,
            verdict,
            note,
            paths,
        } => {
            drop(db);
            crate::framework_judgments::judge(
                store,
                &files,
                &id,
                &candidate_id,
                configuration_revision,
                &verdict,
                &note,
                &paths,
                identity.as_ref(),
            )
        }
        Request::Start { request_id, job } => {
            crate::asset_task::validate_id(&request_id)?;
            let source = if identity.is_some() { "model" } else { "owner" };
            let (thread_id, turn_id) = identity
                .clone()
                .map(|(a, b)| (Some(a), Some(b)))
                .unwrap_or_default();
            if let Some(op) = db
                .list::<Operation>(operations::KIND)?
                .into_iter()
                .find(|o| o.task_id == id && o.request_id == request_id)
            {
                ensure!(
                    json!(op.job) == json!(job)
                        && op.source == source
                        && op.thread_id == thread_id
                        && op.turn_id == turn_id,
                    "Framework request ID conflict"
                );
                return Ok(operations::view(&op));
            }
            if let Some((thread, turn)) = &identity {
                crate::task_callback::active(&db.connection, &id, thread, turn)?;
            }
            operations::idle(&db, &id)?;
            let prepared =
                crate::framework_jobs::prepare(&db, &files, &id, &job, identity.as_ref())?;
            let op = Operation {
                id: uuid::Uuid::new_v4().to_string(),
                task_id: id,
                request_id,
                job,
                source: source.into(),
                thread_id,
                turn_id,
                status: "running".into(),
                created_at: crate::asset_task::now(),
                ended_at: None,
                result: None,
                error: None,
            };
            db.transaction(|db| {
                if let crate::framework_contract::Job::Check { candidate_id } = &op.job {
                    crate::task_callback::put(
                        db,
                        &format!("framework-check/{}", op.task_id),
                        candidate_id,
                        &json!({"candidateId":candidate_id,"passed":false,"status":"running"}),
                    )?;
                }
                crate::task_callback::put(db, operations::KIND, &op.id, &op)
            })?;
            drop(db);
            launch(store, files, op.clone(), prepared);
            Ok(operations::view(&op))
        }
    }
}

fn launch(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    op: Operation,
    prepared: crate::framework_jobs::Prepared,
) {
    tokio::spawn(async move {
        let cancelled = Arc::new(AtomicBool::new(false));
        let db = store.clone();
        let job = op.clone();
        let flag = cancelled.clone();
        let mut worker = tokio::task::spawn_blocking(move || {
            crate::framework_jobs::execute(db, files, &job, prepared, &flag)
        });
        loop {
            tokio::select! {
                result = &mut worker => {
                    if let Ok(db) = store.lock() {
                        if let Ok(mut current) = operations::get(&db, &op.task_id, &op.id) {
                            if !operations::terminal(&current.status) {
                                current.ended_at = Some(crate::asset_task::now());
                                match result {
                                    Ok(Ok(_)) => { current.status = "failed".into(); current.error = Some("Worker returned without committing its result".into()); }
                                    error => { current.status = if current.status == "cancelRequested" {"cancelled"} else {"failed"}.into(); current.error = Some(format!("{}", match error {Ok(Err(e))=>e.to_string(), Err(e)=>e.to_string(), _=>unreachable!()})); }
                                }
                                let _ = db.put(operations::KIND, &current.id, &current);
                            }
                        }
                    }
                    break;
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    let stop = store.lock().ok().is_none_or(|db| {
                        if operations::get(&db,&op.task_id,&op.id).ok().is_none_or(|o|o.status != "running") { return true; }
                        if op.source == "model" {
                            let task = db.get::<Value>("task",&op.task_id).ok().flatten().unwrap_or(Value::Null);
                            if task["threadId"] != json!(op.thread_id) || task["turnId"] != json!(op.turn_id) || task["status"] != "running" {
                                let _ = operations::cancel_task(&db,&op.task_id,true);
                                return true;
                            }
                        }
                        false
                    });
                    if stop { cancelled.store(true, Ordering::SeqCst); }
                }
            }
        }
    });
}

pub async fn shutdown(store: &Arc<Mutex<Store>>) -> Result<()> {
    {
        let db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        for op in db.list::<Operation>(operations::KIND)? {
            operations::cancel_task(&db, &op.task_id, false)?;
        }
    }
    loop {
        let pending = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?
            .list::<Operation>(operations::KIND)?
            .iter()
            .any(|o| !operations::terminal(&o.status));
        if !pending {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
