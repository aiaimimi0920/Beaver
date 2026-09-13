use super::{repository, task_gate};
use crate::{
    executor::{Control, Outcome},
    files::Files,
    store::Store,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::mpsc;

/// Accept late instructions durably; never merge a candidate before applying them.
pub fn steer(store: &mut Store, id: &str, text: &str) -> Result<()> {
    let mut task: Value = repository::get(store, "task", id)?;
    anyhow::ensure!(task["status"] == "running", "Task is no longer running");
    anyhow::ensure!(
        !text.trim().is_empty() && text.encode_utf16().count() <= 32000,
        "Invalid instruction"
    );
    task["prompt"] = json!(format!(
        "{}\n\n补充要求：{text}",
        task["prompt"].as_str().unwrap_or("")
    ));
    let pending = task["validationSteering"].as_str().unwrap_or("");
    task["validationSteering"] = json!(format!("{pending}\n{text}").trim());
    task["updatedAt"] = json!(repository::now());
    store.transaction(|db| {
        db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), id],
        )?;
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,'user',?)",
            rusqlite::params![id, repository::now(), text],
        )?;
        Ok(())
    })
}

pub(crate) async fn run(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    id: String,
    engine: Option<PathBuf>,
    cancelled: Arc<AtomicBool>,
    controls: &mut mpsc::Receiver<Control>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> Outcome {
    let gate_cancel = Arc::new(AtomicBool::new(cancelled.load(Ordering::SeqCst)));
    let worker_store = store.clone();
    let worker_id = id.clone();
    let token = gate_cancel.clone();
    let notify = changed.clone();
    let mut worker = tokio::task::spawn_blocking(move || {
        task_gate::execute(
            worker_store,
            &files,
            &worker_id,
            engine.as_deref(),
            &token,
            &*notify,
        )
    });
    let mut steered = false;
    let mut accept = |control: Control| match control {
        Control::Interrupt => gate_cancel.store(true, Ordering::SeqCst),
        Control::Steer { text, reply } => {
            let result = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))
                .and_then(|mut db| steer(&mut db, &id, &text))
                .map_err(|e| e.to_string());
            if result.is_ok() {
                steered = true;
                gate_cancel.store(true, Ordering::SeqCst);
                changed();
            }
            let _ = reply.send(result);
        }
    };
    let outcome = loop {
        tokio::select! {
            result = &mut worker => break result.unwrap_or_else(|_| Outcome::Failed("Code validation worker failed".into())),
            control = controls.recv() => match control {
                Some(control) => accept(control),
                None => { gate_cancel.store(true, Ordering::SeqCst); break worker.await.unwrap_or(Outcome::Interrupted); }
            }
        }
    };
    controls.close();
    while let Ok(control) = controls.try_recv() {
        accept(control);
    }
    if cancelled.load(Ordering::SeqCst) {
        Outcome::Interrupted
    } else if steered {
        Outcome::Completed
    } else {
        outcome
    }
}

pub fn take(task: &mut Value) -> Result<Option<String>> {
    Ok(task
        .as_object_mut()
        .context("Invalid task")?
        .remove("validationSteering")
        .and_then(|value| value.as_str().map(str::to_owned)))
}
