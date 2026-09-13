use crate::{
    executor::{Control, Execution, Outcome},
    files::Files,
    store::Store,
    task_finish,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

pub struct Launch {
    pub command: Command,
    pub model: String,
    pub prompt: String,
    pub ask_user_tool: Value,
    pub secrets: Vec<String>,
    pub max_minutes: u64,
    pub blender: Option<crate::blender_session::Request>,
}
pub type Factory = Arc<dyn Fn(&Value) -> Result<Launch, String> + Send + Sync>;
type Reply = oneshot::Sender<Result<(), String>>;
enum Message {
    Wake,
    Interrupt(String, Reply),
    Steer(String, String, Reply),
    Shutdown(Reply),
}
struct Active {
    worker: Option<tokio::task::Id>,
    controls: mpsc::Sender<Control>,
    cancelled: Arc<AtomicBool>,
    waiters: Vec<Reply>,
}

#[derive(Clone)]
pub struct Scheduler {
    sender: mpsc::UnboundedSender<Message>,
}

impl Scheduler {
    pub fn start(
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        factory: Factory,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(run(store, files, factory, changed, receiver));
        Self { sender }
    }
    pub fn wake(&self) -> Result<(), String> {
        self.sender
            .send(Message::Wake)
            .map_err(|_| "任务调度器已关闭".into())
    }
    pub async fn interrupt(&self, id: String) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .send(Message::Interrupt(id, reply))
            .map_err(|_| "任务调度器已关闭")?;
        receiver.await.map_err(|_| "任务调度器已关闭".to_string())?
    }
    pub async fn steer(&self, id: String, text: String) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .send(Message::Steer(id, text, reply))
            .map_err(|_| "任务调度器已关闭")?;
        receiver.await.map_err(|_| "任务调度器已关闭".to_string())?
    }
    pub async fn shutdown(&self) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .send(Message::Shutdown(reply))
            .map_err(|_| "任务调度器已关闭")?;
        receiver.await.map_err(|_| "任务调度器已关闭".to_string())?
    }
}

fn claim(store: &mut Store, files: &Files, active: usize) -> anyhow::Result<Vec<Value>> {
    crate::task_plan::reconcile(store, files)?;
    let limit = store
        .get::<Value>("settings", "main")?
        .and_then(|s| s["maxParallel"].as_u64())
        .unwrap_or(2)
        .clamp(1, 6) as usize;
    let tasks: Vec<Value> = store.list("task")?;
    let count = tasks
        .iter()
        .filter(|t| t["status"] == "running")
        .count()
        .max(active);
    let selected: Vec<_> = tasks
        .iter()
        .rev()
        .filter(|t| t["status"] == "queued" && crate::task_plan::eligible(t, &tasks))
        .take(limit.saturating_sub(count))
        .cloned()
        .collect();
    let mut prepared = Vec::new();
    for mut task in selected {
        if let Err(error) = crate::task_plan::prepare(store, files, &mut task) {
            task["status"] = json!("failed");
            task["error"] = json!(error.to_string());
            store.put("task", task["id"].as_str().unwrap(), &task)?;
        } else {
            prepared.push(task);
        }
    }
    store.transaction(|db| {
        let mut result = Vec::new();
        for mut task in prepared {
            let id = task["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("任务标识无效"))?
                .to_string();
            task["status"] = json!("running");
            task.as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("任务格式无效"))?
                .remove("error");
            task["updatedAt"] =
                json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            db.execute(
                "UPDATE entities SET value=? WHERE kind='task' AND id=?",
                rusqlite::params![task.to_string(), id],
            )?;
            result.push(task);
        }
        Ok(result)
    })
}

fn interrupt_queued(store: &mut Store, id: Option<&str>) -> Result<(), String> {
    let tasks: Vec<Value> = store.list("task").map_err(|e| e.to_string())?;
    for mut task in tasks {
        let task_id = task["id"].as_str().ok_or("任务标识无效")?.to_string();
        if task["status"] == "queued" && id.is_none_or(|id| id == task_id) {
            task["status"] = json!("interrupted");
            store
                .put("task", &task_id, &task)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn cancel(entry: &Active) {
    if !entry.cancelled.swap(true, Ordering::SeqCst) {
        let sender = entry.controls.clone();
        tokio::spawn(async move {
            let _ = sender.send(Control::Interrupt).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn dropping_last_handle_interrupts_unstarted_queue() {
        let temp = tempfile::tempdir().unwrap();
        let store = Arc::new(Mutex::new(Store::open(temp.path()).unwrap()));
        store
            .lock()
            .unwrap()
            .put("task", "t", &json!({"id":"t","status":"queued"}))
            .unwrap();
        let scheduler = Scheduler::start(
            store.clone(),
            Arc::new(Files::new(temp.path().into())),
            Arc::new(|_| panic!("closed scheduler must not start tasks")),
            Arc::new(|| {}),
        );
        drop(scheduler);
        tokio::task::yield_now().await;
        assert_eq!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "t")
                .unwrap()
                .unwrap()["status"],
            "interrupted"
        );
    }
}

async fn run(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    factory: Factory,
    changed: Arc<dyn Fn() + Send + Sync>,
    mut receiver: mpsc::UnboundedReceiver<Message>,
) {
    let mut active: HashMap<String, Active> = HashMap::new();
    let mut jobs = JoinSet::new();
    let closing = Arc::new(AtomicBool::new(false));
    let mut shutdown: Vec<Reply> = Vec::new();
    let mut failure: Option<String> = None;
    let mut drained = false;
    loop {
        if receiver.is_closed() && receiver.is_empty() {
            closing.store(true, Ordering::SeqCst);
        }
        if !closing.load(Ordering::SeqCst) {
            let claimed = store
                .lock()
                .map_err(|_| "数据库锁不可用".to_string())
                .and_then(|mut store| {
                    claim(&mut store, &files, active.len()).map_err(|e| e.to_string())
                });
            match claimed {
                Ok(tasks) => {
                    for task in tasks {
                        let id = task["id"].as_str().unwrap().to_string();
                        let (controls, input) = mpsc::channel(16);
                        let cancelled = Arc::new(AtomicBool::new(false));
                        active.insert(
                            id.clone(),
                            Active {
                                worker: None,
                                controls,
                                cancelled: cancelled.clone(),
                                waiters: Vec::new(),
                            },
                        );
                        let factory = factory.clone();
                        let store = store.clone();
                        let active_id = id.clone();
                        let worker = jobs.spawn(async move {
                            if task["decompose"] == true && task["plan"].is_object() {
                                return (id, Outcome::Completed);
                            }
                            let factory_task = task.clone();
                            let prepared =
                                tokio::task::spawn_blocking(move || factory(&factory_task)).await;
                            let outcome = match prepared {
                                Ok(Ok(launch)) if !cancelled.load(Ordering::SeqCst) => {
                                    let session = if let Some(request) = launch.blender {
                                        let session_store = store.clone();
                                        let session_task = task.clone();
                                        let session_cancelled = cancelled.clone();
                                        match tokio::task::spawn_blocking(move || {
                                            request.start_logged(
                                                session_store,
                                                &session_task,
                                                &session_cancelled,
                                            )
                                        })
                                        .await
                                        {
                                            Ok(Ok(session)) => Some(session),
                                            Ok(Err(error)) => {
                                                return (
                                                    id,
                                                    if cancelled.load(Ordering::SeqCst) {
                                                        Outcome::Interrupted
                                                    } else {
                                                        Outcome::Failed(error.to_string())
                                                    },
                                                )
                                            }
                                            Err(_) => {
                                                return (
                                                    id,
                                                    Outcome::Failed(
                                                        "Blender preparation worker failed".into(),
                                                    ),
                                                )
                                            }
                                        }
                                    } else {
                                        None
                                    };
                                    let outcome = if cancelled.load(Ordering::SeqCst) {
                                        Outcome::Interrupted
                                    } else {
                                        Execution {
                                            store,
                                            task_id: id.clone(),
                                            model: launch.model,
                                            prompt: launch.prompt,
                                            ask_user_tool: launch.ask_user_tool,
                                            max_minutes: launch.max_minutes,
                                            secrets: launch.secrets,
                                        }
                                        .run(launch.command, input)
                                        .await
                                    };
                                    if let Some(session) = session {
                                        let _ = tokio::task::spawn_blocking(move || drop(session))
                                            .await;
                                    }
                                    outcome
                                }
                                Ok(Ok(_)) => Outcome::Interrupted,
                                Ok(Err(error)) => Outcome::Failed(error),
                                Err(_) => Outcome::Failed("任务环境准备异常".into()),
                            };
                            (id, outcome)
                        });
                        active.get_mut(&active_id).unwrap().worker = Some(worker.id());
                        changed();
                    }
                }
                Err(error) => {
                    failure = Some(error);
                    closing.store(true, Ordering::SeqCst);
                }
            }
        }
        if closing.load(Ordering::SeqCst) {
            if !drained {
                drained = true;
                if let Err(error) = store
                    .lock()
                    .map_err(|_| "数据库锁不可用".into())
                    .and_then(|mut store| interrupt_queued(&mut store, None))
                {
                    failure = Some(error);
                }
                changed();
            }
            for entry in active.values() {
                cancel(entry);
            }
            if active.is_empty() {
                receiver.close();
                for reply in shutdown {
                    let _ = reply.send(failure.clone().map_or(Ok(()), Err));
                }
                break;
            }
        }
        tokio::select! {
            result = jobs.join_next(), if !jobs.is_empty() => {
                match result {
                    Some(Ok((id, outcome))) => {
                        if let Some(entry) = active.remove(&id) {
                            let outcome = if entry.cancelled.load(Ordering::SeqCst) { Outcome::Interrupted } else { outcome };
                            let store = store.clone(); let files = files.clone(); let finish_closing = closing.clone();
                            let finalized = tokio::task::spawn_blocking(move || {
                                let mut store = store.lock().map_err(|_| "数据库锁不可用".to_string())?;
                                task_finish::finish(&mut store,&files,&id,outcome,&finish_closing).map(|_| ()).map_err(|e| e.to_string())
                            }).await.unwrap_or_else(|_| Err("任务结果收尾异常".into()));
                            for reply in entry.waiters { let _ = reply.send(finalized.clone()); }
                            if let Err(error) = finalized { failure = Some(error); closing.store(true,Ordering::SeqCst); }
                            changed();
                        }
                    },
                    Some(Err(error)) => {
                        if let Some(id) = active.iter().find(|(_, entry)| entry.worker == Some(error.id())).map(|(id,_)| id.clone()) {
                            if let Some(entry) = active.remove(&id) { for reply in entry.waiters { let _ = reply.send(Err("任务执行线程异常".into())); } }
                        }
                        failure = Some("任务执行线程异常，未合入的任务将在下次启动恢复".into()); closing.store(true,Ordering::SeqCst);
                    },
                    None => {}
                }
            }
            message = receiver.recv(), if !receiver.is_closed() || !receiver.is_empty() => {
                match message {
                    Some(Message::Wake) => {},
                    Some(Message::Steer(id,text,reply)) => {
                        if let Some(entry) = active.get(&id).filter(|entry| !closing.load(Ordering::SeqCst) && !entry.cancelled.load(Ordering::SeqCst)) {
                            if let Err(error) = entry.controls.try_send(Control::Steer { text,reply }) { if let Control::Steer { reply,.. } = error.into_inner() { let _ = reply.send(Err("任务暂时无法接收补充".into())); } }
                        } else { let _ = reply.send(Err("任务尚未就绪或已经结束".into())); }
                    },
                    Some(Message::Interrupt(id,reply)) => {
                        if let Some(entry) = active.get_mut(&id) {
                            cancel(entry);
                            entry.waiters.push(reply);
                        } else {
                            let result = store.lock().map_err(|_| "数据库锁不可用".into()).and_then(|mut store| interrupt_queued(&mut store,Some(&id)));
                            let _ = reply.send(result); changed();
                        }
                    },
                    Some(Message::Shutdown(reply)) => { closing.store(true,Ordering::SeqCst); shutdown.push(reply); },
                    None => { closing.store(true,Ordering::SeqCst); }
                }
                if closing.load(Ordering::SeqCst) {
                    if let Err(error) = store.lock().map_err(|_| "数据库锁不可用".into()).and_then(|mut store| interrupt_queued(&mut store,None)) { failure = Some(error); }
                    changed();
                }
            }
        }
    }
}
