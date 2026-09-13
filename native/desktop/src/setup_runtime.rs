use crate::Backend;
use anyhow::{bail, Context, Result};
use beaver_core::{
    preferences,
    tool_setup::{self, Operations},
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

struct Adapter<'a> {
    backend: &'a Backend,
    app: tauri::AppHandle,
    configured: Value,
    secrets: Vec<String>,
}
impl Adapter<'_> {
    fn check(&self, cancelled: &AtomicBool) {
        if self.backend.closing.load(Ordering::SeqCst) {
            cancelled.store(true, Ordering::SeqCst);
        }
    }
}
impl Operations for Adapter<'_> {
    fn detect(&mut self, name: &str, cancelled: &AtomicBool) -> Result<Value> {
        self.check(cancelled);
        let mut configured = self.configured[name].as_str().unwrap_or("").to_owned();
        if name == "codex" && configured.is_empty() {
            if let Some(binary) = beaver_core::tools::native_codex(&self.backend.root.join("tools"))
            {
                configured = binary.to_string_lossy().into_owned();
            }
        }
        beaver_core::tools::detect_one(name, &configured, &self.secrets, cancelled)
    }
    fn install(&mut self, name: &str, cancelled: &AtomicBool) -> Result<()> {
        self.check(cancelled);
        beaver_core::tool_install::install(
            name,
            &self.backend.root,
            self.configured["node"].as_str().unwrap_or(""),
            cancelled,
        )?;
        // An explicit stale path must not hide the freshly managed installation.
        if name == "codex" {
            if let Some(binary) = beaver_core::tools::native_codex(&self.backend.root.join("tools"))
            {
                self.configured[name] = json!(binary);
            }
        }
        Ok(())
    }
    fn accept(&mut self, tool: &Value) -> Result<()> {
        let name = tool["name"].as_str().context("工具名称无效")?;
        let mut store = self
            .backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let mut settings = preferences::read(
            &store,
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
        )?;
        settings["tools"][name] = tool["path"].clone();
        preferences::save(&mut store, &preferences::SystemVault, settings, json!({}))?;
        self.configured[name] = tool["path"].clone();
        Ok(())
    }
    fn save(&mut self, state: &Value) -> Result<()> {
        self.backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?
            .put("toolSetup", "main", state)?;
        let _ = self.app.emit("beaver:changed", ());
        Ok(())
    }
    fn redact(&self, message: &str) -> String {
        let mut message = message.to_owned();
        for secret in &self.secrets {
            message = message.replace(secret, "[REDACTED_SECRET]");
        }
        message
    }
}

pub fn prepare(
    backend: &Backend,
    app: tauri::AppHandle,
    method: &str,
    input: Value,
) -> Result<Value> {
    let _gate = backend
        .setup_gate
        .try_lock()
        .map_err(|_| anyhow::anyhow!("已有环境准备正在进行"))?;
    if backend.closing.load(Ordering::SeqCst) {
        bail!("应用正在退出");
    }
    let (settings, mut secrets) = {
        let store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let settings = preferences::read(
            &store,
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
        )?;
        let secrets = preferences::CAPABILITIES
            .into_iter()
            .chain(["cloud"])
            .map(|slot| preferences::key(&store, &preferences::SystemVault, slot))
            .collect::<Result<Vec<_>>>()?;
        (settings, secrets)
    };
    secrets.retain(|secret| !secret.is_empty());
    secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
    let configured = input
        .get("tools")
        .cloned()
        .unwrap_or(settings["tools"].clone());
    for name in ["node", "codex", "godot", "blender"] {
        if configured[name]
            .as_str()
            .is_none_or(|value| value.encode_utf16().count() > 2000)
        {
            bail!("工具配置格式无效");
        }
    }
    let names = if method == "tools.install" {
        vec![input["name"].as_str().context("缺少工具名称")?]
    } else {
        vec!["node", "codex", "godot", "blender"]
    };
    let state = backend.setup.prepare(
        &names,
        &mut Adapter {
            backend,
            app,
            configured,
            secrets,
        },
    )?;
    if method == "tools.install" && state["status"] != "completed" {
        bail!("{}", state["error"].as_str().unwrap_or("环境准备未完成"));
    }
    Ok(state)
}

pub fn recover(store: &beaver_core::store::Store) -> Result<()> {
    let previous = store
        .get::<Value>("toolSetup", "main")?
        .unwrap_or_else(tool_setup::idle);
    store.put("toolSetup", "main", &tool_setup::recover(previous))
}
