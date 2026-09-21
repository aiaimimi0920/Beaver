use crate::{
    codex_home::{self, HomeRequest},
    execution_settings::ExecutionSettings,
    files::Files,
    scheduler::Launch,
    store::Store,
    task_brief,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub struct Resources {
    pub skills: PathBuf,
    pub media: PathBuf,
    pub catalog: Value,
    pub ask_user_tool: Value,
}

/// Called on the scheduler's blocking preparation worker, before any process starts.
pub fn prepare(
    files: &Files,
    store: &Store,
    settings: &ExecutionSettings,
    task: &Value,
    resources: &Resources,
    tools: &BTreeMap<String, String>,
    inherited: BTreeMap<OsString, OsString>,
) -> Result<Launch> {
    if crate::validation::task_gate::validation_only(task) {
        return Ok(Launch {
            command: None,
            model: String::new(),
            prompt: String::new(),
            ask_user_tool: Value::Null,
            secrets: vec![],
            max_minutes: 0,
            blender: None,
            godot: tools
                .get("godot")
                .map(PathBuf::from)
                .filter(|p| p.is_file()),
        });
    }
    let codex = Path::new(tools.get("codex").context("请先安装或配置 Codex")?);
    if !codex.is_file() {
        anyhow::bail!("Codex 程序不存在，请重新配置路径");
    }
    let codex = std::fs::canonicalize(codex)?;
    let capability = task["capability"].as_str().context("任务能力无效")?;
    settings.require_capability(capability)?;
    let provider = settings.provider()?;
    let task_id = task["id"].as_str().context("任务标识无效")?;
    let recorded_workspace = Path::new(task["workspace"].as_str().context("任务工作副本无效")?);
    let workspace = files.resolve_workspace(task_id, recorded_workspace)?;
    let baseline = serde_json::from_value(task["baseline"].clone()).context("任务基线无效")?;
    crate::validation::task_completion::freeze_repair(store, files, &workspace, task)?;
    let environment = codex_home::prepare(
        files,
        settings,
        HomeRequest {
            task_id,
            asset_task: task["assetTask"] == true,
            retained_blender_port: task["retainedBlenderPort"]
                .as_u64()
                .and_then(|port| u16::try_from(port).ok())
                .filter(|port| *port != 0),
            workspace: recorded_workspace,
            baseline: &baseline,
            skills: &resources.skills,
            media_executable: &resources.media,
            media_args: &[crate::media_server::MODE_ARGUMENT.into()],
            resolved_tools: tools,
        },
        inherited,
    )?;
    let mut command = tokio::process::Command::new(codex);
    command
        .arg("app-server")
        .args(["-c", "model_provider=\"beaver\""])
        .current_dir(workspace)
        .env_clear()
        .envs(environment.environment);
    // Workspace/ancestor .codex files must not loosen Beaver's execution limits.
    for value in codex_home::latency_overrides() {
        command.arg("-c").arg(value);
    }
    Ok(Launch {
        command: Some(command),
        model: provider.model.clone(),
        prompt: task_brief::prompt(task, &resources.catalog),
        ask_user_tool: resources.ask_user_tool.clone(),
        secrets: settings.secrets(),
        max_minutes: task["maxMinutes"].as_u64().unwrap_or(0),
        blender: environment.blender,
        godot: tools
            .get("godot")
            .map(PathBuf::from)
            .filter(|p| p.is_file()),
    })
}
