use super::*;
use crate::{
    files::Files,
    launch::{self, Resources},
};
use std::{ffi::OsStr, fs};

struct TestVault;
impl Vault for TestVault {
    fn encrypt(&self, _: &str) -> Result<String> {
        unreachable!()
    }
    fn decrypt(&self, cipher: &str) -> Result<String> {
        cipher
            .strip_prefix("fixture:")
            .map(str::to_owned)
            .context("Invalid fixture cipher")
    }
}

fn configured() -> Value {
    let mut settings: Value =
        serde_json::from_str(include_str!("../../../dist-native/default-settings.json")).unwrap();
    settings["mode"] = json!("local");
    for capability in preferences::CAPABILITIES {
        settings["local"][capability] = json!({"baseUrl":"https://host.example/v1","model":format!("host-{capability}"),"route":"fixture"});
    }
    settings["mcp"] = json!({"godot":false,"blender":false});
    settings
}

#[test]
fn launch_uses_frozen_host_configuration_and_never_project_credentials() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    host.put("settings", "main", &configured())?;
    host.put("secret", "code", &"fixture:host-key")?;
    host.put("secret", "image", &"fixture:host-image-key")?;
    let snapshot = ExecutionSettings::read(&host, &TestVault, Value::Null, Some("code"))?;
    let mut changed = configured();
    changed["local"]["code"]["model"] = json!("later-model");
    host.put("settings", "main", &changed)?;
    host.put("secret", "code", &"fixture:later-key")?;

    let root = temp.path().join("tasks");
    let project = Store::open(&root)?;
    // These deliberately cannot be read as preferences or decrypted as credentials.
    project.put(
        "settings",
        "main",
        &json!(["project data is not host settings"]),
    )?;
    project.put("secret", "code", &json!({"invalid":"project credential"}))?;
    let workspace = temp.path().join("workspace");
    let skills = temp.path().join("skills");
    fs::create_dir(&workspace)?;
    fs::create_dir(&skills)?;
    let executable = std::env::current_exe()?;
    let resources = Resources {
        skills,
        media: executable.clone(),
        catalog: Value::Null,
        ask_user_tool: Value::Null,
    };
    let tools = BTreeMap::from([("codex".into(), executable.to_string_lossy().into_owned())]);
    let task = json!({"id":"host-boundary","capability":"code","workspace":workspace,"baseline":{},"prompt":"Fixture","references":[]});
    let launch = launch::prepare(
        &Files::new(root.clone()),
        &project,
        &snapshot,
        &task,
        &resources,
        &tools,
        BTreeMap::new(),
    )?;
    assert_eq!(launch.model, "host-code");
    assert_eq!(launch.secrets, ["host-key", "host-image-key"]);
    let command = launch.command.as_ref().unwrap().as_std();
    let environment: BTreeMap<_, _> = command.get_envs().collect();
    assert_eq!(
        environment[OsStr::new("BEAVER_CODEX_KEY")],
        Some(OsStr::new("host-key"))
    );
    let media: Value = serde_json::from_str(
        environment[OsStr::new("BEAVER_MEDIA_PROVIDERS")]
            .unwrap()
            .to_str()
            .unwrap(),
    )?;
    assert_eq!(media["image"]["model"], "host-image");
    assert_eq!(media["image"]["key"], "host-image-key");
    let config = fs::read_to_string(root.join("codex/host-boundary/config.toml"))?;
    assert!(config.contains("host-code"));
    assert!(
        !config.contains("host-key")
            && !config.contains("host-image-key")
            && !config.contains("later-model")
    );
    assert!(snapshot.require_capability("image").is_err());
    Ok(())
}

#[test]
fn validation_only_needs_no_provider_decryption_or_codex() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(temp.path())?;
    let mut settings = configured();
    settings["local"]["code"]["model"] = json!("");
    host.put("settings", "main", &settings)?;
    host.put("secret", "image", &"broken-cipher")?;
    let snapshot = ExecutionSettings::read(&host, &TestVault, Value::Null, None)?;
    assert!(snapshot.provider().is_err());
    assert!(snapshot.secrets().is_empty());
    let resources = Resources {
        skills: temp.path().join("missing-skills"),
        media: temp.path().join("missing-exe"),
        catalog: Value::Null,
        ask_user_tool: Value::Null,
    };
    for flag in ["validationOnly", "integrationValidation"] {
        let launch = launch::prepare(
            &Files::new(temp.path().to_owned()),
            &host,
            &snapshot,
            &json!({flag:true}),
            &resources,
            &BTreeMap::new(),
            BTreeMap::new(),
        )?;
        assert!(launch.command.is_none());
        assert!(launch.secrets.is_empty());
    }
    assert!(ExecutionSettings::read(&host, &TestVault, Value::Null, Some("code")).is_err());
    Ok(())
}

#[test]
fn required_provider_failure_is_fatal_but_optional_media_is_unavailable() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(temp.path())?;
    host.put("settings", "main", &configured())?;
    host.put("secret", "code", &"fixture:shared-key")?;
    host.put("secret", "review", &"fixture:shared-key")?;
    host.put("secret", "image", &"broken-cipher")?;
    let snapshot = ExecutionSettings::read(&host, &TestVault, Value::Null, Some("code"))?;
    assert!(snapshot.media_providers()["image"].is_null());
    assert_eq!(snapshot.secrets(), ["shared-key"]);
    assert!(ExecutionSettings::read(&host, &TestVault, Value::Null, Some("image")).is_err());
    assert!(ExecutionSettings::read(&host, &TestVault, Value::Null, Some("unknown")).is_err());
    Ok(())
}

#[test]
fn host_parallel_limit_preserves_defaults_and_bounds() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(temp.path())?;
    assert_eq!(parallel_limit(&host)?, 2);
    for (input, expected) in [(0, 1), (4, 4), (7, 6)] {
        host.put("settings", "main", &json!({"maxParallel":input}))?;
        assert_eq!(parallel_limit(&host)?, expected);
    }
    Ok(())
}
