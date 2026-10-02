use super::{attempt_fixture::*, queue_fixture};
use crate::{
    execution_settings::ExecutionSettings, object_attempt, object_attempt_launch,
    object_run_preparation, preferences::Vault, store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{collections::BTreeMap, ffi::OsString, fs};

struct TestVault;
impl Vault for TestVault {
    fn encrypt(&self, _: &str) -> Result<String> {
        unreachable!()
    }
    fn decrypt(&self, cipher: &str) -> Result<String> {
        assert_eq!(cipher, "test-cipher");
        Ok("test-provider-key".into())
    }
}

#[test]
fn isolated_launch_preserves_project_personal_home_and_existing_attempt_sessions() -> Result<()> {
    let fixture = fixture()?;
    let lease = start(&fixture)?;
    let attempt = lease.record();
    let project_before = fs::read(fixture.temp.path().join("project.godot"))?;
    let host = Store::open(&fixture.temp.path().join("host"))?;
    host.put("secret", "code", &"test-cipher")?;
    let mut defaults: Value =
        serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?;
    defaults["mode"] = json!("local");
    defaults["mcp"]["godot"] = json!(true);
    defaults["tools"]["godot"] = json!("configured-godot");
    defaults["local"]["code"]["baseUrl"] = json!("https://example.com/v1");
    defaults["local"]["code"]["model"] = json!("test-model");
    let settings = ExecutionSettings::read(&host, &TestVault, defaults, Some("code"))?;
    let personal = fixture.temp.path().join("personal-home");
    fs::create_dir(&personal)?;
    fs::write(personal.join("config.toml"), "personal configuration")?;
    fs::write(personal.join("session.jsonl"), "personal session")?;
    let mut inherited: BTreeMap<OsString, OsString> = [
        ("PATH", "retained-path"),
        ("CODEX_TEST_MODE", "inherited-codex"),
        ("OPENAI_API_KEY", "inherited-key"),
        ("openai_base_url", "inherited-url"),
        ("BEAVER_CODEX_KEY", "inherited-beaver"),
        ("BEAVER_TEST", "inherited-test"),
        ("ELECTRON_RUN_AS_NODE", "1"),
    ]
    .into_iter()
    .map(|(key, value)| (key.into(), value.into()))
    .collect();
    inherited.insert("CODEX_HOME".into(), personal.as_os_str().to_owned());
    let executable = std::env::current_exe()?;
    let launch = object_attempt_launch::prepare(
        &fixture.runtime,
        attempt,
        &settings,
        &executable,
        inherited,
    )?;
    let home = fixture.runtime.files().codex_home(&attempt.id)?;
    assert_eq!(
        launch.godot.as_ref().unwrap().executable,
        std::path::PathBuf::from("configured-godot")
    );
    let command = launch.command.as_std();
    let environment: BTreeMap<_, _> = command
        .get_envs()
        .map(|(key, value)| (key.to_os_string(), value.unwrap().to_os_string()))
        .collect();
    assert_eq!(
        environment,
        BTreeMap::from([
            (OsString::from("PATH"), OsString::from("retained-path")),
            (OsString::from("CODEX_HOME"), home.as_os_str().to_owned()),
            (
                OsString::from("BEAVER_CODEX_KEY"),
                OsString::from("test-provider-key")
            ),
        ])
    );
    assert_eq!(
        command.get_program(),
        fs::canonicalize(&executable)?.as_os_str()
    );
    assert_eq!(command.get_current_dir(), Some(launch.cwd.as_path()));
    assert_eq!(launch.cwd, fs::canonicalize(workspace(&fixture, attempt)?)?);
    assert_eq!(
        command.get_args().next(),
        Some(std::ffi::OsStr::new("app-server"))
    );
    let config_text = fs::read_to_string(home.join("config.toml"))?;
    let config: Value = toml::from_str(&config_text)?;
    assert_eq!(config["sandbox_mode"], "workspace-write");
    if cfg!(windows) {
        assert_eq!(config["windows"]["sandbox"], "unelevated");
    } else {
        assert!(config.get("windows").is_none());
    }
    assert_eq!(config["approval_policy"], "never");
    assert_eq!(config["web_search"], "disabled");
    assert_eq!(config["project_doc_max_bytes"], 0);
    assert_eq!(config["mcp_servers"], json!({}));
    assert_eq!(config["features"]["skip_host_skill_discovery"], true);
    assert_eq!(config["features"]["multi_agent"], false);
    assert_eq!(config["model"], "test-model");
    assert_eq!(
        config["model_providers"]["beaver"]["env_key"],
        "BEAVER_CODEX_KEY"
    );
    let instructions = fs::read_to_string(home.join("AGENTS.md"))?;
    assert!(instructions.contains("file processing only"));
    assert!(!config_text.contains("test-provider-key"));
    assert!(!instructions.contains("test-provider-key"));
    fs::write(home.join("session.jsonl"), "existing attempt session")?;
    assert!(object_attempt_launch::prepare(
        &fixture.runtime,
        attempt,
        &settings,
        &executable,
        BTreeMap::new(),
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(home.join("session.jsonl"))?,
        "existing attempt session"
    );
    assert_eq!(fs::read_to_string(home.join("config.toml"))?, config_text);
    queue_fixture::enqueue(&fixture, &["independent"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "other")?.unwrap();
    let second = object_attempt::start(&fixture.runtime, claim)?.unwrap();
    object_attempt_launch::prepare(
        &fixture.runtime,
        second.record(),
        &settings,
        &executable,
        BTreeMap::new(),
    )?;
    let second_home = fixture.runtime.files().codex_home(&second.record().id)?;
    assert_ne!(second_home, home);
    assert!(second_home.join("config.toml").is_file());
    assert!(!second_home.join("session.jsonl").exists());
    assert_eq!(
        fs::read(fixture.temp.path().join("project.godot"))?,
        project_before
    );
    assert_eq!(
        fs::read_to_string(personal.join("config.toml"))?,
        "personal configuration"
    );
    assert_eq!(
        fs::read_to_string(personal.join("session.jsonl"))?,
        "personal session"
    );
    Ok(())
}
