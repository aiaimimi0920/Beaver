use crate::{asset_task, call_log, store::Store};
use anyhow::Result;
use serde_json::{json, Value};

pub fn context(store: &Store, task: &str) -> Result<Value> {
    let identity: Value = store.get("task", task)?.unwrap_or(Value::Null);
    let state: Option<asset_task::State> = store.get("asset-task", task)?;
    Ok(
        json!({"taskId":task,"threadId":identity["threadId"],"turnId":identity["turnId"],
        "assetRevision":state.as_ref().map(|s|s.revision),
        "attemptId":state.as_ref().and_then(|s|s.work.attempts.iter().find(|a|a.status=="running")).map(|a|&a.id),
        "sessionId":state.as_ref().and_then(|s|s.session_id.as_ref()),
        "checkpoint":state.as_ref().and_then(|s|s.checkpoint.as_ref())}),
    )
}

pub fn start(
    store: &Store,
    task: &str,
    id: &str,
    method: &str,
    item: &Value,
    identity: Option<(&str, &str)>,
    missing_start: bool,
) -> Result<()> {
    let mut context = if missing_start {
        json!({"taskId":task,"attemptId":null,"sessionId":null,"checkpoint":null})
    } else {
        context(store, task)?
    };
    if let Some((thread, turn)) = identity {
        context["threadId"] = json!(thread);
        context["turnId"] = json!(turn);
    }
    // Durable evidence is independent of bounded diagnostic call retention.
    store.put(&format!("framework-trace/{task}"),id,&json!({"id":id,"itemId":item["id"],"method":method,"context":context,
        "source":"codex-event","version":null,"skillEvidence":"not-observed","startedAt":if missing_start {Value::Null} else {json!(asset_task::now())},
        "gap":missing_start.then_some("completion-without-start; no attempt attribution"),"status":"running","input":if missing_start {Value::Null} else {call_log::summary(item)}}))
}

pub fn finish(store: &Store, task: &str, id: &str, status: &str, output: &Value) -> Result<()> {
    let kind = format!("framework-trace/{task}");
    if let Some(mut trace) = store.get::<Value>(&kind, id)? {
        trace["status"] = json!(status);
        trace["finishedAt"] = json!(asset_task::now());
        trace["output"] = call_log::summary(output);
        store.put(&kind, id, &trace)?;
    }
    Ok(())
}

pub fn observation(store: &Store, task: &str, reference: &asset_task::Reference) -> Result<()> {
    store.put(&format!("framework-observation/{task}"),&reference.id,&json!({"referenceId":reference.id,"frame":reference.frame,"sha256":reference.sha256,
        "context":context(store,task)?,"source":"live-frame","candidateId":null,"retention":"Image follows observer retention; this identity record is durable. Candidate images are separate frozen artifacts."}))
}

pub fn recovery(store: &Store, task: &str, reason: &str, operation: Option<&str>) -> Result<()> {
    let state: Option<asset_task::State> = store.get("asset-task", task)?;
    store.put(&format!("framework-recovery/{task}"),&uuid::Uuid::new_v4().to_string(),&json!({"time":asset_task::now(),"context":context(store,task)?,"reason":reason,"operationId":operation,
        "frameId":state.as_ref().and_then(|s|s.last_frame.as_ref()).map(|r|&r.frame.id),"source":"beaver-host","replayed":false}))
}

pub fn recover(store: &Store) -> Result<()> {
    store.connection.execute("UPDATE entities SET value=json_set(value,'$.status','interrupted','$.finishedAt',?) WHERE kind LIKE 'framework-trace/%' AND json_extract(value,'$.status')='running'",[asset_task::now()])?;
    Ok(())
}
