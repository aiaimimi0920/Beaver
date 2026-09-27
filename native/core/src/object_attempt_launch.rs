//! Isolated model file processing followed by optional host-owned Godot import.
//! Interactive Godot/Blender sessions remain unavailable to the model.
use crate::{
    codex_home, execution_settings::ExecutionSettings, object_attempt::Attempt,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::process::Command;

pub type Factory = Arc<dyn Fn(&Attempt, &ProjectRuntime) -> Result<Launch, String> + Send + Sync>;

pub struct Launch {
    pub command: Command,
    pub cwd: PathBuf,
    pub model: String,
    pub secrets: Vec<String>,
    pub timeout: Duration,
    pub godot: Option<crate::object_attempt_godot::Import>,
}

pub const INSTRUCTIONS: &str = concat!(
    "Execute only the supplied fine task in this shared object workspace. ",
    "The task definition, acceptance criteria and input snapshot are frozen. ",
    "Use beaver_object_attempt to discover available Beaver capabilities, inspect the frozen context or read original input files by path and hash. Its scope is bound to this attempt; unavailable capabilities must not be assumed. ",
    "For frozen PNG feedback, call inputFile with its path and hash and inspect the returned image before editing. Region numbers follow the feedback array order in normalized top-left image coordinates; these are historical image regions, not engine hits. Recheck the current content before applying changes. ",
    "When context.feedbackImage is present, call feedbackImage without arguments other than operation to inspect that historical publication feedback. It is bound to this fine task and may differ from the current input; use its exact numbered regions and re-localize against current content. ",
    "Do not edit application state, the original project, sibling workspaces or personal configuration. ",
    "This execution supports file processing only. Do not start Godot, Blender, background services or detached writers. ",
    "Do not start another fine task, merge, publish, accept a result or claim that a completed turn passes any gate. ",
    "Finish all owned file operations before returning. If clarification or an unavailable managed tool is necessary, report the blocker without guessing success. ",
);

pub fn prepare(
    runtime: &ProjectRuntime,
    attempt: &Attempt,
    settings: &ExecutionSettings,
    codex: &Path,
    inherited: BTreeMap<OsString, OsString>,
) -> Result<Launch> {
    settings.require_capability("code")?;
    let godot = crate::object_attempt_godot::configured(settings)?;
    ensure!(
        attempt.preparation.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let files = runtime.files();
    let cwd = fs::canonicalize(files.resolve_workspace(
        &attempt.preparation.run.id,
        Path::new(&attempt.preparation.workspace),
    )?)?;
    let home = files.codex_home(&attempt.id)?;
    let executable = fs::canonicalize(codex)?;
    ensure!(
        executable.is_file() && cwd.is_dir(),
        "OBJECT_ATTEMPT_EXECUTION_PATH_MISSING"
    );
    fs::create_dir_all(
        home.parent()
            .ok_or_else(|| anyhow::anyhow!("OBJECT_ATTEMPT_HOME_INVALID"))?,
    )?;
    fs::create_dir(&home)?;
    let provider = settings.provider()?;
    let mut config = codex_home::provider_config(provider, "workspace-write");
    for (key, value) in restrictions().as_object().unwrap() {
        config[key] = value.clone();
    }
    fs::write(home.join("config.toml"), toml::to_string(&config)?)?;
    fs::write(
        home.join("AGENTS.md"),
        format!("{INSTRUCTIONS}\n{}", crate::code_structure::INSTRUCTIONS),
    )?;
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
    Ok(Launch {
        command,
        cwd,
        model: provider.model.clone(),
        secrets: settings.secrets(),
        timeout: Duration::from_secs(1800),
        godot,
    })
}

pub(crate) fn restrictions() -> Value {
    json!({
        "approval_policy":"never","sandbox_mode":"workspace-write","web_search":"disabled",
        "mcp_servers":{},"project_doc_max_bytes":0,
        "features":{"multi_agent":false,"multi_agent_v2":false,"image_generation":false,
            "skill_search":false,"skip_host_skill_discovery":true}
    })
}
