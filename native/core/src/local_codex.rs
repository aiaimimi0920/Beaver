use crate::{preferences, store::Store};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

// Deliberately neither Debug nor Serialize: source credentials stay in memory.
pub struct Defaults {
    pub base_url: String,
    pub model: String,
    pub key: String,
}

pub fn read(env: &BTreeMap<String, String>, user_home: &Path) -> Result<Defaults> {
    let home = env
        .get("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| user_home.join(".codex"));
    let text = std::fs::read_to_string(home.join("config.toml"))
        .map_err(|_| anyhow::anyhow!("无法读取本机 Codex 配置，请手动填写 API 设置"))?;
    let mut config: toml::Table = toml::from_str(&text)
        .map_err(|_| anyhow::anyhow!("无法读取本机 Codex 配置，请手动填写 API 设置"))?;
    let profile = env
        .get("CODEX_PROFILE")
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .or_else(|| config.get("profile").and_then(toml::Value::as_str));
    if let Some(name) = profile.filter(|name| !name.is_empty()) {
        let selected = config
            .get("profiles")
            .and_then(|profiles| profiles.get(name))
            .and_then(toml::Value::as_table)
            .filter(|table| !table.is_empty())
            .context("本机 Codex profile 不存在")?
            .clone();
        config.extend(selected);
    }
    let id = config
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .unwrap_or("openai");
    let provider = config.get("model_providers").and_then(|all| all.get(id));
    let field = |key: &str| {
        provider
            .and_then(|provider| provider.get(key))
            .and_then(toml::Value::as_str)
    };
    if field("wire_api").is_some_and(|wire| !wire.is_empty() && wire != "responses") {
        bail!("本机 Codex 服务不是 Responses API，请手动配置兼容服务");
    }
    let base_url = preferences::valid_base(field("base_url").unwrap_or_else(|| {
        if id == "openai" {
            env.get("OPENAI_BASE_URL")
                .filter(|value| !value.is_empty())
                .map(String::as_str)
                .unwrap_or("https://api.openai.com/v1")
        } else {
            ""
        }
    }))?;
    let model = config
        .get("model")
        .and_then(toml::Value::as_str)
        .filter(|model| !model.trim().is_empty())
        .context("本机 Codex 未指定模型，请手动选择模型")?
        .to_owned();
    let key = if let Some(name) = field("env_key") {
        env.get(name)
            .filter(|value| !value.is_empty())
            .context("本机 Codex 指定的 API Key 环境变量不可用")?
            .clone()
    } else if let Some(token) = field("experimental_bearer_token") {
        token.to_owned()
    } else if let Some(key) = env.get("OPENAI_API_KEY").filter(|key| !key.is_empty()) {
        key.clone()
    } else {
        // Import only an explicit API key, never OAuth tokens or credential commands.
        std::fs::read(home.join("auth.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|auth| auth["OPENAI_API_KEY"].as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    if key.trim().is_empty() {
        bail!("未找到可导入的 API Key；不导入 ChatGPT 登录令牌或执行凭据命令");
    }
    Ok(Defaults {
        base_url,
        model,
        key,
    })
}

pub fn import(
    store: &mut Store,
    vault: &impl preferences::Vault,
    source: Defaults,
    value: Value,
    keys: Value,
) -> Result<Value> {
    let mut settings = preferences::validate(value)?;
    let base = preferences::valid_base(&source.base_url)?;
    let mut keys: BTreeMap<String, String> =
        serde_json::from_value(keys).map_err(|_| anyhow::anyhow!("凭据格式无效"))?;
    let mut filled = Vec::new();
    for cap in ["code", "review", "translation"] {
        let ep = &mut settings["local"][cap];
        let has_key = keys.get(cap).is_some_and(|key| !key.is_empty())
            || store
                .get::<String>("secret", cap)?
                .is_some_and(|key| !key.is_empty());
        let endpoint = ep["baseUrl"].as_str().unwrap_or("");
        let model = ep["model"].as_str().unwrap_or("");
        if !endpoint.is_empty() && preferences::valid_base(endpoint)? != base {
            continue;
        }
        if endpoint.is_empty() && (!model.is_empty() || has_key) {
            continue;
        }
        if endpoint.is_empty() || model.is_empty() || !has_key {
            let need_base = endpoint.is_empty();
            let need_model = model.is_empty();
            if need_base {
                ep["baseUrl"] = json!(base);
            }
            if need_model {
                ep["model"] = json!(source.model);
            }
            if !has_key {
                keys.insert(cap.into(), source.key.clone());
            }
            filled.push(cap);
        }
    }
    let settings = preferences::save(store, vault, settings, serde_json::to_value(keys)?)?;
    Ok(json!({"settings":settings,"filled":filled}))
}

pub fn read_environment(env: &BTreeMap<String, String>) -> Result<Defaults> {
    // An explicit isolated CODEX_HOME does not require a normal desktop profile.
    if env.get("CODEX_HOME").is_some_and(|home| !home.is_empty()) {
        return read(env, Path::new(""));
    }
    let home = env
        .get(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|home| !home.is_empty())
        .context("无法定位用户目录")?;
    read(env, Path::new(home))
}

pub fn import_current(store: &mut Store, input: &Value) -> Result<Value> {
    if store
        .get::<Value>("toolSetup", "main")?
        .is_some_and(|setup| setup["status"] == "running")
    {
        bail!("工具安装期间不能导入设置");
    }
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    import(
        store,
        &preferences::SystemVault,
        read_environment(&env)?,
        input["settings"].clone(),
        input["keys"].clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use preferences::Vault;
    const CONFIG: &str = "model='base'\nmodel_provider='local'\n[profiles.game]\nmodel='game-model'\n[model_providers.local]\nbase_url='http://localhost:8317/v1/'\nwire_api='responses'\n";

    #[test]
    fn explicit_home_works_without_user_profile_and_missing_home_is_rejected() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(temp.path().join("config.toml"), CONFIG)?;
        let env = BTreeMap::from([
            (
                "CODEX_HOME".into(),
                temp.path().to_string_lossy().into_owned(),
            ),
            ("OPENAI_API_KEY".into(), "fixture-only".into()),
        ]);
        assert_eq!(read_environment(&env)?.key, "fixture-only");
        assert!(read_environment(&BTreeMap::new()).is_err());
        let mut empty = env.clone();
        empty.insert("CODEX_HOME".into(), "".into());
        assert!(read_environment(&empty).is_err());
        let home = temp.path().join("profile");
        std::fs::create_dir_all(home.join(".codex"))?;
        std::fs::write(home.join(".codex/config.toml"), CONFIG)?;
        empty.insert(
            if cfg!(windows) { "USERPROFILE" } else { "HOME" }.into(),
            home.to_string_lossy().into_owned(),
        );
        assert_eq!(read_environment(&empty)?.model, "base");
        Ok(())
    }

    #[test]
    fn sources_profiles_precedence_and_secret_safe_failures() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let mut env = BTreeMap::from([("CODEX_HOME".into(), root.to_string_lossy().into_owned())]);
        std::fs::write(root.join("config.toml"), CONFIG)?;
        std::fs::write(
            root.join("auth.json"),
            r#"{"OPENAI_API_KEY":"fixture-auth"}"#,
        )?;
        let source = read(&env, root)?;
        assert_eq!(source.base_url, "http://localhost:8317/v1");
        assert_eq!(source.model, "base");
        assert_eq!(source.key, "fixture-auth");
        env.insert("CODEX_PROFILE".into(), "game".into());
        env.insert("OPENAI_API_KEY".into(), "fixture-env".into());
        assert_eq!(read(&env, root)?.model, "game-model");
        assert_eq!(read(&env, root)?.key, "fixture-env");
        std::fs::write(
            root.join("config.toml"),
            format!("{CONFIG}experimental_bearer_token='fixture-token'\n"),
        )?;
        assert_eq!(read(&env, root)?.key, "fixture-token");
        std::fs::write(
            root.join("config.toml"),
            format!("{CONFIG}env_key='DECLARED'\nexperimental_bearer_token='must-not-fallback'\n"),
        )?;
        assert!(read(&env, root)
            .err()
            .unwrap()
            .to_string()
            .contains("环境变量不可用"));
        env.insert("DECLARED".into(), "fixture-declared".into());
        assert_eq!(read(&env, root)?.key, "fixture-declared");
        env.insert("CODEX_PROFILE".into(), "missing".into());
        assert!(read(&env, root).is_err());
        env.remove("CODEX_PROFILE");
        env.remove("OPENAI_API_KEY");
        std::fs::write(root.join("config.toml"), CONFIG)?;
        std::fs::write(
            root.join("auth.json"),
            r#"{"tokens":{"access_token":"never-copy"}}"#,
        )?;
        assert!(read(&env, root)
            .err()
            .unwrap()
            .to_string()
            .contains("未找到可导入"));
        std::fs::write(root.join("config.toml"), "secret='never-echo")?;
        assert!(!read(&env, root)
            .err()
            .unwrap()
            .to_string()
            .contains("never-echo"));
        std::fs::write(
            root.join("config.toml"),
            CONFIG.replace("responses", "chat"),
        )?;
        assert!(read(&env, root)
            .err()
            .unwrap()
            .to_string()
            .contains("Responses API"));
        Ok(())
    }

    struct TestVault;
    impl Vault for TestVault {
        fn encrypt(&self, text: &str) -> Result<String> {
            if text == "fail" {
                bail!("fixture encryption failure");
            }
            Ok(format!("encrypted:{text}"))
        }
        fn decrypt(&self, text: &str) -> Result<String> {
            Ok(text.trim_start_matches("encrypted:").into())
        }
    }
    fn source() -> Defaults {
        Defaults {
            base_url: "http://localhost:8317/v1".into(),
            model: "fixture-model".into(),
            key: "fixture-key".into(),
        }
    }
    fn defaults() -> Value {
        serde_json::from_str(include_str!("../../../dist-native/default-settings.json")).unwrap()
    }

    #[test]
    fn imports_only_compatible_blanks_preserves_drafts_and_is_atomic() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        let mut value = defaults();
        value["local"]["review"]["baseUrl"] = json!("https://review.example/v1");
        value["local"]["review"]["model"] = json!("independent");
        preferences::save(
            &mut store,
            &TestVault,
            value.clone(),
            json!({"review":"keep"}),
        )?;
        let result = import(&mut store, &TestVault, source(), value, json!({}))?;
        assert_eq!(result["filled"], json!(["code", "translation"]));
        assert!(!result.to_string().contains("fixture-key"));
        assert_eq!(preferences::key(&store, &TestVault, "code")?, "fixture-key");
        assert_eq!(preferences::key(&store, &TestVault, "review")?, "keep");
        assert_eq!(result["settings"]["local"]["image"]["baseUrl"], "");
        let again = import(
            &mut store,
            &TestVault,
            source(),
            result["settings"].clone(),
            json!({}),
        )?;
        assert_eq!(again["filled"], json!([]));
        let mut draft = again["settings"].clone();
        draft["maxParallel"] = json!(4);
        import(
            &mut store,
            &TestVault,
            source(),
            draft,
            json!({"code":"draft"}),
        )?;
        assert_eq!(preferences::key(&store, &TestVault, "code")?, "draft");
        let before = preferences::read(&store, defaults())?;
        assert_eq!(before["maxParallel"], 4);
        let mut changed = before.clone();
        changed["maxParallel"] = json!(5);
        assert!(import(
            &mut store,
            &TestVault,
            source(),
            changed,
            json!({"code":"replacement","review":"fail"})
        )
        .is_err());
        assert_eq!(preferences::read(&store, defaults())?, before);
        assert_eq!(preferences::key(&store, &TestVault, "code")?, "draft");
        // An incomplete endpoint with an existing key/model must not be rebound.
        let mut unbound = defaults();
        unbound["local"]["translation"]["model"] = json!("manual");
        let result = import(&mut store, &TestVault, source(), unbound, json!({}))?;
        assert_eq!(result["filled"], json!([]));
        assert_eq!(result["settings"]["local"]["code"]["baseUrl"], "");
        Ok(())
    }
}
