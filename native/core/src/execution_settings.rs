use crate::{
    preferences::{self, Provider, Vault},
    store::Store,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Host-owned snapshot. Credentials are resolved once and never read from a task store.
/// Intentionally not serializable or debuggable: providers contain plaintext keys.
pub struct ExecutionSettings {
    pub tools: Value,
    pub mcp: Value,
    capability: Option<String>,
    providers: BTreeMap<String, Provider>,
}

impl ExecutionSettings {
    /// None is used for validation-only work, which needs no AI provider or decryption.
    pub fn read(
        host: &Store,
        vault: &impl Vault,
        defaults: Value,
        capability: Option<&str>,
    ) -> Result<Self> {
        let settings = preferences::read(host, defaults)?;
        let mut providers = BTreeMap::new();
        if let Some(selected) = capability {
            providers.insert(
                selected.into(),
                preferences::resolve(host, vault, &settings, selected)?,
            );
            for name in preferences::CAPABILITIES {
                if name != selected {
                    if let Ok(provider) = preferences::resolve(host, vault, &settings, name) {
                        providers.insert(name.into(), provider);
                    }
                }
            }
        }
        Ok(Self {
            tools: settings["tools"].clone(),
            mcp: settings["mcp"].clone(),
            capability: capability.map(str::to_owned),
            providers,
        })
    }

    pub fn provider(&self) -> Result<&Provider> {
        self.capability
            .as_ref()
            .and_then(|name| self.providers.get(name))
            .context("AI execution settings were not resolved")
    }

    pub fn require_capability(&self, capability: &str) -> Result<()> {
        anyhow::ensure!(
            self.capability.as_deref() == Some(capability),
            "Execution capability changed after host settings were resolved"
        );
        Ok(())
    }

    pub fn media_providers(&self) -> Value {
        Value::Object(
            ["image", "speech", "music", "translation"]
                .into_iter()
                .map(|name| (name.into(), json!(self.providers.get(name))))
                .collect(),
        )
    }

    pub fn secrets(&self) -> Vec<String> {
        let mut secrets = Vec::new();
        for provider in self.providers.values() {
            if !provider.key.is_empty() && !secrets.contains(&provider.key) {
                secrets.push(provider.key.clone());
            }
        }
        secrets
    }
}

pub fn parallel_limit(host: &Store) -> Result<usize> {
    Ok(host
        .get::<Value>("settings", "main")?
        .and_then(|settings| settings["maxParallel"].as_u64())
        .unwrap_or(2)
        .clamp(1, 6) as usize)
}

#[cfg(test)]
#[path = "execution_settings_tests.rs"]
mod tests;
