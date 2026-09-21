use crate::{asset_task, store::Store, task_callback_contract::Request};
use anyhow::{bail, ensure, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

pub(crate) fn get<T: DeserializeOwned>(db: &Connection, kind: &str, id: &str) -> Result<Option<T>> {
    let value: Option<String> = db
        .query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![kind, id],
            |r| r.get(0),
        )
        .optional()?;
    value.map(|v| Ok(serde_json::from_str(&v)?)).transpose()
}

pub(crate) fn put(db: &Connection, kind: &str, id: &str, value: &impl Serialize) -> Result<()> {
    db.execute("INSERT INTO entities(kind,id,value) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
        params![kind,id,serde_json::to_string(value)?])?;
    Ok(())
}

fn receipt_kind(task_id: &str) -> String {
    format!("task-callback/{task_id}")
}

fn revision(db: &Connection, id: &str) -> Result<u64> {
    Ok(get(db, "task-callback-revision", id)?.unwrap_or(0))
}

fn task(db: &Connection, id: &str) -> Result<Value> {
    get(db, "task", id)?.context("Task does not exist")
}

pub(crate) fn active(db: &Connection, id: &str, thread: &str, turn: &str) -> Result<Value> {
    let task = task(db, id)?;
    crate::object_framework::require_legacy(&task)?;
    ensure!(
        !thread.is_empty()
            && !turn.is_empty()
            && task["status"] == "running"
            && task["threadId"] == thread
            && task["turnId"] == turn,
        "Stale execution: callback must belong to this running task's current thread and turn"
    );
    Ok(task)
}

fn inspect_db(db: &Connection, id: &str, request_id: Option<&str>) -> Result<Value> {
    let task = task(db, id)?;
    let revision = revision(db, id)?;
    if let Some(request_id) = request_id {
        asset_task::validate_id(request_id)?;
        return Ok(
            json!({"revision":revision,"receipt":get::<Value>(db,&receipt_kind(id),request_id)?}),
        );
    }
    let asset: Option<asset_task::State> = get(db, "asset-task", id)?;
    let mut query =
        db.prepare("SELECT value FROM entities WHERE kind=? ORDER BY rowid DESC LIMIT 20")?;
    let receipts = query
        .query_map([receipt_kind(id)], |r| r.get::<_, String>(0))?
        .map(|r| {
            let receipt: Value = serde_json::from_str(&r?)?;
            let request = &receipt["request"];
            let summary: String = request["report"]["summary"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect();
            Ok(
                json!({"requestId":receipt["requestId"],"time":receipt["time"],
                "threadId":receipt["threadId"],"turnId":receipt["turnId"],
                "revision":receipt["response"]["revision"],"operation":request["operation"],
                "stageId":request["report"]["stageId"],"kind":request["report"]["kind"],
                "source":receipt["source"],"summary":summary}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"protocolVersion":crate::task_callback_contract::VERSION,"revision":revision,
        "task":{"id":id,"projectId":task["projectId"],"parentTaskId":task["parentTaskId"],
            "status":task["status"],"threadId":task["threadId"],"turnId":task["turnId"],
            "prompt":task["prompt"],"direction":task["direction"],"stopConditions":task["stopConditions"],
            "workspace":task["workspace"],"references":task["references"],"accepted":task["accepted"]},
        "asset":asset.as_ref().map(crate::asset_tool::projection),"receipts":receipts,
        "evidencePolicy":"Reports are model-reported. Recording a result does not validate files, accept a stage, or complete a task."}),
    )
}

/// Owner query: remains available after a turn ends or the host recovers tasks.
pub fn inspect(store: &Store, id: &str, request_id: Option<&str>) -> Result<Value> {
    inspect_db(&store.connection, id, request_id)
}

/// Host-bound task identity is never read from model-controlled arguments.
pub fn call(store: &mut Store, id: &str, thread: &str, turn: &str, input: &Value) -> Result<Value> {
    call_prepared(store, id, thread, turn, input, None)
}

pub(crate) fn call_prepared(
    store: &mut Store,
    id: &str,
    thread: &str,
    turn: &str,
    input: &Value,
    prepared: Option<&crate::task_callback_prepare::Prepared>,
) -> Result<Value> {
    ensure!(
        serde_json::to_vec(input)?.len() <= 64 * 1024,
        "Callback input exceeds 64 KiB; use file references"
    );
    let request: Request =
        serde_json::from_value(input.clone()).context("Invalid task callback request")?;
    store.transaction(|db| {
        // A submitted candidate parks this identity; only its exact receipt retry is allowed.
        let mut task = task(db, id)?;
        crate::object_framework::require_legacy(&task)?;
        if !(task["status"] == "awaitingInput" && task["waitingDelivery"].is_string()
            && task["threadId"] == thread && task["turnId"] == turn) {
            active(db, id, thread, turn)?;
        }
        let (request_id, expected) = match &request {
            Request::State => return inspect_db(db, id, None),
            Request::Receipt { request_id } => return inspect_db(db, id, Some(request_id)),
            Request::Report { request_id, expected_revision, .. }
            | Request::Stages { request_id, expected_revision, .. }
            | Request::DeliveryPlan { request_id, expected_revision, .. }
            | Request::Work { request_id, expected_revision, .. }
            | Request::SubmitDelivery { request_id, expected_revision, .. } => (request_id, *expected_revision),
        };
        asset_task::validate_id(request_id)?;
        let kind = receipt_kind(id);
        // Look up the receipt before the revision check: exact retries retain the original result.
        if let Some(receipt) = get::<Value>(db, &kind, request_id)? {
            ensure!(receipt["request"] == *input && receipt["threadId"] == thread && receipt["turnId"] == turn,
                "Request ID conflict: the saved request has different content or execution identity");
            return Ok(receipt["response"].clone());
        }
        active(db, id, thread, turn)?;
        let current = revision(db, id)?;
        ensure!(current == expected, "Callback revision changed; query state or receipt before retrying");
        if crate::task_callback_prepare::required(&request) {
            prepared.context("File capture or verification requires the asynchronous callback adapter")?.authorize(input)?;
        }
        let next = current.checked_add(1).context("Callback revision exhausted")?;
        let mut asset: Option<asset_task::State> = get(db, "asset-task", id)?;
        let mut candidate_id = None;
        let mut work_result = json!({});
        match &request {
            Request::Report { report, .. } => {
                report.validate()?;
                if let Some(stage_id) = &report.stage_id {
                    let state = asset.as_ref().context("Stage report requires an asset task")?;
                    ensure!(report.asset_revision == Some(state.revision), "Asset revision changed; inspect the current stage before reporting");
                    ensure!(state.stages.iter().any(|s| s.id == *stage_id), "Unknown asset stage");
                }
            }
            Request::Stages { asset_revision, stages, .. } => {
                let state = asset.as_mut().context("Stages require an asset task")?;
                crate::asset_stages::update(state, *asset_revision, stages.clone())?;
                put(db, "asset-task", id, state)?;
            }
            Request::DeliveryPlan { asset_revision, template, .. } => {
                ensure!(task["decompose"] != true, "Delivery requires a non-planning asset task");
                let state = asset.as_mut().context("Delivery requires an asset task")?;
                crate::asset_delivery::configure(state, *asset_revision, template)?;
                put(db, "asset-task", id, state)?;
            }
            Request::Work { asset_revision, stage_id, change, .. } => {
                let state = asset.as_mut().context("Work requires an asset task")?;
                work_result = crate::asset_work_actions::apply(state, *asset_revision, stage_id, thread, turn, change, prepared.and_then(|p| p.inputs()))?;
                put(db, "asset-task", id, state)?;
            }
            Request::SubmitDelivery { asset_revision, stage_id, input_candidates, summary, paths, .. } => {
                let state = asset.as_mut().context("Delivery requires an asset task")?;
                crate::asset_delivery::can_submit(state, *asset_revision, stage_id, input_candidates)?;
                ensure!(!summary.trim().is_empty() && summary.len() <= 12000, "Supply a candidate summary of at most 12000 bytes");
                let files = prepared.context("File submission requires the asynchronous callback adapter")?.delivery()?;
                ensure!(files.len() == paths.len() && paths.iter().all(|p| files.contains_key(p)), "Captured files do not match submission");
                let attempt_ids = crate::asset_work::completed_attempts(state, stage_id)?;
                let flow = state.delivery.as_mut().context("Missing delivery workflow")?;
                let candidate = crate::asset_delivery_files::Candidate {
                    id: uuid::Uuid::new_v4().to_string(), task_id: id.into(), stage_id: stage_id.clone(),
                    template_id: flow.template_id.clone(), template_version: flow.template_version,
                    input_candidates: input_candidates.clone(), attempt_ids, files: files.clone(), summary: summary.clone(),
                    thread_id: thread.into(), turn_id: turn.into(), session_id: state.session_id.clone(), created_at: asset_task::now(),
                };
                flow.pending = Some(candidate.id.clone());
                flow.history.push(candidate.id.clone());
                state.stages[flow.approved.len()].status = "checking".into();
                state.revision += 1;
                task["status"] = json!("awaitingInput");
                task["waitingDelivery"] = json!(candidate.id);
                put(db, "task", id, &task)?;
                put(db, "asset-task", id, state)?;
                put(db, &crate::asset_delivery_files::kind(id), &candidate.id, &candidate)?;
                candidate_id = Some(candidate.id);
            }
            _ => bail!("Unsupported mutation"),
        }
        let mut response = json!({"requestId":request_id,"revision":next,"recorded":true,
            "assetRevision":asset.as_ref().map(|s|s.revision),"acceptance":"notEvaluated"});
        response.as_object_mut().unwrap().extend(work_result.as_object().unwrap().clone());
        if let Some(id) = candidate_id {
            response["candidateId"] = json!(id);
            response["acceptance"] = json!("awaitingOwnerReview");
            response["paused"] = json!(true);
        }
        let receipt = json!({"id":uuid::Uuid::new_v4().to_string(),"requestId":request_id,
            "taskId":id,"projectId":task["projectId"],"threadId":thread,"turnId":turn,
            "sessionId":asset.as_ref().and_then(|s|s.session_id.as_ref()),
            "assetRevision":asset.as_ref().map(|s|s.revision),"time":chrono::Utc::now().to_rfc3339(),
            "source":"codexReported","request":input,"response":response});
        put(db, &kind, request_id, &receipt)?;
        put(db, "task-callback-revision", id, &next)?;
        // Event is a pointer, never the authoritative record (events are bounded).
        db.execute("INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)",params![id,
            receipt["time"].as_str().unwrap(),"taskCallback",
            json!({"requestId":request_id,"revision":next,"operation":input["operation"],"summary":input["report"]["summary"]}).to_string()])?;
        Ok(response)
    })
}

pub fn dynamic(store: &mut Store, id: &str, params: &Value) -> Result<Value> {
    call(
        store,
        id,
        params["threadId"]
            .as_str()
            .context("Missing callback threadId")?,
        params["turnId"]
            .as_str()
            .context("Missing callback turnId")?,
        params
            .get("arguments")
            .context("Missing callback arguments")?,
    )
}

pub fn business(store: &mut Store, method: &str, input: &Value) -> Result<Value> {
    let id = input["id"].as_str().context("Missing task ID")?;
    match method {
        "task.callback" => call(
            store,
            id,
            input["threadId"]
                .as_str()
                .context("Missing callback threadId")?,
            input["turnId"]
                .as_str()
                .context("Missing callback turnId")?,
            input.get("request").context("Missing callback request")?,
        ),
        "task.callbackState" => inspect(
            store,
            id,
            input
                .get("requestId")
                .map(|v| v.as_str().context("Invalid requestId"))
                .transpose()?,
        ),
        _ => bail!("Unknown task callback method"),
    }
}
