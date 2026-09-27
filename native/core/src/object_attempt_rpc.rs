//! One thread and one turn. No reconnect or replay can repeat side effects.
use crate::{
    object_attempt::{self, Lease},
    object_attempt_launch::{restrictions, INSTRUCTIONS},
    project_runtime::ProjectRuntime,
    rpc::{Event, Rpc},
};
use serde_json::{json, Value};
use std::{collections::VecDeque, future::Future, path::Path};
use tokio::sync::broadcast;

pub(crate) async fn conversation(
    runtime: &ProjectRuntime,
    lease: &mut Lease,
    rpc: &Rpc,
    events: &mut broadcast::Receiver<Event>,
    cwd: &Path,
    model: &str,
) -> Result<(), String> {
    let mut pending = VecDeque::new();
    handshake(rpc.initialize(), rpc, events, &mut pending).await?;
    object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
    let thread = handshake(rpc.request("thread/start", json!({
        "cwd":cwd,"model":model,"modelProvider":"beaver","approvalPolicy":"never",
        "sandbox":"workspace-write","config":restrictions(),"dynamicTools":[crate::object_attempt_callback::definition()],
        "developerInstructions":format!("{INSTRUCTIONS}\n{}", crate::code_structure::INSTRUCTIONS)
    })), rpc, events, &mut pending).await?;
    let thread = identifier(&thread, "thread")?;
    object_attempt::bind(runtime, lease, &thread, None).map_err(|error| error.to_string())?;
    let context = crate::object_attempt_callback::context(runtime, lease)
        .map_err(|error| error.to_string())?;
    let turn = handshake(rpc.request("turn/start", json!({
        "threadId":thread,"input":[{"type":"text","text":context.to_string(),"text_elements":[]}]
    })), rpc, events, &mut pending).await?;
    let turn = identifier(&turn, "turn")?;
    object_attempt::bind(runtime, lease, &thread, Some(&turn))
        .map_err(|error| error.to_string())?;
    loop {
        let event = match pending.pop_front() {
            Some(event) => event,
            None => events
                .recv()
                .await
                .map_err(|_| "OBJECT_ATTEMPT_EVENT_STREAM_LOST")?,
        };
        match event {
            Event::Notification { method, params } if params["threadId"] == thread => {
                if let Some(entry) =
                    crate::object_attempt_trace::notification(lease, &method, &params)
                {
                    object_attempt::record_event(runtime, lease, entry)
                        .map_err(|error| error.to_string())?;
                }
                if method == "turn/completed" && params["turn"]["id"] == turn {
                    object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
                    if params["turn"]["status"] == "completed" && params["turn"]["error"].is_null()
                    {
                        return Ok(());
                    }
                    return Err(params["turn"]["error"]["message"]
                        .as_str()
                        .unwrap_or("OBJECT_ATTEMPT_TURN_FAILED")
                        .into());
                }
                if method == "error" && params["turnId"] == turn && params["willRetry"] != true {
                    object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
                    return Err(params["error"]["message"]
                        .as_str()
                        .unwrap_or("OBJECT_ATTEMPT_PROVIDER_ERROR")
                        .into());
                }
            }
            Event::ServerRequest { id, method, params } => {
                if params["threadId"] != thread || params["turnId"] != turn {
                    rpc.reject(id, "OBJECT_ATTEMPT_RPC_IDENTITY_MISMATCH")
                        .await?;
                    continue;
                }
                object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
                if method == "item/tool/call"
                    && params["tool"] == crate::object_attempt_callback::TOOL
                {
                    let reply = match crate::object_attempt_callback::call(runtime, lease, &params)
                    {
                        Ok(value) => value,
                        Err(error) => crate::asset_tool::error_result(&error.to_string()),
                    };
                    object_attempt::validate_callback(runtime, lease)
                        .map_err(|error| error.to_string())?;
                    rpc.respond(id, reply).await?;
                    continue;
                }
                rpc.reject(id, "OBJECT_ATTEMPT_INTERACTION_UNAVAILABLE")
                    .await?;
                return Err("OBJECT_ATTEMPT_INTERACTION_UNAVAILABLE".into());
            }
            Event::Exit => return Err("OBJECT_ATTEMPT_PROCESS_EXITED".into()),
            _ => {}
        }
    }
}

async fn handshake(
    request: impl Future<Output = Result<Value, String>>,
    rpc: &Rpc,
    events: &mut broadcast::Receiver<Event>,
    pending: &mut VecDeque<Event>,
) -> Result<Value, String> {
    tokio::pin!(request);
    loop {
        tokio::select! {
            biased;
            result = &mut request => return result,
            event = events.recv() => match event.map_err(|_| "OBJECT_ATTEMPT_EVENT_STREAM_LOST")? {
                Event::ServerRequest { id, .. } => rpc.reject(id, "OBJECT_ATTEMPT_NOT_READY").await?,
                Event::Exit => return Err("OBJECT_ATTEMPT_PROCESS_EXITED".into()),
                event @ Event::Notification { .. } => {
                    if pending.len() >= 512 { return Err("OBJECT_ATTEMPT_EVENT_OVERFLOW".into()); }
                    pending.push_back(event);
                }
                Event::Log(_) => {},
            }
        }
    }
}

fn identifier(value: &Value, field: &str) -> Result<String, String> {
    value[field]["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("OBJECT_ATTEMPT_MISSING_{}", field.to_ascii_uppercase()))
}
