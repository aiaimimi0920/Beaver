use crate::Backend;
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};
use tauri::Emitter;

pub(crate) async fn call(
    app: tauri::AppHandle,
    state: Arc<Backend>,
    method: String,
    input: Option<Value>,
) -> Result<Value, String> {
    if method == "task.answer" {
        let value = input.as_ref().ok_or("缺少回答参数")?;
        let automatic: Vec<String> =
            serde_json::from_value(value.get("automatic").cloned().unwrap_or_else(|| json!([])))
                .map_err(|_| "自动回答标识无效")?;
        let id = value["id"].as_str().ok_or("缺少任务标识")?.to_owned();
        let question = value["questionId"]
            .as_str()
            .ok_or("缺少问题标识")?
            .to_owned();
        let answers = value["answers"].clone();
        let backend = state.clone();
        let task_id = id.clone();
        let question_id = question.clone();
        let check_answers = answers.clone();
        let check_automatic = automatic.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
            if backend.closing.load(Ordering::SeqCst) {
                return Err("应用正在退出".into());
            }
            let task: Value = store
                .get("task", &task_id)
                .map_err(|error| error.to_string())?
                .ok_or("任务不存在")?;
            if task["status"] != "awaitingInput" {
                return Err("任务当前没有等待回答".into());
            }
            let question = task["clarifications"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["id"] == question_id))
                .ok_or("问题不存在或已过期")?;
            let checked = beaver_core::clarifications::validate_answers(question, check_answers)
                .map_err(|error| error.to_string())?;
            beaver_core::clarifications::validate_automatic(
                question,
                &checked,
                &check_automatic,
                beaver_core::autonomy::effective(&store, &task).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())??;
        state.scheduler.interrupt(id.clone()).await?;
        let backend = state.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let mut store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
            if backend.closing.load(Ordering::SeqCst) {
                return Err("应用正在退出".into());
            }
            beaver_core::clarifications::answer_with_auto(
                &mut store, &id, &question, answers, &automatic,
            )
            .map_err(|error| error.to_string())?;
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())??;
        state.scheduler.wake()?;
        let _ = app.emit("beaver:changed", ());
        return Ok(Value::Null);
    }
    if method == "task.continue" {
        let input = input.as_ref().ok_or("缺少补充参数")?;
        let id = input["id"].as_str().ok_or("缺少任务标识")?.to_owned();
        let text = input["text"].as_str().ok_or("缺少补充要求")?.to_owned();
        let fresh_context = input["freshContext"].as_bool().unwrap_or(false);
        let backend = state.clone();
        let task_id = id.clone();
        let addition = text.clone();
        let steer = tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
            let mut store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
            if backend.closing.load(Ordering::SeqCst) {
                return Err("应用正在退出".into());
            }
            beaver_core::task_actions::continue_task(&mut store, &task_id, &addition, fresh_context)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        if steer {
            state.scheduler.steer(id, text).await?;
        } else {
            state.scheduler.wake()?;
        }
        let _ = app.emit("beaver:changed", ());
        return Ok(Value::Null);
    }
    if method == "task.interrupt" {
        if state.closing.load(Ordering::SeqCst) {
            return Err("应用正在退出".into());
        }
        let id = input
            .as_ref()
            .and_then(|v| v["id"].as_str())
            .ok_or("缺少任务标识")?;
        let ids = {
            let store = state.store.lock().map_err(|_| "数据库锁不可用")?;
            let mut task: Value = store
                .get("task", id)
                .map_err(|e| e.to_string())?
                .ok_or("任务不存在")?;
            if task["status"] == "waitingChildren" {
                task["planPaused"] = json!(true);
                store.put("task", id, &task).map_err(|e| e.to_string())?;
                serde_json::from_value::<Vec<String>>(task["subtaskIds"].clone())
                    .unwrap_or_default()
            } else {
                vec![id.to_owned()]
            }
        };
        for task_id in ids {
            state.scheduler.interrupt(task_id).await?;
        }
        let _ = app.emit("beaver:changed", ());
        return Ok(Value::Null);
    }
    Err(format!("Unknown task control operation: {method}"))
}
