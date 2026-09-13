use crate::{
    files::{safe_path, Files, Snapshot},
    preferences::{resolve, Vault},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const AUTO_COMPACT_TOKENS: u64 = 65536;
const TOOL_OUTPUT_TOKENS: u64 = 4096;
const PROVIDER_RETRIES: u64 = 1;
const STREAM_IDLE_MS: u64 = 180000;

pub fn latency_overrides() -> [String; 5] {
    [
        format!("model_auto_compact_token_limit={AUTO_COMPACT_TOKENS}"),
        format!("tool_output_token_limit={TOOL_OUTPUT_TOKENS}"),
        format!("model_providers.beaver.request_max_retries={PROVIDER_RETRIES}"),
        format!("model_providers.beaver.stream_max_retries={PROVIDER_RETRIES}"),
        format!("model_providers.beaver.stream_idle_timeout_ms={STREAM_IDLE_MS}"),
    ]
}

pub struct HomeRequest<'a> {
    pub task_id: &'a str,
    pub workspace: &'a Path,
    pub baseline: &'a Snapshot,
    pub capability: &'a str,
    pub skills: &'a Path,
    pub media_executable: &'a Path,
    pub media_args: &'a [String],
    pub resolved_tools: &'a BTreeMap<String, String>,
    pub asset_task: bool,
    pub retained_blender_port: Option<u16>,
}

pub struct PreparedHome {
    pub home: PathBuf,
    pub environment: BTreeMap<OsString, OsString>,
    pub blender: Option<crate::blender_session::Request>,
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("配置路径无效")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

fn copy_skills(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Skill 文件名不是 UTF-8"))?;
        let from = safe_path(source, &name)?;
        let to = safe_path(target, &name)?;
        let metadata = fs::symlink_metadata(&from)?;
        if metadata.is_dir() {
            copy_skills(&from, &to)?;
        } else if metadata.is_file() {
            atomic_write(&to, &fs::read(from)?)?;
        } else {
            bail!("Skill 包含不支持的文件类型");
        }
    }
    Ok(())
}

/// Creates task-scoped Codex configuration without touching the user's CODEX_HOME.
/// Tool discovery and installing the native media server are caller responsibilities.
pub fn prepare(
    root: &Path,
    store: &Store,
    vault: &impl Vault,
    settings: &Value,
    request: HomeRequest<'_>,
    inherited: BTreeMap<OsString, OsString>,
) -> Result<PreparedHome> {
    if request.task_id.is_empty()
        || !request
            .task_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("任务标识无效");
    }
    let provider = resolve(store, vault, settings, request.capability)?;
    let workspace = fs::canonicalize(request.workspace)?;
    if !workspace.is_dir() {
        bail!("任务工作副本不存在");
    }
    if !request.media_executable.is_file() {
        bail!("原生媒体 MCP 程序不存在");
    }
    fs::create_dir_all(root)?;
    let codex = safe_path(root, "codex")?;
    fs::create_dir_all(&codex)?;
    let home = safe_path(&codex, request.task_id)?;
    fs::create_dir_all(&home)?;
    let home = fs::canonicalize(home)?;
    let mut env: BTreeMap<OsString, OsString> = inherited
        .into_iter()
        .filter(|(key, _)| {
            let key = key.to_string_lossy().to_ascii_uppercase();
            !key.starts_with("CODEX_")
                && !key.starts_with("OPENAI_")
                && !key.starts_with("BEAVER_")
                && key != "ELECTRON_RUN_AS_NODE"
        })
        .collect();
    env.insert("CODEX_HOME".into(), home.clone().into_os_string());
    let baseline = crate::code_structure::snapshot_baseline(
        &Files::new(root.to_path_buf()),
        request.baseline,
    )?;
    let baseline_path = safe_path(&home, "code-structure-baseline.json")?;
    atomic_write(&baseline_path, &serde_json::to_vec(&baseline)?)?;
    env.insert(
        crate::code_structure::BASELINE_ENV.into(),
        baseline_path.into_os_string(),
    );
    env.insert("BEAVER_CODEX_KEY".into(), provider.key.clone().into());
    env.insert(
        "BEAVER_PROJECT_ROOT".into(),
        workspace.clone().into_os_string(),
    );
    let mut media = serde_json::Map::new();
    for capability in ["image", "speech", "music", "translation"] {
        media.insert(
            capability.into(),
            match resolve(store, vault, settings, capability) {
                Ok(provider) => serde_json::to_value(provider)?,
                Err(_) => Value::Null,
            },
        );
    }
    env.insert(
        "BEAVER_MEDIA_PROVIDERS".into(),
        Value::Object(media).to_string().into(),
    );
    if let Some(node) = request
        .resolved_tools
        .get("node")
        .filter(|v| v.as_str() != "not installed")
    {
        if let Some(parent) = Path::new(node).parent() {
            let path_key = env
                .keys()
                .find(|k| k.to_string_lossy().eq_ignore_ascii_case("PATH"))
                .cloned()
                .unwrap_or_else(|| "PATH".into());
            let mut paths = vec![parent.to_path_buf()];
            if let Some(old) = env.get(&path_key) {
                paths.extend(std::env::split_paths(old));
            }
            env.insert(path_key, std::env::join_paths(paths)?);
        }
    }
    let mut config = json!({
        "model":provider.model,"model_provider":"beaver","approval_policy":"never","sandbox_mode":"danger-full-access",
        "model_auto_compact_token_limit":AUTO_COMPACT_TOKENS,"tool_output_token_limit":TOOL_OUTPUT_TOKENS,
        "model_providers":{"beaver":{"name":"Beaver configured AI","base_url":provider.base_url,"wire_api":"responses","request_max_retries":PROVIDER_RETRIES,"stream_max_retries":PROVIDER_RETRIES,"stream_idle_timeout_ms":STREAM_IDLE_MS}},
        "mcp_servers":{"beaver_media":{"command":fs::canonicalize(request.media_executable)?,"args":request.media_args,"env_vars":["BEAVER_PROJECT_ROOT","BEAVER_MEDIA_PROVIDERS",crate::code_structure::BASELINE_ENV],"tool_timeout_sec":600}}
    });
    if !provider.key.is_empty() {
        config["model_providers"]["beaver"]["env_key"] = json!("BEAVER_CODEX_KEY");
    }
    if settings["mcp"]["godot"] == true {
        config["mcp_servers"]["godot"] =
            json!({"command":"npx","args":["--yes","@coding-solo/godot-mcp@0.1.1"]});
        if let Some(godot) = request
            .resolved_tools
            .get("godot")
            .filter(|s| s.as_str() != "not installed")
        {
            config["mcp_servers"]["godot"]["env"] = json!({"GODOT_PATH":godot});
        }
    }
    let mut blender = None;
    if request.asset_task && settings["mcp"]["blender"] != true {
        bail!("请在设置中启用 Blender MCP 后创建资产制作任务");
    }
    if settings["mcp"]["blender"] == true {
        config["mcp_servers"]["blender"] = json!({"command":"uvx","args":["blender-mcp==1.9.1"],"env":{"DISABLE_TELEMETRY":"true"}});
        if request.asset_task
            || request.retained_blender_port.is_some()
            || crate::workflows::runtime(&workspace)?.is_some()
        {
            let port = if let Some(port) = request.retained_blender_port {
                port
            } else {
                let executable = request
                    .resolved_tools
                    .get("blender")
                    .context("Configure Blender for asset production")?;
                let session = crate::blender_session::Request::prepare(
                    &home,
                    &workspace,
                    Path::new(executable),
                )?;
                let port = session.port;
                blender = Some(session);
                port
            };
            config["mcp_servers"]["blender"]["env"]["BLENDER_HOST"] = json!("127.0.0.1");
            config["mcp_servers"]["blender"]["env"]["BLENDER_PORT"] = json!(port.to_string());
            config["mcp_servers"]["blender"]["tool_timeout_sec"] = json!(600);
        }
    }
    let config_text = toml::to_string(&config)?;
    let agents = format!("# Beaver execution environment\nWork only in the supplied project copy. Do not edit the original project or application state. Complete the user's game goal autonomously and verify the result. Do not claim unavailable tools or failed exports succeeded. Use the supplied game-production skills. Detected executable paths: {}.\n", serde_json::to_string(request.resolved_tools)?);
    let asset_instructions =
        if request.asset_task || blender.is_some() || request.retained_blender_port.is_some() {
            crate::asset_tool::INSTRUCTIONS
        } else {
            ""
        };
    let agents = format!(
        "{agents}\n{asset_instructions}\n{}",
        crate::code_structure::INSTRUCTIONS
    );
    let skills = safe_path(&home, "skills")?;
    copy_skills(request.skills, &skills)?;
    atomic_write(&safe_path(&home, "config.toml")?, config_text.as_bytes())?;
    atomic_write(&safe_path(&home, "AGENTS.md")?, agents.as_bytes())?;
    Ok(PreparedHome {
        home,
        environment: env,
        blender,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestVault;
    impl Vault for TestVault {
        fn encrypt(&self, _: &str) -> Result<String> {
            unreachable!()
        }
        fn decrypt(&self, _: &str) -> Result<String> {
            Ok("dummy-env-only-key".into())
        }
    }
    #[test]
    fn isolates_config_and_keeps_secrets_out_of_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let store = Store::open(&root)?;
        store.put("secret", "code", &"test-cipher")?;
        let skills = temp.path().join("skills");
        fs::create_dir_all(skills.join("production"))?;
        fs::write(skills.join("production/SKILL.md"), "# 游戏制作\n")?;
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace)?;
        fs::write(workspace.join("AGENTS.md"), "# User-owned project rules\n")?;
        let mut settings: Value =
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?;
        settings["local"]["code"]["baseUrl"] = json!("https://example.com/v1");
        settings["local"]["code"]["model"] = json!("model\"quoted");
        settings["mcp"] = json!({"godot":true,"blender":true});
        let tools = BTreeMap::from([("godot".into(), "C:/Tools/Godot.exe".into())]);
        let executable = std::env::current_exe()?;
        let user_home = temp.path().join("user-codex");
        fs::create_dir(&user_home)?;
        fs::write(
            user_home.join("config.toml"),
            "# user config must remain unchanged\n",
        )?;
        let prepared = prepare(
            &root,
            &store,
            &TestVault,
            &settings,
            HomeRequest {
                task_id: "task-1",
                asset_task: false,
                retained_blender_port: None,
                workspace: &workspace,
                baseline: &Snapshot::new(),
                capability: "code",
                skills: &skills,
                media_executable: &executable,
                media_args: &["--media-mcp".into()],
                resolved_tools: &tools,
            },
            BTreeMap::from([
                ("CODEX_HOME".into(), user_home.clone().into_os_string()),
                ("OPENAI_API_KEY".into(), "old-secret".into()),
                ("ELECTRON_RUN_AS_NODE".into(), "1".into()),
            ]),
        )?;
        assert!(prepared.home.is_absolute());
        assert!(fs::read_to_string(prepared.home.join("AGENTS.md"))?
            .ends_with(crate::code_structure::INSTRUCTIONS));
        assert_eq!(
            fs::read_to_string(workspace.join("AGENTS.md"))?,
            "# User-owned project rules\n"
        );
        let baseline_path = prepared
            .environment
            .get(&OsString::from(crate::code_structure::BASELINE_ENV))
            .unwrap();
        assert!(
            crate::code_structure::Baseline::parse(&fs::read(Path::new(baseline_path))?)?
                .files
                .is_empty()
        );
        assert_eq!(
            prepared
                .environment
                .get(&OsString::from("BEAVER_CODEX_KEY"))
                .unwrap(),
            "dummy-env-only-key"
        );
        assert!(!prepared
            .environment
            .contains_key(&OsString::from("OPENAI_API_KEY")));
        assert!(!prepared
            .environment
            .contains_key(&OsString::from("ELECTRON_RUN_AS_NODE")));
        let config = fs::read_to_string(prepared.home.join("config.toml"))?;
        assert!(!config.contains("dummy-env-only-key") && !config.contains("old-secret"));
        let parsed: toml::Value = toml::from_str(&config)?;
        assert_eq!(parsed["model"].as_str(), Some("model\"quoted"));
        assert_eq!(
            parsed["model_auto_compact_token_limit"].as_integer(),
            Some(65536)
        );
        assert_eq!(parsed["tool_output_token_limit"].as_integer(), Some(4096));
        assert_eq!(
            parsed["model_providers"]["beaver"]["request_max_retries"].as_integer(),
            Some(1)
        );
        assert_eq!(
            parsed["model_providers"]["beaver"]["stream_max_retries"].as_integer(),
            Some(1)
        );
        assert_eq!(
            parsed["model_providers"]["beaver"]["stream_idle_timeout_ms"].as_integer(),
            Some(180000)
        );
        assert_eq!(
            parsed["mcp_servers"]["beaver_media"]["args"][0].as_str(),
            Some("--media-mcp")
        );
        assert_eq!(
            fs::read_to_string(prepared.home.join("skills/production/SKILL.md"))?,
            "# 游戏制作\n"
        );
        fs::create_dir(prepared.home.join("sessions"))?;
        fs::write(
            prepared.home.join("sessions/preserved.jsonl"),
            "session fixture",
        )?;
        let resumed = prepare(
            &root,
            &store,
            &TestVault,
            &settings,
            HomeRequest {
                task_id: "task-1",
                asset_task: false,
                retained_blender_port: None,
                workspace: &workspace,
                baseline: &Snapshot::new(),
                capability: "code",
                skills: &skills,
                media_executable: &executable,
                media_args: &[],
                resolved_tools: &tools,
            },
            BTreeMap::new(),
        )?;
        assert_eq!(resumed.home, prepared.home);
        assert!(fs::read_to_string(resumed.home.join("AGENTS.md"))?
            .ends_with(crate::code_structure::INSTRUCTIONS));
        assert_eq!(
            fs::read_to_string(resumed.home.join("sessions/preserved.jsonl"))?,
            "session fixture"
        );
        assert_eq!(
            fs::read_to_string(user_home.join("config.toml"))?,
            "# user config must remain unchanged\n"
        );
        assert!(prepare(
            &root,
            &store,
            &TestVault,
            &settings,
            HomeRequest {
                task_id: "../escape",
                asset_task: false,
                retained_blender_port: None,
                workspace: &workspace,
                baseline: &Snapshot::new(),
                capability: "code",
                skills: &skills,
                media_executable: &executable,
                media_args: &[],
                resolved_tools: &tools,
            },
            BTreeMap::new()
        )
        .is_err());
        fs::write(
            workspace.join(crate::workflows::RUNTIME_FILE),
            json!({"schemaVersion":1,"packages":{"npr-characters":{"status":"ready"}}}).to_string(),
        )?;
        let mut tools = tools;
        tools.insert("blender".into(), executable.to_string_lossy().into_owned());
        let managed = prepare(
            &root,
            &store,
            &TestVault,
            &settings,
            HomeRequest {
                task_id: "managed-task",
                asset_task: false,
                retained_blender_port: None,
                workspace: &workspace,
                baseline: &Snapshot::new(),
                capability: "code",
                skills: &skills,
                media_executable: &executable,
                media_args: &[],
                resolved_tools: &tools,
            },
            BTreeMap::new(),
        )?;
        let managed_config: toml::Value =
            toml::from_str(&fs::read_to_string(managed.home.join("config.toml"))?)?;
        let port = managed.blender.as_ref().unwrap().port;
        assert_eq!(
            managed_config["mcp_servers"]["blender"]["env"]["BLENDER_PORT"].as_str(),
            Some(port.to_string().as_str())
        );
        assert_eq!(
            managed_config["mcp_servers"]["blender"]["env"]["BLENDER_HOST"].as_str(),
            Some("127.0.0.1")
        );
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
        Ok(())
    }
}
