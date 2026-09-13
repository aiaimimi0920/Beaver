use anyhow::{ensure, Context, Result};
use beaver_core::{
    legacy_vault::{migrate, LegacyVault},
    preferences::{key, SystemVault},
    store::Store,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("fixture directory required")?,
    );
    let fixture: Value = serde_json::from_slice(&fs::read(root.join("legacy.json"))?)?;
    ensure!(fixture["fixture"] == true, "not a test fixture");
    let cipher = fixture["cipher"].as_str().context("missing cipher")?;
    let state = root.join("electron-profile/Local State");
    let original_state = fs::read(&state)?;
    let vault = LegacyVault::open(&state)?;
    ensure!(
        vault.decrypt(cipher)? == "dummy-legacy-vault-凭据",
        "legacy value mismatch"
    );
    let data = root.join(format!("native-{}", uuid::Uuid::new_v4()));
    let mut store = Store::open(&data)?;
    store.put("secret", "code", &cipher)?;
    ensure!(migrate(&mut store, &state)? == 1);
    ensure!(key(&store, &SystemVault, "code")? == "dummy-legacy-vault-凭据");
    ensure!(store.list::<Value>("secret_backup")?[0]["cipher"] == cipher);
    ensure!(migrate(&mut store, &state)? == 0);
    let needle = "dummy-legacy-vault-凭据".as_bytes();
    for entry in fs::read_dir(&data)? {
        let path = entry?.path();
        if path.is_file() {
            let bytes = fs::read(path)?;
            ensure!(
                !bytes.windows(needle.len()).any(|part| part == needle),
                "plaintext persisted"
            );
        }
    }
    ensure!(
        fs::read(&state)? == original_state,
        "source profile modified"
    );
    let proof = json!({"checks":["real Electron 44 synchronous safeStorage decrypted in Rust", "converted to native DPAPI without plaintext persistence", "original cipher retained", "repeat conversion is no-op", "source Local State unchanged"], "passed":true});
    fs::write(root.join("proof.json"), serde_json::to_vec_pretty(&proof)?)?;
    println!("{proof}");
    Ok(())
}
