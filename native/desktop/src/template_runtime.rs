use crate::Backend;
use anyhow::{bail, Result};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

struct Active<'a>(&'a Backend);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.template_cancel.lock() {
            *active = None;
        }
    }
}
pub fn cancel(backend: &Backend) -> Result<()> {
    if let Some(token) = backend
        .template_cancel
        .lock()
        .map_err(|_| anyhow::anyhow!("模板准备锁不可用"))?
        .as_ref()
    {
        token.store(true, Ordering::SeqCst);
    }
    Ok(())
}
pub fn prepare(backend: &Backend, import: bool, path: Option<std::path::PathBuf>) -> Result<Value> {
    if backend.closing.load(Ordering::SeqCst) {
        bail!("应用正在退出");
    }
    // A pending human file choice must not hold the shutdown/operation gate.
    let archive = if import {
        let selected = path.or_else(|| {
            rfd::FileDialog::new()
                .set_title("导入可信的 Godot Windows x86_64 导出模板")
                .add_filter("Godot 导出模板", &["tpz"])
                .pick_file()
        });
        if selected.is_none() {
            return Ok(Value::Null);
        }
        selected
    } else {
        None
    };
    let _gate = backend
        .template_gate
        .try_lock()
        .map_err(|_| anyhow::anyhow!("导出模板正在准备中"))?;
    let token = Arc::new(AtomicBool::new(false));
    {
        let mut active = backend
            .template_cancel
            .lock()
            .map_err(|_| anyhow::anyhow!("模板准备锁不可用"))?;
        if backend.closing.load(Ordering::SeqCst) {
            bail!("应用正在退出");
        }
        *active = Some(token.clone());
    }
    let _active = Active(backend);
    if token.load(Ordering::SeqCst) || backend.closing.load(Ordering::SeqCst) {
        bail!("模板准备已取消");
    }
    let configured = {
        let store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let settings = beaver_core::preferences::read(
            &store,
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
        )?;
        settings["tools"]["godot"].as_str().unwrap_or("").to_owned()
    };
    let executable = beaver_core::tools::find("godot", &configured)?;
    let app_data = std::env::var_os("APPDATA").map(std::path::PathBuf::from);
    tauri::async_runtime::block_on(beaver_core::export_templates::prepare(
        &executable,
        archive.as_deref(),
        app_data.as_deref(),
        &token,
    ))
}
