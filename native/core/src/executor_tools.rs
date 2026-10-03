use crate::{asset_agent, clarifications, executor::Outcome, files::Files, rpc::Rpc, store::Store};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "executor_tools_tests.rs"]
mod tests;

#[allow(clippy::too_many_arguments)]
pub async fn dispatch(
    store: &Arc<Mutex<Store>>,
    files: &Arc<Files>,
    task_id: &str,
    asset: Option<&asset_agent::Context>,
    rpc: &Rpc,
    id: Value,
    method: &str,
    params: &Value,
) -> Result<Option<Outcome>, String> {
    if method == "item/tool/call"
        && (params["tool"] == "beaver_task" || params["tool"] == "beaver_workflow")
    {
        let result = if params["tool"] == "beaver_workflow" {
            crate::framework::dynamic(store.clone(), files.clone(), task_id, params).await
        } else {
            crate::task_callback_runtime::dynamic(store.clone(), files.clone(), task_id, params)
                .await
        };
        let paused = result.as_ref().is_ok_and(|value| value["paused"] == true);
        let reply = match result {
            Ok(value) => crate::asset_tool::text_result(value),
            Err(error) => crate::asset_tool::error_result(&error.to_string()),
        };
        rpc.respond(id, reply).await?;
        return Ok(paused.then_some(Outcome::AwaitingInput));
    }
    if method == "item/tool/call" && params["tool"] == "beaver_asset_task" {
        if let Some(asset) = asset {
            match asset.call(params).await {
                Ok(reply) => {
                    rpc.respond(id, reply.value).await?;
                    if let Some(id) = reply.delivered {
                        asset.delivered(&id).map_err(|e| e.to_string())?;
                    }
                }
                Err(error) => {
                    rpc.respond(id, crate::asset_tool::error_result(&error.to_string()))
                        .await?;
                }
            }
        } else {
            rpc.respond(
                id,
                crate::asset_tool::error_result("This task has no managed asset session"),
            )
            .await?;
        }
        return Ok(None);
    }
    if method == "item/tool/call" && params["tool"] == "beaver_submit_plan" {
        let result = {
            let store = store.lock().map_err(|_| "数据库锁不可用")?;
            crate::task_plan::submit(&store, files, task_id, params)
        };
        match result {
            Ok(()) => {
                rpc.respond(id, json!({"success":true,"contentItems":[{"type":"inputText","text":"计划已保存。Beaver 将停止规划会话并建立子任务。"}]})).await?;
                return Ok(Some(Outcome::Completed));
            }
            Err(error) => rpc.reject(id, &error.to_string()).await?,
        }
        return Ok(None);
    }
    let result = {
        let mut store = store.lock().map_err(|_| "数据库锁不可用")?;
        clarifications::park(&mut store, task_id, method, params)
    };
    match result {
        Ok(task) if task["status"] == "running" => {
            let answers = &task["clarifications"]
                .as_array()
                .ok_or("问题记录无效")?
                .last()
                .ok_or("问题记录为空")?["answers"];
            let result = if method == "item/tool/call" {
                json!({"success":true,"contentItems":[{"type":"inputText","text":json!({"answers":answers,"source":"automatic"}).to_string()}]})
            } else {
                let mapped: serde_json::Map<String, Value> = answers
                    .as_object()
                    .ok_or("回答记录无效")?
                    .iter()
                    .map(|(k, v)| (k.clone(), json!({"answers":[v]})))
                    .collect();
                json!({"answers":mapped})
            };
            rpc.respond(id, result).await?;
        }
        Ok(_) => return Ok(Some(Outcome::AwaitingInput)),
        Err(error) => rpc.reject(id, &error.to_string()).await?,
    }
    Ok(None)
}
