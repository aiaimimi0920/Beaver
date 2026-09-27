//! One read-only app-server round. Every exit awaits disposal of its owned process.
use crate::{
    asset_tool::{error_result, text_result},
    object_task_planning::RoundKey,
    object_task_planning_launch::{restrictions, Launch, INSTRUCTIONS},
    object_task_planning_round::{self as round, Questions},
    project_runtime::ProjectRuntime,
    rpc::{Event, Rpc},
};
use serde_json::{json, Value};
use std::{future::Future, sync::Arc};
use tokio::sync::{broadcast, watch};

pub type Notify = Arc<dyn Fn(&str) + Send + Sync>;

pub(crate) async fn run(
    runtime: &ProjectRuntime,
    key: &RoundKey,
    launch: Launch,
    mut stop: watch::Receiver<bool>,
    notify: &Notify,
) -> Result<(), String> {
    round::context(runtime, key).map_err(|error| error.to_string())?;
    if *stop.borrow() {
        return Err("OBJECT_PLANNING_INTERRUPTED".into());
    }
    let Launch {
        command,
        cwd,
        model,
        context,
        ask_tool,
        proposal_tool,
        secrets,
        timeout,
        scratch,
    } = launch;
    let _scratch = scratch;
    let (rpc, mut events) = Rpc::spawn(command).map_err(|error| redact(&error, &secrets))?;
    let params = json!({
        "cwd":cwd,"model":model,"modelProvider":"beaver","approvalPolicy":"never",
        "sandbox":"read-only","ephemeral":true,"developerInstructions":INSTRUCTIONS,
        "config":restrictions(),"dynamicTools":[ask_tool,proposal_tool]
    });
    let result = tokio::select! {
        biased;
        _ = cancelled(&mut stop) => Err("OBJECT_PLANNING_INTERRUPTED".into()),
        _ = tokio::time::sleep(timeout) => Err("OBJECT_PLANNING_TIMEOUT".into()),
        result = conversation(runtime, key, &rpc, &mut events, params, context, notify) => result,
    };
    // Keep the temporary HOME/cwd alive until the child and its descendants are gone.
    let closed = rpc.close().await;
    closed.and(result).map_err(|error| redact(&error, &secrets))
}

async fn cancelled(stop: &mut watch::Receiver<bool>) {
    while !*stop.borrow_and_update() {
        if stop.changed().await.is_err() {
            break;
        }
    }
}

async fn handshake(
    request: impl Future<Output = Result<Value, String>>,
    rpc: &Rpc,
    events: &mut broadcast::Receiver<Event>,
) -> Result<Value, String> {
    tokio::pin!(request);
    loop {
        tokio::select! {
            biased;
            result = &mut request => return result,
            event = events.recv() => match event.map_err(|_| "OBJECT_PLANNING_EVENT_STREAM_LOST")? {
                Event::ServerRequest { id, .. } => rpc.reject(id, "Planning round is not ready; no execution is authorized").await?,
                Event::Exit => return Err("OBJECT_PLANNING_PROCESS_EXITED".into()),
                _ => {},
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn conversation(
    runtime: &ProjectRuntime,
    key: &RoundKey,
    rpc: &Rpc,
    events: &mut broadcast::Receiver<Event>,
    params: Value,
    context: Value,
    notify: &Notify,
) -> Result<(), String> {
    handshake(rpc.initialize(), rpc, events).await?;
    let thread = handshake(rpc.request("thread/start", params), rpc, events).await?;
    let thread = identifier(&thread, "thread")?;
    round::bind(runtime, key, &thread, None).map_err(|error| error.to_string())?;
    notify(&key.project_id);
    let turn = handshake(rpc.request("turn/start", json!({
        "threadId":thread,"input":[{"type":"text","text":context.to_string(),"text_elements":[]}]
    })), rpc, events).await?;
    let turn = identifier(&turn, "turn")?;
    round::bind(runtime, key, &thread, Some(&turn)).map_err(|error| error.to_string())?;
    notify(&key.project_id);
    loop {
        match events
            .recv()
            .await
            .map_err(|_| "OBJECT_PLANNING_EVENT_STREAM_LOST")?
        {
            Event::ServerRequest { id, method, params } => {
                let result = dispatch(runtime, key, &thread, &turn, &method, &params);
                let (reply, settled) = match result {
                    Ok((reply, settled)) => {
                        notify(&key.project_id);
                        (text_result(reply), settled)
                    }
                    Err(error) => (error_result(&error.to_string()), false),
                };
                rpc.respond(id, reply).await?;
                if settled {
                    return Ok(());
                }
            }
            Event::Notification { method, params } if params["threadId"] == thread => {
                if method == "turn/completed" && params["turn"]["id"] == turn {
                    return Err(params["turn"]["error"]["message"]
                        .as_str()
                        .unwrap_or("OBJECT_PLANNING_NO_PROPOSAL: submit a proposal or a question")
                        .into());
                }
                if method == "error" && params["turnId"] == turn && params["willRetry"] != true {
                    return Err(params["error"]["message"]
                        .as_str()
                        .unwrap_or("OBJECT_PLANNING_PROVIDER_ERROR")
                        .into());
                }
            }
            Event::Exit => return Err("OBJECT_PLANNING_PROCESS_EXITED".into()),
            // Free-form text, plans, logs and unmatched notifications cannot change durable state.
            _ => {}
        }
    }
}

fn identifier(value: &Value, field: &str) -> Result<String, String> {
    value[field]["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("OBJECT_PLANNING_MISSING_{}", field.to_ascii_uppercase()))
}

fn dispatch(
    runtime: &ProjectRuntime,
    key: &RoundKey,
    thread: &str,
    turn: &str,
    method: &str,
    params: &Value,
) -> anyhow::Result<(Value, bool)> {
    anyhow::ensure!(
        method == "item/tool/call" && params["namespace"].is_null(),
        "OBJECT_PLANNING_TOOL_DENIED"
    );
    anyhow::ensure!(
        params["threadId"] == thread && params["turnId"] == turn,
        "OBJECT_PLANNING_RPC_IDENTITY_MISMATCH"
    );
    match params["tool"].as_str() {
        Some("beaver_ask_user") => match round::questions(runtime, key, &params["arguments"])? {
            Questions::Automatic(answers) => {
                Ok((json!({"answers":answers,"source":"automatic"}), false))
            }
            Questions::Waiting => Ok((
                json!({"status":"awaitingInput","message":"Questions saved. The host will stop this process and wait for the user."}),
                true,
            )),
        },
        Some("beaver_propose_object_tasks") => {
            round::propose(
                runtime,
                key,
                serde_json::from_value(params["arguments"].clone())?,
            )?;
            Ok((
                json!({"status":"proposed","message":"Proposal saved for explicit user adoption. No tasks were committed or executed."}),
                true,
            ))
        }
        _ => anyhow::bail!("OBJECT_PLANNING_TOOL_DENIED"),
    }
}

pub(crate) fn redact(message: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.into(), |text, secret| {
            text.replace(secret, "[REDACTED_SECRET]")
        })
}

#[cfg(test)]
#[path = "object_task_planning_executor_tests.rs"]
mod tests;
