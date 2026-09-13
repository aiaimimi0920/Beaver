use crate::{
    asset_agent,
    execution_health::{Health, IdleLimits},
    rpc::{Event, Rpc},
    store::Store,
};
use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
    time::{timeout, Instant},
};

pub enum Control {
    Interrupt,
    Steer {
        text: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Completed,
    Interrupted,
    AwaitingInput,
    Failed(String),
}

pub struct Execution {
    pub store: Arc<Mutex<Store>>,
    pub task_id: String,
    pub model: String,
    pub prompt: String,
    pub ask_user_tool: Value,
    pub max_minutes: u64,
    /// Plaintext values are used only in memory for log redaction.
    pub secrets: Vec<String>,
}

impl Execution {
    fn redact(&self, text: &str) -> String {
        let mut text = text.to_string();
        let mut keys: Vec<_> = self.secrets.iter().filter(|s| !s.is_empty()).collect();
        keys.sort_by_key(|s| std::cmp::Reverse(s.len()));
        for key in keys {
            text = text.replace(key, "[REDACTED_SECRET]");
        }
        static BEARER: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        BEARER
            .get_or_init(|| regex::Regex::new(r"(?i)Bearer\s+[A-Za-z0-9._~+/\-]+").unwrap())
            .replace_all(&text, "Bearer [REDACTED_SECRET]")
            .into_owned()
    }
    fn task(&self) -> Result<Value, String> {
        self.store
            .lock()
            .map_err(|_| "数据库锁不可用")?
            .get("task", &self.task_id)
            .map_err(|e| e.to_string())?
            .ok_or("任务不存在".into())
    }
    fn update(&self, fields: &[(&str, Value)]) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "数据库锁不可用")?;
        let mut task: Value = store
            .get("task", &self.task_id)
            .map_err(|e| e.to_string())?
            .ok_or("任务不存在")?;
        for (key, value) in fields {
            task[*key] = value.clone();
        }
        task["updatedAt"] =
            json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        store
            .put("task", &self.task_id, &task)
            .map_err(|e| e.to_string())
    }
    fn event(&self, kind: &str, text: &str) -> Result<(), String> {
        self.store
            .lock()
            .map_err(|_| "数据库锁不可用")?
            .event(
                &self.task_id,
                &chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                kind,
                &self.redact(text),
            )
            .map_err(|e| e.to_string())
    }
    /// Final project merge/status transition belongs to the task manager, after close.
    pub async fn run(self, command: Command, controls: mpsc::Receiver<Control>) -> Outcome {
        self.run_with_limits(command, controls, IdleLimits::default())
            .await
    }

    pub async fn run_with_limits(
        self,
        command: Command,
        mut controls: mpsc::Receiver<Control>,
        limits: IdleLimits,
    ) -> Outcome {
        self.run_controlled(command, &mut controls, limits, None)
            .await
    }

    pub(crate) async fn run_controlled(
        self,
        command: Command,
        controls: &mut mpsc::Receiver<Control>,
        limits: IdleLimits,
        asset: Option<asset_agent::Context>,
    ) -> Outcome {
        let started = std::time::Instant::now();
        let span = self.task().ok().and_then(|task| {
            self.store.lock().ok().and_then(|store| {
                crate::call_log::begin(
                    &store,
                    "codex",
                    "codex.turn",
                    Some(&self.task_id),
                    task["projectId"].as_str(),
                    &json!({"model":self.model,"prompt":self.prompt}),
                )
                .ok()
            })
        });
        let outcome = self
            .run_inner(command, controls, limits, asset.as_ref())
            .await;
        if matches!(outcome, Outcome::Failed(_) | Outcome::Interrupted) {
            if let Some(asset) = &asset {
                let _ = asset.uncertain("Execution stopped without a verified receipt. Inspect the retained scene before replaying edits.");
            }
        }
        if let Some(id) = span {
            if let Ok(store) = self.store.lock() {
                let status = match &outcome {
                    Outcome::Completed => "succeeded",
                    Outcome::Failed(_) => "failed",
                    Outcome::AwaitingInput => "awaitingInput",
                    Outcome::Interrupted => "interrupted",
                };
                let _ = crate::call_log::finish(
                    &store,
                    &id,
                    status,
                    started.elapsed().as_millis() as u64,
                    &json!({"status":status,"error":match &outcome {Outcome::Failed(error)=>Some(error),_=>None}}),
                );
            }
        }
        outcome
    }

    async fn run_inner(
        &self,
        command: Command,
        controls: &mut mpsc::Receiver<Control>,
        limits: IdleLimits,
        asset: Option<&asset_agent::Context>,
    ) -> Outcome {
        let (rpc, mut events) = match Rpc::spawn(command) {
            Ok(value) => value,
            Err(error) => return Outcome::Failed(self.redact(&error)),
        };
        let result = self
            .execute(&rpc, &mut events, controls, limits, asset)
            .await;
        let _ = self.event("execution", &json!({"phase":"finishing","lastProgressAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"activeTools":0,"retries":0,"pauseAfterSeconds":limits.pause_after.as_secs_f64()}).to_string());
        if let Err(error) = rpc.close().await {
            return Outcome::Failed(self.redact(&error));
        }
        match result {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Failed(self.redact(&error)),
        }
    }

    async fn execute(
        &self,
        rpc: &Rpc,
        events: &mut tokio::sync::broadcast::Receiver<Event>,
        controls: &mut mpsc::Receiver<Control>,
        limits: IdleLimits,
        asset: Option<&asset_agent::Context>,
    ) -> Result<Outcome, String> {
        let task = self.task()?;
        let mut activity = crate::call_log::Activity::new(self.store.clone(), &task);
        let workspace = task["workspace"].as_str().ok_or("任务工作副本无效")?;
        if stage(rpc.initialize(), controls).await?.is_none() {
            return Ok(Outcome::Interrupted);
        }
        let instructions = if asset.is_some() {
            format!(
                "{}\n{}",
                crate::asset_tool::INSTRUCTIONS,
                crate::code_structure::INSTRUCTIONS
            )
        } else {
            crate::code_structure::INSTRUCTIONS.into()
        };
        let fresh_asset_thread = asset.is_some() && task["assetToolVersion"] != 1;
        if fresh_asset_thread && task["threadId"].is_string() {
            self.update(&[("priorAssetThreadId", task["threadId"].clone())])?;
            self.event(
                "recovery",
                "资产工具已启用，建立新会话并保留旧会话标识；先核对现场、阶段和已有结果。",
            )?;
        }
        let mut params = json!({"cwd":workspace,"approvalPolicy":"never","sandbox":"danger-full-access","model":self.model,"developerInstructions":instructions});
        let method = if let Some(thread) = task["threadId"].as_str().filter(|_| !fresh_asset_thread)
        {
            params["threadId"] = json!(thread);
            "thread/resume"
        } else {
            let mut tools = vec![self.ask_user_tool.clone()];
            if task["decompose"] == true {
                tools.push(crate::task_plan::tool());
            }
            if asset.is_some() {
                tools.push(crate::asset_tool::definition());
            }
            params["dynamicTools"] = json!(tools);
            "thread/start"
        };
        let Some(thread) = stage(rpc.request(method, params), controls).await? else {
            return Ok(Outcome::Interrupted);
        };
        let thread_id = thread["thread"]["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Codex 未返回会话标识")?
            .to_string();
        self.update(&[("threadId", json!(thread_id)), ("turnId", Value::Null)])?;
        if asset.is_some() {
            self.update(&[("assetToolVersion", json!(1))])?;
        }
        let Some(turn) = stage(rpc.request("turn/start", json!({"threadId":thread_id,"cwd":workspace,"input":[{"type":"text","text":self.prompt,"text_elements":[]}]})), controls).await? else { return Ok(Outcome::Interrupted); };
        let mut turn_id = turn["turn"]["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Codex 未返回轮次标识")?
            .to_string();
        self.update(&[("turnId", json!(turn_id))])?;
        let mut health = Health::new(limits);
        self.event("execution", &health.snapshot().to_string())?;
        let tick = Duration::from_secs(1)
            .min(limits.pause_after / 4)
            .max(Duration::from_millis(10));
        let mut heartbeat = tokio::time::interval(tick);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_health = Instant::now();
        let mut recovery_attempts = 0;
        let mut notices = std::collections::HashSet::new();
        let deadline = if self.max_minutes > 0 {
            Instant::now().checked_add(Duration::from_secs(self.max_minutes.saturating_mul(60)))
        } else {
            None
        };
        loop {
            let limit = async {
                match deadline {
                    Some(time) => tokio::time::sleep_until(time).await,
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                _ = heartbeat.tick() => {
                    if let Some(asset) = asset {
                        if let Some(id) = asset.pending_notice().map_err(|e|e.to_string())? {
                            if notices.insert(id.clone()) {
                                let notice = format!("Asset feedback {id} is queued. At the next safe Blender boundary call beaver_asset_task poll, inspect its image, analyze impact, and process serially. Do not interrupt an unfinished scene mutation.");
                                match stage(rpc.request("turn/steer", json!({"threadId":thread_id,"expectedTurnId":turn_id,"input":[{"type":"text","text":notice,"text_elements":[]}]})), controls).await {
                                    Ok(Some(_)) => {},
                                    Ok(None) => { interrupt(rpc, &thread_id, &turn_id).await; return Ok(Outcome::Interrupted); },
                                    Err(error) => self.event("assetNotice", &error)?,
                                }
                            }
                        }
                    }
                    let warning = health.warning();
                    if warning {
                        self.event("watchdog", &format!("AI 已连续 {} 秒没有新输出，且没有运行中的工具；达到 {} 秒将暂停并保留工作副本。", limits.warning_after.as_secs(), limits.pause_after.as_secs()))?;
                    }
                    if warning || last_health.elapsed() >= Duration::from_secs(15) {
                        self.event("execution", &health.snapshot().to_string())?;
                        last_health = Instant::now();
                    }
                    if health.stalled() {
                        let message = format!("AI 等待超时：连续 {} 秒没有新输出，且没有运行中的工具。已暂停并保留工作副本；可继续任务，或用新会话从已有文件恢复。", limits.pause_after.as_secs());
                        self.event("watchdog", &message)?;
                        interrupt(rpc, &thread_id, &turn_id).await;
                        return Ok(Outcome::Failed(message));
                    }
                }
                _ = limit => {
                    self.event("system", "达到任务时间限制，正在中止。")?;
                    interrupt(rpc, &thread_id, &turn_id).await;
                    return Ok(Outcome::Interrupted);
                }
                control = controls.recv() => match control {
                    Some(Control::Steer { text, reply }) => {
                        let response = stage(rpc.request("turn/steer", json!({"threadId":thread_id,"expectedTurnId":turn_id,"input":[{"type":"text","text":text,"text_elements":[]}]})), controls).await;
                        let result = match response {
                            Ok(Some(_)) => {
                                let current = self.task()?;
                                self.update(&[("prompt", json!(format!("{}\n\n补充要求：{text}", current["prompt"].as_str().unwrap_or(""))))])?;
                                self.event("user", &text)?;
                                Ok(())
                            },
                            Ok(None) => {
                                let _ = reply.send(Err("任务已中止，补充要求未确认".into()));
                                interrupt(rpc, &thread_id, &turn_id).await;
                                return Ok(Outcome::Interrupted);
                            },
                            Err(error) => Err(self.redact(&error)),
                        };
                        let _ = reply.send(result);
                    }
                    _ => {
                        interrupt(rpc, &thread_id, &turn_id).await;
                        return Ok(Outcome::Interrupted);
                    }
                },
                event = events.recv() => {
                    match event.map_err(|_| "Codex 事件通道中断或消费落后；保留工作副本，不合入")? {
                        Event::Log(text) => self.event("system", &text)?,
                        Event::Exit => return Err("Codex 进程意外退出；工作副本已保留。".into()),
                        Event::ServerRequest { id, method, params } => {
                            if let Some(outcome) = crate::executor_tools::dispatch(&self.store, &self.task_id, asset, rpc, id, &method, &params).await? { return Ok(outcome); }
                        }
                        Event::Notification { method, params } => {
                            if params["threadId"].as_str().is_some_and(|id| id != thread_id) { continue; }
                            if params["turnId"].as_str().is_some_and(|id| id != turn_id) { continue; }
                            health.observe(&method, &params);
                            match method.as_str() {
                                "item/started" => activity.item(&params["item"],false).map_err(|e|e.to_string())?,
                                "item/agentMessage/delta" => self.event("assistant", params["delta"].as_str().unwrap_or(""))?,
                                "item/completed" => {
                                    let item = &params["item"];
                                    activity.item(item,true).map_err(|e|e.to_string())?;
                                    if item["type"] == "reasoning" { continue; }
                                    let text = item["text"].as_str().map(str::to_string).unwrap_or_else(|| item.to_string());
                                    self.event(item["type"].as_str().unwrap_or("item"), &text)?;
                                    if item["type"] == "agentMessage" { self.update(&[("report", json!(self.redact(&text)))])?; }
                                }
                                "turn/plan/updated" => self.event("plan", &params["plan"].to_string())?,
                                "thread/tokenUsage/updated" => self.event("usage", &params["tokenUsage"].to_string())?,
                                "error" => {
                                    let retrying = params["willRetry"] == true;
                                    if retrying { health.retry(); }
                                    let message = params["error"]["message"].as_str().unwrap_or("上游服务错误");
                                    self.event("providerError", &format!("{}：{message}", if retrying { "上游请求失败，Codex 正在有限重试" } else { "上游请求失败" }))?;
                                    self.event("execution", &health.snapshot().to_string())?;
                                    last_health = Instant::now();
                                }
                                "turn/completed" => {
                                    if params["turn"]["id"].as_str().is_some_and(|id| id != turn_id) { continue; }
                                    let error = params["turn"]["error"]["message"].as_str().unwrap_or("");
                                    if params["turn"]["status"] == "failed" && recovery_attempts == 0 && recoverable(error) {
                                        if let Some(asset) = asset { asset.uncertain("Connection failed after possible mutation; verify before replaying relative edits.").map_err(|e|e.to_string())?; }
                                        notices.clear();
                                        recovery_attempts += 1;
                                        health.retry();
                                        health.next_turn();
                                        self.event("recovery", "已确认本轮因临时连接故障结束；自动恢复 1/1。先检查已有结果，不重放成功操作。")?;
                                        let Some(next) = stage(rpc.request("turn/start", json!({"threadId":thread_id,"cwd":workspace,"input":[{"type":"text","text":"上轮连接故障已结束。先检查已有文件和操作结果，再从未完成处继续；不要重放已成功操作。若仍失败，请明确报告。","text_elements":[]}]})), controls).await? else { return Ok(Outcome::Interrupted); };
                                        turn_id = next["turn"]["id"].as_str().filter(|id|!id.is_empty()).ok_or("恢复未返回轮次标识")?.to_owned();
                                        self.update(&[("turnId",json!(turn_id)),("automaticRetries",json!(recovery_attempts))])?;
                                        continue;
                                    }
                                    return Ok(match params["turn"]["status"].as_str() {
                                        Some("completed") => Outcome::Completed,
                                        Some("interrupted") => Outcome::Interrupted,
                                        _ => Outcome::Failed(self.redact(params["turn"]["error"]["message"].as_str().unwrap_or("Codex 未完成本轮任务"))),
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn recoverable(message: &str) -> bool {
    let text = message.to_ascii_lowercase();
    ![
        "401",
        "403",
        "unauthorized",
        "quota",
        "context",
        "permission",
    ]
    .iter()
    .any(|v| text.contains(v))
        && [
            "connection reset",
            "connection closed",
            "stream disconnected",
            "temporarily unavailable",
            "502 bad gateway",
            "503 service unavailable",
        ]
        .iter()
        .any(|v| text.contains(v))
}

#[cfg(test)]
mod recovery_tests {
    #[test]
    fn only_confirmed_transient_failures_qualify() {
        assert!(super::recoverable("stream disconnected before completion"));
        assert!(!super::recoverable("401 unauthorized: connection closed"));
        assert!(!super::recoverable("apply_patch invalid patch"));
        assert!(!super::recoverable("request timed out"));
    }
}

async fn stage(
    future: impl std::future::Future<Output = Result<Value, String>>,
    controls: &mut mpsc::Receiver<Control>,
) -> Result<Option<Value>, String> {
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result.map(Some),
            control = controls.recv() => match control {
                Some(Control::Steer { reply, .. }) => { let _ = reply.send(Err("任务尚未就绪，请稍后补充".into())); },
                _ => return Ok(None),
            }
        }
    }
}

async fn interrupt(rpc: &Rpc, thread: &str, turn: &str) {
    let _ = timeout(
        Duration::from_millis(1500),
        rpc.request("turn/interrupt", json!({"threadId":thread,"turnId":turn})),
    )
    .await;
}
