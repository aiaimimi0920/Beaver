use super::{read, scan, Baseline, Scope, BASELINE_ENV};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

pub const NAME: &str = "beaver_check_code_structure";

pub fn definition() -> Value {
    json!({
        "name":NAME,
        "description":"Check project source sizes before completion. Read-only; target 100-250 effective lines, split above 500, no changed/new file above 700. Reports canonical hashes for rare 501-700 exceptions. Unchanged task-baseline debt is allowed.",
        "inputSchema":{"type":"object","properties":{},"additionalProperties":false},
        "annotations":{"readOnlyHint":true,"destructiveHint":false}
    })
}

pub async fn call(root: PathBuf, input: Value) -> Result<String> {
    ensure!(
        input.as_object().is_some_and(|object| object.is_empty()),
        "Expected empty code-structure arguments"
    );
    let baseline_path = std::env::var_os(BASELINE_ENV).map(PathBuf::from);
    tokio::task::spawn_blocking(move || {
        let baseline = match baseline_path {
            Some(path) => Baseline::parse(&read(&path, 8 * 1024 * 1024)?)?,
            None => Baseline::empty(),
        };
        let mut report = scan(&root, Scope::Game, &baseline, false)?;
        let checked = report.files.len();
        report.files.sort_by(|a, b| b.stamp.effective_lines.cmp(&a.stamp.effective_lines).then(a.path.cmp(&b.path)));
        report.files.truncate(40);
        Ok(json!({"ok":report.ok,"checkedFiles":checked,"files":report.files,"violations":report.violations,"displayLimit":40}).to_string())
    }).await.context("Code-structure worker stopped")?
}
