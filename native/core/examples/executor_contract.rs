use anyhow::{ensure, Context, Result};
use beaver_core::{
    execution_health::IdleLimits,
    executor::{Control, Execution, Outcome},
    files::Files,
    store::Store,
    task_actions, task_finish,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
    time::timeout,
};

#[path = "support/executor_callbacks.rs"]
mod callbacks;

fn fixture() -> Result<()> {
    let owned = PathBuf::from(
        std::env::var_os("BEAVER_EXECUTOR_FIXTURE")
            .or_else(|| std::env::var_os("BEAVER_PROJECT_ROOT"))
            .context("fixture workspace required")?,
    );
    ensure!(
        std::fs::canonicalize(owned)? == std::fs::canonicalize(std::env::current_dir()?)?,
        "fixture workspace mismatch"
    );
    let mut output = std::io::stdout();
    let mut thread = format!("thread-{}", uuid::Uuid::new_v4());
    for line in std::io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let id = request["id"].clone();
        let method = request["method"].as_str().unwrap_or("");
        let mut notifications = Vec::new();
        let mut delayed = None;
        let result = match method {
            "initialize" => json!({}),
            "initialized" => continue,
            "thread/start" | "thread/resume" => {
                let instructions = request["params"]["developerInstructions"]
                    .as_str()
                    .unwrap_or("");
                ensure!(
                    instructions.contains(beaver_core::code_structure::INSTRUCTIONS),
                    "built-in code-structure instructions missing from thread start/resume"
                );
                ensure!(instructions.contains(beaver_core::task_callback_contract::INSTRUCTIONS));
                if method == "thread/start" {
                    callbacks::validate_tools(&request)?;
                }
                if let Some(old) = request["params"]["threadId"].as_str() {
                    thread = old.into();
                }
                json!({"thread":{"id":thread}})
            }
            "turn/start" => {
                let prompt = request["params"]["input"][0]["text"].as_str().unwrap_or("");
                let prompt = if let Some(value) = prompt.strip_prefix("native-desktop-fixture:") {
                    let home = PathBuf::from(
                        std::env::var_os("CODEX_HOME").context("task home required")?,
                    );
                    ensure!(
                        home.join("config.toml").is_file()
                            && home.join("skills/godot-production/SKILL.md").is_file(),
                        "task-scoped configuration and skills missing"
                    );
                    ensure!(
                        prompt.contains("docs/decisions") && prompt.contains("beaver_ask_user"),
                        "production prompt missing"
                    );
                    if value.starts_with("fresh") {
                        if prompt.contains("这是同一任务的新 AI 会话") {
                            "complete"
                        } else {
                            "silent"
                        }
                    } else if value.starts_with("ask") && prompt.contains("用户已确认的补充")
                    {
                        "complete"
                    } else {
                        value.lines().next().unwrap_or("")
                    }
                } else {
                    prompt
                };
                if prompt.starts_with("callback-") {
                    notifications.push(callbacks::report_request(prompt, &thread)?);
                } else if prompt == "ask" {
                    notifications.push(json!({"id":"question","method":"item/tool/call","params":{"threadId":thread,"turnId":"turn","tool":"beaver_ask_user","arguments":{"questions":[{"id":"tone","question":"氛围？"}]}}}));
                } else if prompt == "complete" {
                    std::fs::write("result.txt", "native executor output")?;
                    notifications.push(json!({"method":"turn/completed","params":{"threadId":thread,"turn":{"id":"old","status":"failed"}}}));
                    notifications.push(json!({"method":"item/completed","params":{"threadId":thread,"turnId":"turn","item":{"type":"agentMessage","text":"dummy-executor-secret Bearer fake-token 已完成"}}}));
                    notifications.push(json!({"method":"turn/completed","params":{"threadId":thread,"turn":{"id":"turn","status":"completed"}}}));
                } else if matches!(prompt, "silent" | "retrying") {
                    std::fs::write("partial.txt", "preserve unfinished work")?;
                    if prompt == "retrying" {
                        delayed = Some(prompt.to_owned());
                    }
                } else if matches!(prompt, "tool" | "reasoning" | "tool-interrupt") {
                    if prompt != "reasoning" {
                        notifications.push(json!({"method":"item/started","params":{"threadId":thread,"turnId":"turn","item":{"id":"tool-1","type":"commandExecution","command":"fixture-long-tool"}}}));
                    }
                    if prompt != "tool-interrupt" {
                        delayed = Some(prompt.to_owned());
                    }
                }
                json!({"turn":{"id":"turn"}})
            }
            "turn/steer" => {
                notifications.push(json!({"method":"turn/completed","params":{"threadId":thread,"turn":{"id":"turn","status":"completed"}}}));
                json!({})
            }
            "turn/interrupt" => {
                std::fs::write("interrupted.txt", "interrupt acknowledged")?;
                json!({})
            }
            "" if matches!(
                id.as_str(),
                Some("callback-report" | "callback-state" | "callback-work-begin")
            ) =>
            {
                writeln!(output, "{}", callbacks::after_reply(&request, &thread)?)?;
                output.flush()?;
                continue;
            }
            _ => continue,
        };
        writeln!(output, "{}", json!({"id":id,"result":result}))?;
        for event in notifications {
            writeln!(output, "{event}")?;
        }
        output.flush()?;
        if let Some(mode) = delayed {
            let thread = thread.clone();
            std::thread::spawn(move || {
                let send = |event: Value| {
                    let mut output = std::io::stdout().lock();
                    let _ = writeln!(output, "{event}");
                    let _ = output.flush();
                };
                for _ in 0..6 {
                    std::thread::sleep(Duration::from_millis(350));
                    if mode == "retrying" {
                        send(
                            json!({"method":"error","params":{"threadId":thread,"turnId":"turn","willRetry":true,"error":{"message":"busy dummy-executor-secret Bearer fake-token"}}}),
                        );
                        send(
                            json!({"method":"item/agentMessage/delta","params":{"threadId":thread,"turnId":"stale-turn","delta":"stale output"}}),
                        );
                        send(
                            json!({"method":"thread/tokenUsage/updated","params":{"threadId":thread,"tokenUsage":{"totalTokens":100}}}),
                        );
                    } else if mode == "reasoning" {
                        send(
                            json!({"method":"item/reasoning/textDelta","params":{"threadId":thread,"turnId":"turn","delta":"private-fixture-reasoning"}}),
                        );
                    }
                }
                if mode == "tool" {
                    send(
                        json!({"method":"item/completed","params":{"threadId":thread,"turnId":"turn","item":{"id":"tool-1","type":"commandExecution","exitCode":0}}}),
                    );
                }
                if mode != "retrying" {
                    send(
                        json!({"method":"turn/completed","params":{"threadId":thread,"turn":{"id":"turn","status":"completed"}}}),
                    );
                }
            });
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("codex-cli 0.1.0-fixture");
        return Ok(());
    }
    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("--fixture" | "app-server")
    ) {
        return fixture();
    }
    let output = PathBuf::from(std::env::args_os().nth(1).context("output root required")?);
    std::fs::create_dir_all(&output)?;
    let callbacks_only = std::env::args().nth(2).as_deref() == Some("--callbacks");
    for (index, mode) in [
        "complete",
        "ask",
        "steer",
        "interrupt",
        "silent",
        "retrying",
        "tool",
        "reasoning",
        "tool-interrupt",
        "callback-new",
        "callback-resume",
        "callback-upgrade",
    ]
    .into_iter()
    .filter(|mode| !callbacks_only || mode.starts_with("callback-"))
    .enumerate()
    {
        let root = output.join(format!("case-{index}-{}", uuid::Uuid::new_v4()));
        let store = Arc::new(Mutex::new(Store::open(&root.join("data"))?));
        let project = root.join("project");
        std::fs::create_dir(&project)?;
        std::fs::write(project.join("project.godot"), "[application]\n")?;
        let files = Arc::new(Files::new(root.join("data")));
        let baseline = files.capture(&project)?;
        let workspace = root.join("workspace");
        files.restore_copy(&baseline, &workspace)?;
        let workspace = std::fs::canonicalize(workspace)?;
        store
            .lock()
            .unwrap()
            .put("project", "p", &json!({"id":"p","path":project}))?;
        let mut initial = json!({"id":"t","projectId":"p","workspace":workspace,"baseline":baseline,"capability":"code","status":"running","prompt":mode,"direction":"story","threadId":"existing-thread","unknown":42,
            "callbackToolVersion":beaver_core::task_callback_contract::VERSION});
        if mode == "callback-new" {
            initial.as_object_mut().unwrap().remove("threadId");
        } else if mode == "callback-upgrade" {
            initial
                .as_object_mut()
                .unwrap()
                .remove("callbackToolVersion");
        }
        store.lock().unwrap().put("task", "t", &initial)?;
        if mode.starts_with("callback-") {
            callbacks::seed_unfinished_work(store.clone(), files.clone(), &initial).await?;
        }
        let execution = Execution {
            store: store.clone(),
            files: files.clone(),
            task_id: "t".into(),
            model: "fixture".into(),
            prompt: mode.into(),
            ask_user_tool: json!({"name":"beaver_ask_user"}),
            max_minutes: 0,
            secrets: vec!["dummy-executor-secret".into()],
        };
        let (sender, receiver) = mpsc::channel(8);
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg("--fixture")
            .current_dir(&workspace)
            .env("BEAVER_EXECUTOR_FIXTURE", &workspace);
        let started = std::time::Instant::now();
        let handle = tokio::spawn(execution.run_with_limits(
            command,
            receiver,
            IdleLimits {
                warning_after: Duration::from_millis(500),
                pause_after: Duration::from_millis(1200),
            },
        ));
        if matches!(mode, "steer" | "interrupt" | "tool-interrupt") {
            for _ in 0..100 {
                if store.lock().unwrap().get::<Value>("task", "t")?.unwrap()["turnId"] == "turn" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            if mode == "steer" {
                let (reply, received) = oneshot::channel();
                sender
                    .send(Control::Steer {
                        text: "补充世界观".into(),
                        reply,
                    })
                    .await?;
                timeout(Duration::from_secs(5), received)
                    .await??
                    .map_err(anyhow::Error::msg)?;
            } else {
                sender.send(Control::Interrupt).await?;
            }
        }
        let outcome = timeout(Duration::from_secs(15), handle).await??;
        let expected = match mode {
            "ask" => Outcome::AwaitingInput,
            "interrupt" | "tool-interrupt" => Outcome::Interrupted,
            "silent" | "retrying" => {
                ensure!(
                    matches!(&outcome, Outcome::Failed(error) if error.contains("AI 等待超时"))
                );
                ensure!(
                    started.elapsed() < Duration::from_secs(6),
                    "idle watchdog did not bound the wait"
                );
                ensure!(
                    workspace.join("interrupted.txt").is_file(),
                    "watchdog did not interrupt before closing"
                );
                match &outcome {
                    Outcome::Failed(error) => Outcome::Failed(error.clone()),
                    _ => unreachable!(),
                }
            }
            _ => Outcome::Completed,
        };
        ensure!(outcome == expected, "unexpected outcome: {outcome:?}");
        let events = serde_json::to_string(&store.lock().unwrap().events("t")?)?;
        ensure!(
            !events.contains("private-fixture-reasoning")
                && !events.contains("dummy-executor-secret")
                && !events.contains("fake-token")
                && !events.contains("stale output")
        );
        if mode == "retrying" {
            ensure!(events.contains("providerError") && events.contains("watchdog"));
        }
        let task: Value = store.lock().unwrap().get("task", "t")?.unwrap();
        if mode.starts_with("callback-") {
            callbacks::verify_receipt(&store.lock().unwrap(), &task, mode, &workspace)?;
            continue;
        }
        ensure!(
            task["threadId"] == "existing-thread"
                && task["unknown"] == 42
                && task["direction"] == "story"
        );
        if mode == "complete" {
            ensure!(
                task["status"] == "running",
                "runner claimed merge before task manager"
            );
            let report = task["report"].as_str().context("report missing")?;
            ensure!(
                report.contains("[REDACTED_SECRET]")
                    && !report.contains("dummy-executor-secret")
                    && !report.contains("fake-token")
            );
        }
        if mode == "ask" {
            ensure!(
                task["status"] == "awaitingInput"
                    && task["clarifications"][0]["questions"][0]["id"] == "tone"
            );
        }
        if mode == "steer" {
            ensure!(task["prompt"].as_str().unwrap().contains("补充世界观"));
        }
        ensure!(
            !project.join("result.txt").exists(),
            "project changed before finalization"
        );
        let finalized = task_finish::finish(
            &mut store.lock().unwrap(),
            &files,
            "t",
            outcome,
            &std::sync::atomic::AtomicBool::new(false),
        )?;
        if mode == "complete" {
            ensure!(finalized["status"] == "completed");
            ensure!(
                std::fs::read_to_string(project.join("result.txt"))? == "native executor output"
            );
            ensure!(store.lock().unwrap().list::<Value>("operation")?[0]["state"] == "complete");
        }
        if mode == "ask" {
            ensure!(finalized["status"] == "awaitingInput");
        }
        if matches!(mode, "interrupt" | "tool-interrupt") {
            ensure!(finalized["status"] == "interrupted");
        }
        if matches!(mode, "silent" | "retrying") {
            ensure!(
                finalized["status"] == "failed"
                    && finalized["baseline"] == serde_json::to_value(&baseline)?
            );
            ensure!(
                std::fs::read_to_string(workspace.join("partial.txt"))?
                    == "preserve unfinished work"
            );
            ensure!(
                !project.join("partial.txt").exists(),
                "stalled execution merged unfinished output"
            );
        }
        if mode == "silent" {
            {
                let mut store = store.lock().unwrap();
                ensure!(!task_actions::continue_task(
                    &mut store,
                    "t",
                    "finish existing work",
                    true
                )?);
                let mut task: Value = store.get("task", "t")?.unwrap();
                ensure!(
                    task["sessionHistory"][0]["threadId"] == "existing-thread"
                        && task.get("threadId").is_none()
                );
                let brief = beaver_core::task_brief::prompt(&task, &Value::Null);
                ensure!(brief.contains("partial.txt") && brief.contains("原基线"));
                task["status"] = json!("running");
                store.put("task", "t", &task)?;
            }
            let mut command = Command::new(std::env::current_exe()?);
            command
                .arg("--fixture")
                .current_dir(&workspace)
                .env("BEAVER_EXECUTOR_FIXTURE", &workspace);
            let (sender, receiver) = mpsc::channel(8);
            let outcome = Execution {
                store: store.clone(),
                files: files.clone(),
                task_id: "t".into(),
                model: "fixture".into(),
                prompt: "complete".into(),
                ask_user_tool: json!({"name":"beaver_ask_user"}),
                max_minutes: 0,
                secrets: vec!["dummy-executor-secret".into()],
            }
            .run(command, receiver)
            .await;
            ensure!(outcome == Outcome::Completed);
            let mut store = store.lock().unwrap();
            let task: Value = store.get("task", "t")?.unwrap();
            ensure!(
                task["threadId"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("thread-") && id != "existing-thread"),
                "fresh recovery resumed the old thread"
            );
            ensure!(
                task["baseline"] == serde_json::to_value(&baseline)?
                    && task["workspace"] == serde_json::to_value(&workspace)?
            );
            ensure!(!project.join("partial.txt").exists());
            let finalized = task_finish::finish(
                &mut store,
                &files,
                "t",
                outcome,
                &std::sync::atomic::AtomicBool::new(false),
            )?;
            ensure!(finalized["status"] == "completed");
            ensure!(
                std::fs::read_to_string(project.join("partial.txt"))? == "preserve unfinished work"
            );
            task_actions::rollback(&mut store, &files, "t", vec![])?;
            ensure!(
                !project.join("partial.txt").exists() && !project.join("result.txt").exists(),
                "fresh context lost the original rollback boundary"
            );
            drop(sender);
        }
        drop(sender);
    }
    let proof = if callbacks_only {
        json!({"passed":true,"checks":["new thread registers callback tool", "compatible thread resumes", "old thread upgrades and retains prior identity", "executor dispatch persists callback input and output", "result report does not create a file or complete task", "new turn retires previous unfinished attempt without Blender", "work retry callback binds a new attempt to the active thread and turn", "turn completion retires unfinished retry without replaying operations", "file-bearing work callbacks freeze and verify dependencies without a Blender session"],"realModelTaskVerified":false})
    } else {
        json!({"passed":true,"checks":["thread resume and turn completion", "stale completion ignored", "reports redact keys and bearer tokens", "question persisted before runner stops", "steer acknowledged and persisted", "interrupt stops runner", "unrelated task fields preserved", "runner does not claim project merge", "isolated subprocess output merged only after executor exit", "final merge has completed durable journal", "silent model is interrupted within idle limit", "retry and stale/usage events cannot extend idle deadline", "active tools and reasoning progress survive beyond idle limit", "active tool remains interruptible", "stalled workspace and baseline preserved without merge", "private reasoning is not persisted", "fresh context starts a new thread in the same workspace", "fresh recovery merges existing output and preserves original rollback boundary", "task callbacks survive new/resumed/upgraded threads"],"realModelTaskVerified":false})
    };
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!("{proof}");
    Ok(())
}
