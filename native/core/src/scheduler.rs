use crate::scheduler_runtime_ops::{close_sessions, interrupt_all, interrupt_task};
use crate::{
    executor::{Control, Outcome},
    files::Files,
    scheduler_runtime::{RuntimeSource, TaskRuntime},
    scheduler_work::{self, Finished},
    store::Store,
    task_finish,
};
use serde_json::Value;
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

#[path = "scheduler_object_control.rs"]
mod object_control;
#[path = "scheduler_object_resume.rs"]
mod object_resume;
pub use object_control::{Availability as ObjectAvailability, Execution as ObjectExecution};

pub struct Launch {
    pub external: Option<crate::external_execution::ExternalLaunch>,
    pub command: Option<Command>,
    pub model: String,
    pub prompt: String,
    pub ask_user_tool: Value,
    pub secrets: Vec<String>,
    pub max_minutes: u64,
    pub blender: Option<crate::blender_session::Request>,
    pub godot: Option<std::path::PathBuf>,
}
pub type Factory = Arc<dyn Fn(&Value) -> Result<Launch, String> + Send + Sync>;
pub type RuntimeFactory = Arc<dyn Fn(&Value, &TaskRuntime) -> Result<Launch, String> + Send + Sync>;
pub type ParallelLimit = Arc<dyn Fn() -> Result<usize, String> + Send + Sync>;
type Reply = oneshot::Sender<Result<(), String>>;
enum Message {
    Wake,
    Object(object_control::ObjectMessage),
    Resume(object_resume::Request),
    Interrupt(String, Reply),
    Synchronize(String, Reply),
    Steer(String, String, Reply),
    Shutdown(Reply),
}
struct Active {
    runtime: TaskRuntime,
    object: Option<crate::object_run_preparation::Preparation>,
    worker: Option<tokio::task::Id>,
    controls: mpsc::Sender<Control>,
    cancelled: Arc<AtomicBool>,
    waiters: Vec<Reply>,
}

#[derive(Clone)]
pub struct Scheduler {
    sender: mpsc::UnboundedSender<Message>,
    sessions: crate::asset_sessions::Sessions,
}

impl Scheduler {
    pub fn start(
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
        factory: Factory,
        changed: Arc<dyn Fn() + Send + Sync>,
        parallel_limit: ParallelLimit,
    ) -> Self {
        let runtime = TaskRuntime::host(store, files);
        let runtimes: RuntimeSource = Arc::new(move || Ok(vec![runtime.clone()]));
        let factory: RuntimeFactory = Arc::new(move |task, _| factory(task));
        Self::start_with_runtimes(runtimes, factory, changed, parallel_limit)
    }

    pub fn start_with_runtimes(
        runtimes: RuntimeSource,
        factory: RuntimeFactory,
        changed: Arc<dyn Fn() + Send + Sync>,
        parallel_limit: ParallelLimit,
    ) -> Self {
        Self::start_all(runtimes, factory, None, changed, parallel_limit)
    }

    pub fn start_with_objects(
        runtimes: RuntimeSource,
        factory: RuntimeFactory,
        objects: crate::object_attempt_launch::Factory,
        changed: Arc<dyn Fn() + Send + Sync>,
        parallel_limit: ParallelLimit,
    ) -> Self {
        Self::start_all(runtimes, factory, Some(objects), changed, parallel_limit)
    }

    fn start_all(
        runtimes: RuntimeSource,
        factory: RuntimeFactory,
        objects: Option<crate::object_attempt_launch::Factory>,
        changed: Arc<dyn Fn() + Send + Sync>,
        parallel_limit: ParallelLimit,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let sessions = crate::asset_sessions::Sessions::default();
        tokio::spawn(run(
            runtimes,
            factory,
            changed,
            receiver,
            sessions.clone(),
            parallel_limit,
            objects,
        ));
        Self { sender, sessions }
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
    pub async fn synchronize(&self, id: String) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .send(Message::Synchronize(id, reply))
            .map_err(|_| "任务调度器已关闭")?;
        receiver.await.map_err(|_| "任务调度器已关闭".to_string())?
    }
    pub fn asset_client(&self, id: &str) -> Result<crate::asset_preview::Client, String> {
        self.sessions.client(id)
    }
    pub async fn shutdown(&self) -> Result<(), String> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .send(Message::Shutdown(reply))
            .map_err(|_| "任务调度器已关闭")?;
        receiver.await.map_err(|_| "任务调度器已关闭".to_string())?
    }
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
#[path = "scheduler_tests.rs"]
mod tests;

async fn run(
    runtimes: RuntimeSource,
    factory: RuntimeFactory,
    changed: Arc<dyn Fn() + Send + Sync>,
    mut receiver: mpsc::UnboundedReceiver<Message>,
    sessions: crate::asset_sessions::Sessions,
    parallel_limit: ParallelLimit,
    objects: Option<crate::object_attempt_launch::Factory>,
) {
    let mut active: HashMap<String, Active> = HashMap::new();
    let mut jobs = JoinSet::new();
    let closing = Arc::new(AtomicBool::new(false));
    let mut shutdown: Vec<Reply> = Vec::new();
    let mut failure: Option<String> = None;
    let mut drained = false;
    let owner = uuid::Uuid::new_v4().to_string();
    loop {
        if receiver.is_closed() && receiver.is_empty() {
            closing.store(true, Ordering::SeqCst);
        }
        if !closing.load(Ordering::SeqCst) {
            let claimed = runtimes().and_then(|visible| {
                scheduler_work::claim(
                    &visible,
                    active.len(),
                    &parallel_limit,
                    objects.is_some(),
                    &owner,
                )
            });
            match claimed {
                Ok(batch) => {
                    for claimed in batch.claimed {
                        let runtime = claimed.runtime.clone();
                        let id = claimed.work.id();
                        let (controls, input) = mpsc::channel(16);
                        let cancelled = Arc::new(AtomicBool::new(false));
                        active.insert(
                            id.clone(),
                            Active {
                                runtime: runtime.clone(),
                                object: match &claimed.work {
                                    scheduler_work::Work::Object(claim) => {
                                        Some(claim.record().clone())
                                    }
                                    scheduler_work::Work::Legacy(_) => None,
                                },
                                worker: None,
                                controls,
                                cancelled: cancelled.clone(),
                                waiters: Vec::new(),
                            },
                        );
                        let factory = factory.clone();
                        let active_id = id.clone();
                        let worker = jobs.spawn(scheduler_work::execute(
                            claimed,
                            factory,
                            objects.clone(),
                            sessions.clone(),
                            cancelled,
                            input,
                            changed.clone(),
                        ));
                        active.get_mut(&active_id).unwrap().worker = Some(worker.id());
                        changed();
                    }
                    if let Some(error) = batch.failure {
                        failure = Some(error);
                        closing.store(true, Ordering::SeqCst);
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
                if let Err(error) = runtimes().and_then(|visible| interrupt_all(&visible)) {
                    failure = Some(error);
                }
                changed();
            }
            for entry in active.values() {
                cancel(entry);
            }
            if active.is_empty() {
                close_sessions(&sessions).await;
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
                            let finalized = match outcome {
                            Finished::Object(result) => result,
                            Finished::Legacy(outcome) => {
                            let outcome = if entry.cancelled.load(Ordering::SeqCst) { Outcome::Interrupted } else { outcome };
                            let db = entry.runtime.store(); let files = entry.runtime.files(); let finish_closing = closing.clone();
                            let finish_id = id.clone();
                            let finish_db = db.clone();
                            let finalized = tokio::task::spawn_blocking(move || {
                                let mut store = finish_db.lock().map_err(|_| "数据库锁不可用".to_string())?;
                                task_finish::finish(&mut store,&files,&finish_id,outcome,&finish_closing).map(|_| ()).map_err(|e| e.to_string())
                            }).await.unwrap_or_else(|_| Err("任务结果收尾异常".into()));
                            let retained = db.lock().ok().and_then(|db| db.get::<Value>("task", &id).ok().flatten())
                                .is_some_and(|task| matches!(task["status"].as_str(), Some("awaitingInput" | "queued")));
                            if !retained { sessions.close(&id).await; }
                            finalized
                            }
                            };
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
                    Some(Message::Object(message)) => object_control::handle(message, &mut active),
                    Some(Message::Resume(message)) => object_resume::handle(message, object_resume::Context {
                        active: &mut active, jobs: &mut jobs, runtimes: &runtimes, objects: &objects,
                        limit: &parallel_limit, closing: &closing, changed: &changed,
                    }),
                    Some(Message::Synchronize(id, reply)) => {
                        if let Some(entry) = active.get_mut(&id) { entry.waiters.push(reply); }
                        else { let _ = reply.send(Ok(())); }
                    },
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
                            let result = runtimes().and_then(|visible| interrupt_task(&visible, &id));
                            if result.as_ref().is_ok_and(Option::is_some) {
                                sessions.close(&id).await;
                            }
                            let _ = reply.send(result.map(|_| ())); changed();
                        }
                    },
                    Some(Message::Shutdown(reply)) => { closing.store(true,Ordering::SeqCst); shutdown.push(reply); },
                    None => { closing.store(true,Ordering::SeqCst); }
                }
                if closing.load(Ordering::SeqCst) {
                    if let Err(error) = runtimes().and_then(|visible| interrupt_all(&visible)) { failure = Some(error); }
                    changed();
                }
            }
        }
    }
}
