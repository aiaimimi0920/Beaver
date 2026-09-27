//! Isolated, ephemeral app-server launch shared by planning and title suggestions.
use crate::{codex_home, execution_settings::ExecutionSettings};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use tokio::process::Command;

pub struct Prepared {
    pub command: Command,
    pub cwd: PathBuf,
    pub model: String,
    pub secrets: Vec<String>,
    pub scratch: tempfile::TempDir,
}

pub fn prepare(
    settings: &ExecutionSettings,
    codex: &Path,
    instructions: &str,
    inherited: BTreeMap<OsString, OsString>,
) -> Result<Prepared> {
    settings.require_capability("code")?;
    let provider = settings.provider()?;
    let executable = fs::canonicalize(codex)?;
    ensure!(executable.is_file(), "Codex executable is unavailable");
    let scratch = tempfile::Builder::new()
        .prefix("beaver-read-only-")
        .tempdir()?;
    let home = scratch.path().join("home");
    let cwd = scratch.path().join("empty");
    fs::create_dir(&home)?;
    fs::create_dir(&cwd)?;
    let mut config = codex_home::provider_config(provider, "read-only");
    for (key, value) in restrictions().as_object().unwrap() {
        config[key] = value.clone();
    }
    fs::write(home.join("config.toml"), toml::to_string(&config)?)?;
    fs::write(home.join("AGENTS.md"), instructions)?;
    let mut environment = codex_home::isolated_environment(&home, inherited);
    environment.insert("BEAVER_CODEX_KEY".into(), provider.key.clone().into());
    let mut command = Command::new(executable);
    command
        .arg("app-server")
        .current_dir(&cwd)
        .env_clear()
        .envs(environment);
    for value in codex_home::latency_overrides() {
        command.arg("-c").arg(value);
    }
    Ok(Prepared {
        command,
        cwd,
        model: provider.model.clone(),
        secrets: settings.secrets(),
        scratch,
    })
}

pub(crate) fn restrictions() -> Value {
    json!({
        "approval_policy":"never","sandbox_mode":"read-only","web_search":"disabled",
        "mcp_servers":{},"project_doc_max_bytes":0,
        "features":{"shell_tool":false,"unified_exec":false,"multi_agent":false,
            "multi_agent_v2":false,"view_image":false,"image_generation":false,
            "skill_search":false,"skip_host_skill_discovery":true}
    })
}
