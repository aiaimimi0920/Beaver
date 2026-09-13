use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CAPABILITIES: [&str; 6] = ["code", "review", "image", "speech", "music", "translation"];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Endpoint {
    base_url: String,
    model: String,
    route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    has_key: Option<bool>,
}

#[derive(Deserialize, Serialize)]
struct CapabilitySet<T> {
    code: T,
    review: T,
    image: T,
    speech: T,
    music: T,
    translation: T,
}

#[derive(Deserialize, Serialize)]
struct Tools {
    codex: String,
    godot: String,
    blender: String,
    node: String,
}

#[derive(Deserialize, Serialize)]
struct Mcp {
    godot: bool,
    blender: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    #[serde(default = "default_ask_ratio")]
    ask_ratio: u8,
    mode: String,
    local: CapabilitySet<Endpoint>,
    cloud: Endpoint,
    cloud_models: CapabilitySet<String>,
    tools: Tools,
    mcp: Mcp,
    max_parallel: u8,
}

fn default_ask_ratio() -> u8 {
    100
}

pub trait Vault {
    fn encrypt(&self, text: &str) -> Result<String>;
    fn decrypt(&self, cipher: &str) -> Result<String>;
}

fn slot_valid(slot: &str) -> Result<()> {
    if slot != "cloud" && !CAPABILITIES.contains(&slot) {
        bail!("未知凭据类型");
    }
    Ok(())
}

pub fn valid_base(value: &str) -> Result<String> {
    let url = url::Url::parse(value).map_err(|_| anyhow::anyhow!("API 地址无效"))?;
    if !["http", "https"].contains(&url.scheme())
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("API 地址必须是无凭据、无查询参数的 HTTP(S) URL");
    }
    if url.scheme() == "http"
        && !["localhost", "127.0.0.1", "[::1]"].contains(&url.host_str().unwrap_or(""))
    {
        bail!("非本机 API 必须使用 HTTPS，避免密钥明文传输");
    }
    Ok(value.trim_end_matches('/').into())
}

pub fn validate(value: Value) -> Result<Value> {
    // Do not expose deserialization errors containing submitted secrets or URLs.
    let settings: Settings =
        serde_json::from_value(value).map_err(|_| anyhow::anyhow!("设置格式无效"))?;
    if !["local", "cloud"].contains(&settings.mode.as_str())
        || !(1..=6).contains(&settings.max_parallel)
        || ![0, 10, 30, 70, 100].contains(&settings.ask_ratio)
    {
        bail!("服务模式或并行数量无效");
    }
    for ep in [
        &settings.local.code,
        &settings.local.review,
        &settings.local.image,
        &settings.local.speech,
        &settings.local.music,
        &settings.local.translation,
        &settings.cloud,
    ] {
        if ep.base_url.encode_utf16().count() > 2000
            || ep.model.encode_utf16().count() > 200
            || ep.route.encode_utf16().count() > 200
        {
            bail!("服务配置过长");
        }
        if !ep.base_url.is_empty() {
            valid_base(&ep.base_url)?;
        }
    }
    Ok(serde_json::to_value(settings)?)
}

pub fn read(store: &Store, defaults: Value) -> Result<Value> {
    let mut settings: Value = store.get("settings", "main")?.unwrap_or(defaults);
    if settings["askRatio"].is_null() {
        settings["askRatio"] = json!(100);
    }
    for slot in CAPABILITIES {
        settings["local"][slot]["hasKey"] = json!(has_key(store, slot)?);
    }
    settings["cloud"]["hasKey"] = json!(has_key(store, "cloud")?);
    Ok(settings)
}

fn has_key(store: &Store, slot: &str) -> Result<bool> {
    Ok(store
        .get::<String>("secret", slot)?
        .is_some_and(|s| !s.is_empty()))
}

pub fn save(store: &mut Store, vault: &impl Vault, value: Value, keys: Value) -> Result<Value> {
    let settings = validate(value)?;
    let keys: BTreeMap<String, String> =
        serde_json::from_value(keys).map_err(|_| anyhow::anyhow!("凭据格式无效"))?;
    let mut encrypted = Vec::new();
    for (slot, key) in keys {
        slot_valid(&slot)?;
        if key.encode_utf16().count() > 10000 {
            bail!("凭据过长");
        }
        if !key.is_empty() {
            encrypted.push((slot, vault.encrypt(&key)?));
        }
    }
    store.transaction(|db| {
        for (slot, cipher) in encrypted {
            db.execute("INSERT INTO entities(kind,id,value) VALUES('secret',?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
                rusqlite::params![slot, serde_json::to_string(&cipher)?])?;
        }
        db.execute("INSERT INTO entities(kind,id,value) VALUES('settings','main',?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
            [serde_json::to_string(&settings)?])?;
        Ok(())
    })?;
    read(store, settings)
}

pub fn clear_key(store: &Store, slot: &str) -> Result<()> {
    slot_valid(slot)?;
    store.put("secret", slot, &"")
}

pub fn key(store: &Store, vault: &impl Vault, slot: &str) -> Result<String> {
    slot_valid(slot)?;
    match store.get::<String>("secret", slot)? {
        Some(cipher) if !cipher.is_empty() => vault.decrypt(&cipher),
        _ => Ok(String::new()),
    }
}

pub struct SystemVault;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub base_url: String,
    pub model: String,
    pub route: String,
    pub key: String,
}

pub fn resolve(
    store: &Store,
    vault: &impl Vault,
    settings: &Value,
    capability: &str,
) -> Result<Provider> {
    if !CAPABILITIES.contains(&capability) {
        bail!("未知 AI 能力");
    }
    let cloud = settings["mode"] == "cloud";
    let endpoint = if cloud {
        &settings["cloud"]
    } else {
        &settings["local"][capability]
    };
    let model = if cloud {
        &settings["cloudModels"][capability]
    } else {
        &endpoint["model"]
    };
    let base = endpoint["baseUrl"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("请先配置服务地址")?;
    let model = model
        .as_str()
        .filter(|s| !s.is_empty())
        .context("请先配置服务模型")?;
    Ok(Provider {
        base_url: valid_base(base)?,
        model: model.into(),
        route: if cloud {
            settings["local"][capability]["route"].as_str()
        } else {
            endpoint["route"].as_str()
        }
        .unwrap_or("")
        .into(),
        key: key(store, vault, if cloud { "cloud" } else { capability })?,
    })
}

impl Vault for SystemVault {
    fn encrypt(&self, text: &str) -> Result<String> {
        use base64::Engine;
        Ok(format!(
            "beaver-dpapi-v1:{}",
            base64::engine::general_purpose::STANDARD.encode(protect(text.as_bytes(), false)?)
        ))
    }
    fn decrypt(&self, cipher: &str) -> Result<String> {
        use base64::Engine;
        let payload = cipher
            .strip_prefix("beaver-dpapi-v1:")
            .context("旧凭据需要迁移，已保留原密文；请勿清除后重试")?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|_| anyhow::anyhow!("凭据密文格式无效"))?;
        String::from_utf8(protect(&bytes, true)?).map_err(|_| anyhow::anyhow!("凭据解密内容无效"))
    }
}

#[cfg(windows)]
pub(crate) fn protect(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len().try_into()?,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI owns output; copy and release it, and never fall back to plaintext.
    unsafe {
        let success = if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if success == 0 {
            bail!("系统凭据保护失败，原凭据未修改");
        }
        let result = if output.cbData == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
        };
        LocalFree(output.pbData.cast());
        Ok(result)
    }
}

#[cfg(not(windows))]
pub(crate) fn protect(_: &[u8], _: bool) -> Result<Vec<u8>> {
    bail!("此平台的系统凭据存储尚未接入，拒绝明文保存")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn defaults() -> Value {
        serde_json::from_str(include_str!("../../../dist-native/default-settings.json")).unwrap()
    }
    struct TestVault;
    impl Vault for TestVault {
        fn encrypt(&self, text: &str) -> Result<String> {
            if text == "fail" {
                bail!("test encryption failure");
            }
            Ok(format!("test:{text}"))
        }
        fn decrypt(&self, cipher: &str) -> Result<String> {
            Ok(cipher.trim_start_matches("test:").into())
        }
    }
    #[test]
    fn save_preserves_empty_keys_and_is_atomic() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        let settings = save(
            &mut store,
            &TestVault,
            defaults(),
            json!({"code":"fixture"}),
        )?;
        assert_eq!(settings["local"]["code"]["hasKey"], true);
        assert!(!settings.to_string().contains("fixture"));
        save(&mut store, &TestVault, defaults(), json!({"code":""}))?;
        assert_eq!(key(&store, &TestVault, "code")?, "fixture");
        let mut changed = defaults();
        changed["maxParallel"] = json!(6);
        assert!(save(
            &mut store,
            &TestVault,
            changed,
            json!({"code":"replacement", "review":"fail"})
        )
        .is_err());
        assert_eq!(key(&store, &TestVault, "code")?, "fixture");
        assert_ne!(read(&store, defaults())?["maxParallel"], 6);
        clear_key(&store, "code")?;
        assert_eq!(read(&store, defaults())?["local"]["code"]["hasKey"], false);
        Ok(())
    }
    #[test]
    fn settings_validate_without_echoing_sensitive_input() {
        for url in [
            "http://remote.example/v1",
            "https://u:secret@example.com",
            "https://example.com?key=secret",
            "file:///tmp",
        ] {
            assert!(valid_base(url).is_err());
        }
        for url in [
            "http://localhost:8080/v1",
            "http://127.0.0.1/v1",
            "http://[::1]/v1",
            "https://example.com/v1",
        ] {
            assert!(valid_base(url).is_ok());
        }
        let mut settings = defaults();
        settings["mode"] = json!("secret-value");
        assert!(!validate(settings)
            .unwrap_err()
            .to_string()
            .contains("secret-value"));
    }
    #[test]
    fn cloud_and_local_resolve_their_own_keys_and_models() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path())?;
        store.put("secret", "code", &"test:local-fixture")?;
        store.put("secret", "cloud", &"test:cloud-fixture")?;
        let mut settings = defaults();
        settings["local"]["code"] = json!({"baseUrl":"https://local.example/v1/","model":"local-model","route":"code-route"});
        settings["cloud"] =
            json!({"baseUrl":"https://platform.example/v1","model":"unused","route":"unused"});
        settings["cloudModels"]["code"] = json!("cloud-model");
        settings["mode"] = json!("local");
        let local = resolve(&store, &TestVault, &settings, "code")?;
        assert_eq!(local.key, "local-fixture");
        assert_eq!(local.base_url, "https://local.example/v1");
        assert_eq!(local.model, "local-model");
        settings["mode"] = json!("cloud");
        let cloud = resolve(&store, &TestVault, &settings, "code")?;
        assert_eq!(cloud.key, "cloud-fixture");
        assert_eq!(cloud.model, "cloud-model");
        assert_eq!(cloud.route, "code-route");
        assert!(resolve(&store, &TestVault, &settings, "unknown").is_err());
        Ok(())
    }
    #[cfg(windows)]
    #[test]
    fn system_vault_roundtrip_and_corruption_rejection() -> Result<()> {
        let vault = SystemVault;
        let cipher = vault.encrypt("dummy-native-凭据")?;
        assert!(!cipher.contains("dummy-native"));
        assert_eq!(vault.decrypt(&cipher)?, "dummy-native-凭据");
        assert_eq!(vault.decrypt(&vault.encrypt("")?)?, "");
        assert!(vault.decrypt("beaver-dpapi-v1:AAAA").is_err());
        assert!(vault.decrypt("legacy-opaque").is_err());
        Ok(())
    }
}
