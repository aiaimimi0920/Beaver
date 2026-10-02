//! Freeze an explicit planner workflow choice without imposing NPR on other tasks.
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub fn selection(root: &Path, id: &str) -> Result<Value> {
    if id == "general" {
        return Ok(json!({"id":"general"}));
    }
    anyhow::ensure!(
        id == crate::workflows::WORKFLOW,
        "Unknown production workflow"
    );
    let available = crate::workflows::list(root)?;
    let descriptor = available["workflows"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["id"] == id && item["enabled"] == true)
        })
        .context("Selected production workflow is not installed or requires migration")?;
    let contract = crate::files::safe_path(root, "addons/npr_character_frame/asset_contract.json")?;
    let hash =
        crate::files::file_hash(&contract)?.context("Selected workflow contract is missing")?;
    Ok(json!({"id":id,"workflowVersion":descriptor["version"],
        "contractVersion":descriptor["contractVersion"],"pluginVersion":descriptor["pluginVersion"],
        "sourceCommit":descriptor["sourceCommit"],"contractSha256":hash}))
}

pub fn verify(root: &Path, task: &Value) -> Result<()> {
    if task["productionWorkflow"].is_null() {
        return Ok(()); // Persisted legacy tasks keep their original workflow policy.
    }
    let frozen = &task["productionWorkflow"];
    let id = frozen["id"]
        .as_str()
        .context("Invalid frozen production workflow")?;
    anyhow::ensure!(
        &selection(root, id)? == frozen,
        "Production workflow changed after planning; create a new reviewed plan"
    );
    Ok(())
}

/// Preserve configured Blender support on general tasks; force it only for NPR choices.
pub fn blender_enabled(task: &Value, configured: bool) -> bool {
    if task["decompose"] == true {
        return false;
    }
    if task["productionWorkflow"]["id"] == "general" {
        configured
    } else {
        true // Existing bound NPR tasks retain the legacy owned-session behavior.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn general_choice_needs_no_npr_package_or_forced_blender() -> Result<()> {
        let root = tempfile::tempdir()?;
        assert_eq!(selection(root.path(), "general")?, json!({"id":"general"}));
        verify(root.path(), &json!({"productionWorkflow":{"id":"general"}}))?;
        assert!(!blender_enabled(
            &json!({"productionWorkflow":{"id":"general"}}),
            false
        ));
        assert!(blender_enabled(
            &json!({"productionWorkflow":{"id":"general"}}),
            true
        ));
        assert!(blender_enabled(&json!({}), false));
        assert!(!blender_enabled(&json!({"decompose":true}), true));
        assert!(selection(root.path(), "npr-character").is_err());
        assert!(selection(root.path(), "unregistered").is_err());
        Ok(())
    }

    #[test]
    fn missing_package_or_changed_frozen_identity_cannot_launch() -> Result<()> {
        let root = tempfile::tempdir()?;
        verify(root.path(), &json!({}))?;
        assert!(verify(
            root.path(),
            &json!({"productionWorkflow":{"id":"npr-character"}})
        )
        .is_err());
        assert!(verify(
            root.path(),
            &json!({"productionWorkflow":{"id":"general","sourceCommit":"forged"}})
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn npr_choice_freezes_source_version_and_actual_contract_hash() -> Result<()> {
        let root = tempfile::tempdir()?;
        let package = root.path().join("addons/npr_character_frame");
        let bundle: Value =
            serde_json::from_str(include_str!("../../../dist-native/npr-package.json"))?;
        for (relative, encoded) in bundle.as_object().unwrap() {
            let relative = if relative == "provenance.json" {
                "addons/npr_character_frame/beaver-provenance.json"
            } else {
                relative.as_str()
            };
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(
                path,
                base64::engine::general_purpose::STANDARD.decode(encoded.as_str().unwrap())?,
            )?;
        }
        let manifest = json!({"schemaVersion":1,"packages":{"npr-characters":{
            "status":"ready","sourcePath":"addons/npr_character_frame",
            "sourceCommit":"a08b47a6cc229b6978afda26d74d13af690a67ba",
            "contractVersion":"1.1.0","pluginVersion":"1.3.0"}}});
        std::fs::write(
            root.path().join(crate::workflows::RUNTIME_FILE),
            manifest.to_string(),
        )?;
        let frozen = selection(root.path(), "npr-character")?;
        assert_eq!(frozen["contractVersion"], "1.1.0");
        assert_eq!(frozen["pluginVersion"], "1.3.0");
        let task = json!({"productionWorkflow":frozen});
        verify(root.path(), &task)?;
        std::fs::write(package.join("asset_contract.json"), "{\"fixture\":2}")?;
        assert!(verify(root.path(), &task).is_err());
        Ok(())
    }
}
