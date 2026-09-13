use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub trait Operations {
    fn detect(&mut self, name: &str, cancelled: &AtomicBool) -> Result<Value>;
    fn install(&mut self, name: &str, cancelled: &AtomicBool) -> Result<()>;
    fn accept(&mut self, tool: &Value) -> Result<()>;
    fn save(&mut self, state: &Value) -> Result<()>;
    fn redact(&self, message: &str) -> String;
}

#[derive(Default)]
pub struct Setup {
    active: Mutex<Option<Arc<AtomicBool>>>,
}

fn stamp(state: &mut Value) {
    state["updatedAt"] =
        json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
}

pub fn idle() -> Value {
    json!({"status":"idle","updatedAt":"","steps":[]})
}

pub fn recover(mut state: Value) -> Value {
    if state["status"] == "running" {
        state["status"] = json!("cancelled");
        state["error"] = json!("上次环境准备已中断；重新准备会保留已安装工具。");
        if let Some(object) = state.as_object_mut() {
            object.remove("active");
        }
        if let Some(steps) = state["steps"].as_array_mut() {
            for step in steps {
                if step["status"] == "checking" || step["status"] == "installing" {
                    step["status"] = json!("cancelled");
                }
            }
        }
        stamp(&mut state);
    }
    state
}

struct Running<'a>(&'a Setup);
impl Drop for Running<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            *active = None;
        }
    }
}

impl Setup {
    pub fn cancel(&self) -> Result<()> {
        if let Some(cancelled) = self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("环境准备锁不可用"))?
            .as_ref()
        {
            cancelled.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn running(&self) -> Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("环境准备锁不可用"))?
            .is_some())
    }

    pub fn prepare(&self, names: &[&str], operations: &mut impl Operations) -> Result<Value> {
        let mut order = Vec::new();
        if names.contains(&"codex") {
            order.push("node");
        }
        for name in names {
            if !["node", "codex", "godot", "blender"].contains(name) {
                bail!("未知工具类型");
            }
            if !order.contains(name) {
                order.push(*name);
            }
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut active = self
                .active
                .lock()
                .map_err(|_| anyhow::anyhow!("环境准备锁不可用"))?;
            if active.is_some() {
                bail!("已有环境准备正在进行");
            }
            *active = Some(cancelled.clone());
        }
        let _running = Running(self);
        let mut state = json!({"status":"running","updatedAt":"","steps":order.iter().map(|name|json!({"name":name,"status":"waiting"})).collect::<Vec<_>>()});
        stamp(&mut state);
        operations.save(&state)?;
        let check = || -> Result<()> {
            if cancelled.load(Ordering::SeqCst) {
                bail!("环境准备已取消");
            }
            Ok(())
        };
        let mut active_index = None;
        let work = (|| -> Result<()> {
            for (index, name) in order.iter().enumerate() {
                check()?;
                active_index = Some(index);
                state["active"] = json!(name);
                state["steps"][index]["status"] = json!("checking");
                stamp(&mut state);
                operations.save(&state)?;
                let mut tool = operations.detect(name, &cancelled)?;
                check()?;
                if tool["available"] != true {
                    state["steps"][index]["status"] = json!("installing");
                    stamp(&mut state);
                    operations.save(&state)?;
                    operations.install(name, &cancelled)?;
                    check()?;
                    state["steps"][index]["status"] = json!("checking");
                    stamp(&mut state);
                    operations.save(&state)?;
                    tool = operations.detect(name, &cancelled)?;
                }
                check()?;
                if tool["available"] != true || tool["path"].as_str().is_none_or(str::is_empty) {
                    bail!(
                        "{name} 安装后仍未通过版本检测：{}",
                        tool["version"].as_str().unwrap_or("")
                    );
                }
                if tool["name"] != *name {
                    bail!("工具检测结果类型不一致");
                }
                operations.accept(&tool)?;
                state["steps"][index]["status"] = json!("ready");
                state["steps"][index]["result"] = tool;
                stamp(&mut state);
                operations.save(&state)?;
                active_index = None;
            }
            check()?;
            Ok(())
        })();
        if let Err(error) = work {
            let stopped = cancelled.load(Ordering::SeqCst);
            let status = if stopped { "cancelled" } else { "failed" };
            state["status"] = json!(status);
            if let Some(index) = active_index {
                state["steps"][index]["status"] = json!(status);
            }
            let message = if stopped {
                "已停止准备；已安装的工具会保留。".into()
            } else {
                operations.redact(&error.to_string())
            };
            let mut tail: Vec<char> = message.chars().rev().take(4000).collect();
            tail.reverse();
            state["error"] = json!(tail.into_iter().collect::<String>());
        } else {
            state["status"] = json!("completed");
        }
        state
            .as_object_mut()
            .context("环境准备状态无效")?
            .remove("active");
        stamp(&mut state);
        operations.save(&state)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fixture {
        calls: Vec<String>,
        states: Vec<Value>,
        installed: Vec<String>,
        fail_identity: bool,
        cancel_install: bool,
        save_fail: bool,
    }
    impl Operations for Fixture {
        fn detect(&mut self, name: &str, _: &AtomicBool) -> Result<Value> {
            self.calls.push(format!("detect:{name}"));
            Ok(
                json!({"name":name,"available":self.installed.iter().any(|item|item == name) && !self.fail_identity,
                "path":format!("/fixture/{name}"),"version":"fixture-secret"}),
            )
        }
        fn install(&mut self, name: &str, cancel: &AtomicBool) -> Result<()> {
            self.calls.push(format!("install:{name}"));
            self.installed.push(name.into());
            if self.cancel_install {
                cancel.store(true, Ordering::SeqCst);
            }
            Ok(())
        }
        fn accept(&mut self, tool: &Value) -> Result<()> {
            self.calls
                .push(format!("accept:{}", tool["name"].as_str().unwrap()));
            Ok(())
        }
        fn save(&mut self, state: &Value) -> Result<()> {
            if self.save_fail {
                bail!("fixture storage unavailable");
            }
            self.states.push(state.clone());
            Ok(())
        }
        fn redact(&self, text: &str) -> String {
            text.replace("fixture-secret", "[REDACTED_SECRET]")
        }
    }
    #[test]
    fn dependency_order_and_redetection_are_required_before_acceptance() -> Result<()> {
        let setup = Setup::default();
        let mut fixture = Fixture {
            installed: vec!["godot".into()],
            ..Default::default()
        };
        let state = setup.prepare(&["codex", "godot", "codex"], &mut fixture)?;
        assert_eq!(state["status"], "completed");
        assert_eq!(
            fixture.calls,
            [
                "detect:node",
                "install:node",
                "detect:node",
                "accept:node",
                "detect:codex",
                "install:codex",
                "detect:codex",
                "accept:codex",
                "detect:godot",
                "accept:godot"
            ]
        );
        assert!(!setup.running()?);
        assert!(state.get("active").is_none());
        assert_eq!(fixture.states.last(), Some(&state));
        let mut invalid = Fixture {
            fail_identity: true,
            ..Default::default()
        };
        let failed = setup.prepare(&["codex", "godot"], &mut invalid)?;
        assert_eq!(failed["status"], "failed");
        assert_eq!(failed["steps"][0]["status"], "failed");
        assert_eq!(failed["steps"][1]["status"], "waiting");
        assert!(!failed.to_string().contains("fixture-secret"));
        assert!(!invalid
            .calls
            .iter()
            .any(|call| call.starts_with("accept:") || call.contains("godot")));
        Ok(())
    }
    #[test]
    fn cancellation_preserves_installed_tools_and_restart_never_installs() -> Result<()> {
        let setup = Setup::default();
        let mut fixture = Fixture {
            cancel_install: true,
            ..Default::default()
        };
        let state = setup.prepare(&["codex", "blender"], &mut fixture)?;
        assert_eq!(state["status"], "cancelled");
        assert_eq!(fixture.installed, ["node"]);
        assert_eq!(fixture.calls, ["detect:node", "install:node"]);
        assert!(!setup.running()?);
        let previous = json!({"status":"running","active":"codex","steps":[
            {"name":"node","status":"ready"},{"name":"codex","status":"installing"},
            {"name":"godot","status":"waiting"}]});
        let recovered = recover(previous);
        assert_eq!(recovered["status"], "cancelled");
        assert_eq!(recovered["steps"][0]["status"], "ready");
        assert_eq!(recovered["steps"][1]["status"], "cancelled");
        assert_eq!(recovered["steps"][2]["status"], "waiting");
        assert!(recovered.get("active").is_none());
        assert_eq!(recover(recovered.clone()), recovered);
        fixture.cancel_install = false;
        assert_eq!(
            setup.prepare(&["node"], &mut fixture)?["status"],
            "completed"
        );
        Ok(())
    }
    #[test]
    fn concurrent_setup_is_rejected_and_failure_releases_guard() -> Result<()> {
        let setup = Arc::new(Setup::default());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let worker = setup.clone();
        let handle = std::thread::spawn(move || {
            struct Waiting(std::sync::mpsc::Sender<()>);
            impl Operations for Waiting {
                fn detect(&mut self, _: &str, cancel: &AtomicBool) -> Result<Value> {
                    self.0.send(())?;
                    let start = std::time::Instant::now();
                    while !cancel.load(Ordering::SeqCst) {
                        if start.elapsed() > std::time::Duration::from_secs(5) {
                            bail!("fixture timeout");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Ok(json!({"available":false}))
                }
                fn install(&mut self, _: &str, _: &AtomicBool) -> Result<()> {
                    unreachable!()
                }
                fn accept(&mut self, _: &Value) -> Result<()> {
                    unreachable!()
                }
                fn save(&mut self, _: &Value) -> Result<()> {
                    Ok(())
                }
                fn redact(&self, text: &str) -> String {
                    text.into()
                }
            }
            worker.prepare(&["node"], &mut Waiting(started_tx))
        });
        started_rx.recv_timeout(std::time::Duration::from_secs(3))?;
        assert!(setup.running()?);
        assert!(setup.prepare(&["node"], &mut Fixture::default()).is_err());
        setup.cancel()?;
        assert_eq!(handle.join().unwrap()?["status"], "cancelled");
        assert!(!setup.running()?);
        let mut broken = Fixture {
            save_fail: true,
            ..Default::default()
        };
        assert!(setup.prepare(&["node"], &mut broken).is_err());
        assert!(!setup.running()?);
        assert!(setup
            .prepare(&["unknown"], &mut Fixture::default())
            .is_err());
        assert_eq!(
            setup.prepare(&[], &mut Fixture::default())?["status"],
            "completed"
        );
        Ok(())
    }
}
