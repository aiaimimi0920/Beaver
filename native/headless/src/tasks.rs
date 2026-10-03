use crate::{resources, runtime, Host};
use anyhow::{Context, Result};
use beaver_core::{autonomy, clarifications, task_actions};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn create(host: &Host, input: Value) -> Result<Value> {
    let runtime = host
        .router
        .runtime_for_project(input["projectId"].as_str().unwrap())?;
    let handle = runtime.store();
    let task = {
        let mut store = handle
            .lock()
            .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
        beaver_core::task_create::create(
            &mut store,
            &runtime.files(),
            input,
            &resources::designs()?,
            &resources::blueprints()?,
        )?
    };
    // Never call the router while holding a Store lock.
    host.router.index_task(&task)?;
    Ok(task)
}

pub async fn control(host: Arc<Host>, method: &str, input: Value) -> Result<Value> {
    let id = input["id"].as_str().unwrap().to_owned();
    let runtime = runtime::task(&host.router, &id)?;
    let handle = runtime.store();
    if method == "task.answer" {
        let automatic: Vec<String> =
            serde_json::from_value(input.get("automatic").cloned().unwrap_or_else(|| json!([])))?;
        let question_id = input["questionId"].as_str().unwrap();
        let answers = input["answers"].clone();
        {
            let store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            let task: Value = store.get("task", &id)?.context("Task not found")?;
            anyhow::ensure!(
                task["status"] == "awaitingInput",
                "Task is not awaiting input"
            );
            let question = task["clarifications"]
                .as_array()
                .and_then(|items| items.iter().find(|question| question["id"] == question_id))
                .context("Question missing or expired")?;
            let checked = clarifications::validate_answers(question, answers.clone())?;
            clarifications::validate_automatic(
                question,
                &checked,
                &automatic,
                autonomy::effective(&store, &task)?,
            )?;
        }
        host.scheduler
            .synchronize(id.clone())
            .await
            .map_err(anyhow::Error::msg)?;
        {
            let mut store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            clarifications::answer_with_auto(&mut store, &id, question_id, answers, &automatic)?;
        }
        host.scheduler.wake().map_err(anyhow::Error::msg)?;
    } else if method == "task.continue" {
        let text = input["text"].as_str().unwrap();
        let steer = {
            let mut store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            task_actions::continue_task(
                &mut store,
                &id,
                text,
                input["freshContext"].as_bool().unwrap_or(false),
            )?
        };
        if steer {
            host.scheduler
                .steer(id, text.to_owned())
                .await
                .map_err(anyhow::Error::msg)?;
        } else {
            host.scheduler.wake().map_err(anyhow::Error::msg)?;
        }
    } else {
        let ids = {
            let store = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            let mut task: Value = store.get("task", &id)?.context("Task not found")?;
            if task["status"] == "waitingChildren" {
                task["planPaused"] = json!(true);
                store.put("task", &id, &task)?;
                serde_json::from_value::<Vec<String>>(task["subtaskIds"].clone())?
            } else {
                vec![id]
            }
        };
        for id in ids {
            host.scheduler
                .interrupt(id)
                .await
                .map_err(anyhow::Error::msg)?;
        }
    }
    Ok(Value::Null)
}
