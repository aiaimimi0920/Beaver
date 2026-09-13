//! Explicit conversion of Electron Windows synchronous safeStorage credentials.
use crate::{
    preferences::{protect, SystemVault, Vault, CAPABILITIES},
    store::Store,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{fs::File, io::Read, path::Path};

pub struct LegacyVault(Aes256Gcm);

impl LegacyVault {
    pub fn open(local_state: &Path) -> Result<Self> {
        let file = File::open(local_state).context("无法读取旧版 Local State，原凭据未修改")?;
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 4 * 1024 * 1024 {
            bail!("旧版 Local State 过大");
        }
        let value: Value = serde_json::from_slice(&bytes).context("旧版 Local State 格式无效")?;
        let encoded = value["os_crypt"]["encrypted_key"]
            .as_str()
            .context("旧版加密主密钥缺失")?;
        let protected = STANDARD.decode(encoded).context("旧版加密主密钥格式无效")?;
        let protected = protected
            .strip_prefix(b"DPAPI")
            .context("不支持此旧版凭据主密钥格式")?;
        let mut key = protect(protected, true)?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow::anyhow!("旧版主密钥长度无效"));
        key.fill(0);
        Ok(Self(cipher?))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String> {
        if encoded.len() > 100000 {
            bail!("旧版凭据过长");
        }
        let bytes = STANDARD.decode(encoded).context("旧版凭据编码无效")?;
        if !bytes.starts_with(b"v10") || bytes.len() < 3 + 12 + 16 {
            bail!("不支持此旧版凭据格式，原密文保留");
        }
        let text = self
            .0
            .decrypt(Nonce::from_slice(&bytes[3..15]), &bytes[15..])
            .map_err(|_| {
                anyhow::anyhow!("旧版凭据认证失败，请选择对应的 Local State；原密文保留")
            })?;
        String::from_utf8(text).context("旧版凭据内容不是 UTF-8")
    }
}

/// Caller owns the data directory. Originals and replacements commit together.
/// This does not replace the full database/profile backup required by import.
pub fn migrate(store: &mut Store, local_state: &Path) -> Result<usize> {
    let prepared = prepare(store, local_state)?;
    store.transaction(|db| prepared.apply(db))
}

pub(crate) struct Prepared(Vec<(&'static str, String, String)>);

pub(crate) fn prepare(store: &Store, local_state: &Path) -> Result<Prepared> {
    let mut pending = Vec::new();
    for slot in CAPABILITIES.into_iter().chain(["cloud"]) {
        if let Some(cipher) = store.get::<String>("secret", slot)? {
            if cipher.starts_with("beaver-dpapi-v1:") {
                // A copied native credential may belong to another Windows user.
                SystemVault.decrypt(&cipher)?;
            } else if !cipher.is_empty() {
                pending.push((slot, cipher));
            }
        }
    }
    if pending.is_empty() {
        return Ok(Prepared(Vec::new()));
    }
    let legacy = LegacyVault::open(local_state)?;
    let mut converted = Vec::new();
    for (slot, original) in pending {
        let plain = legacy.decrypt(&original)?;
        let replacement = SystemVault.encrypt(&plain)?;
        converted.push((slot, original, replacement));
    }
    Ok(Prepared(converted))
}

impl Prepared {
    pub(crate) fn apply(self, db: &rusqlite::Connection) -> Result<usize> {
        let count = self.0.len();
        let batch = uuid::Uuid::new_v4().to_string();
        for (slot, original, replacement) in self.0 {
            let before = serde_json::to_string(&original)?;
            let backup = json!({"slot":slot,"cipher":original,"batch":batch,"format":"electron-windows-v10"});
            db.execute(
                "INSERT INTO entities(kind,id,value) VALUES('secret_backup',?,?)",
                rusqlite::params![format!("{batch}:{slot}"), backup.to_string()],
            )?;
            let changed = db.execute(
                "UPDATE entities SET value=? WHERE kind='secret' AND id=? AND value=?",
                rusqlite::params![serde_json::to_string(&replacement)?, slot, before],
            )?;
            if changed != 1 {
                bail!("迁移期间凭据发生变化，已取消全部转换");
            }
        }
        Ok(count)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::preferences::key;
    #[test]
    fn conversion_is_atomic_idempotent_and_keeps_originals() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(&temp.path().join("data"))?;
        let state = temp.path().join("Local State");
        let key_bytes = [42u8; 32];
        let encrypted_key = [b"DPAPI".as_slice(), protect(&key_bytes, false)?.as_slice()].concat();
        std::fs::write(
            &state,
            json!({"os_crypt":{"encrypted_key":STANDARD.encode(encrypted_key)}}).to_string(),
        )?;
        let aes = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce = [7u8; 12];
        let body = aes
            .encrypt(Nonce::from_slice(&nonce), b"dummy-legacy".as_slice())
            .unwrap();
        let old = STANDARD.encode([b"v10".as_slice(), &nonce, &body].concat());
        let vault = LegacyVault::open(&state)?;
        let mut corrupted = STANDARD.decode(&old)?;
        *corrupted.last_mut().unwrap() ^= 1;
        assert!(vault.decrypt(&STANDARD.encode(corrupted)).is_err());
        let wrong = LegacyVault(Aes256Gcm::new_from_slice(&[43u8; 32]).unwrap());
        assert!(wrong.decrypt(&old).is_err());
        store.put("secret", "code", &old)?;
        store.put("secret", "review", &"invalid")?;
        assert!(migrate(&mut store, &state).is_err());
        assert_eq!(store.get::<String>("secret", "code")?.unwrap(), old);
        assert!(store.list::<Value>("secret_backup")?.is_empty());
        store.remove("secret", "review")?;
        assert_eq!(migrate(&mut store, &state)?, 1);
        assert_eq!(key(&store, &SystemVault, "code")?, "dummy-legacy");
        assert_eq!(store.list::<Value>("secret_backup")?[0]["cipher"], old);
        assert_eq!(migrate(&mut store, &state)?, 0);
        Ok(())
    }
}
