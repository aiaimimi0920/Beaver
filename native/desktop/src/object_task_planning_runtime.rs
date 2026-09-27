use crate::project_runtime_lifecycle;
use anyhow::{anyhow, Result};
use beaver_core::{
    execution_settings::ExecutionSettings,
    object_task_planning::{self as planning, AnswerRequest, SessionRequest, StartRequest},
    object_task_planning_launch, object_task_planning_round,
    object_task_planning_service::{Factory, Notify, Service},
    preferences,
    project_storage_router::ProjectStorageRouter,
    store::Store,
    tools,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

fn ask_tool() -> Value {
    json!({"name":"beaver_ask_user","description":"Ask focused planning questions.","inputSchema":{"type":"object","properties":{"questions":{"type":"array","minItems":1,"maxItems":3}},"required":["questions"],"additionalProperties":false}})
}

fn proposal_tool() -> Value {
    json!({"name":"beaver_propose_object_tasks","description":"Submit an additions-only object task plan.","inputSchema":{"type":"object","properties":{"objects":{"type":"array"},"tasks":{"type":"array"},"assumptions":{"type":"array"}},"required":["objects","tasks","assumptions"],"additionalProperties":false}})
}

pub fn start(host: Arc<Mutex<Store>>, notify: Notify) -> Service {
    let factory_host = host.clone();
    let factory: Factory = Arc::new(move |runtime, key| {
        let settings = {
            let store = factory_host.lock().map_err(|_| anyhow!("数据库锁不可用"))?;
            ExecutionSettings::read(
                &store,
                &preferences::SystemVault,
                serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
                Some("code"),
            )?
        };
        let codex = tools::find("codex", settings.tools["codex"].as_str().unwrap_or(""))?;
        object_task_planning_launch::prepare(
            &settings,
            &codex,
            object_task_planning_round::context(runtime, key)?,
            ask_tool(),
            proposal_tool(),
            std::env::vars_os().collect::<BTreeMap<_, _>>(),
        )
    });
    Service::new(factory, notify, 2)
}

pub fn call(
    service: &Service,
    router: &ProjectStorageRouter,
    method: &str,
    input: Value,
) -> Result<Value> {
    let project_id = input["projectId"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing projectId"))?;
    let runtime = project_runtime_lifecycle::open_registered(router, project_id)?;
    match method {
        "objectTaskPlanning.start" => {
            let request: StartRequest = serde_json::from_value(input)?;
            Ok(serde_json::to_value(service.start(runtime, &request)?)?)
        }
        "objectTaskPlanning.get" => {
            let draft_id = input["draftId"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing draftId"))?;
            Ok(serde_json::to_value(planning::get(
                &runtime, project_id, draft_id,
            )?)?)
        }
        "objectTaskPlanning.answer" => {
            let request: AnswerRequest = serde_json::from_value(input)?;
            Ok(serde_json::to_value(service.answer(runtime, &request)?)?)
        }
        "objectTaskPlanning.cancel" | "objectTaskPlanning.adopt" => {
            let request: SessionRequest = serde_json::from_value(input)?;
            let result = if method.ends_with("cancel") {
                service.cancel(&runtime, &request)?
            } else {
                service.adopt(&runtime, &request)?
            };
            Ok(serde_json::to_value(result)?)
        }
        _ => Err(anyhow!("Unknown planning method")),
    }
}
