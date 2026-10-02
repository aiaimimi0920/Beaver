use anyhow::Result;
use beaver_core::{files::safe_path, launch::Resources};
use serde_json::Value;
use std::{fs, path::Path};

pub fn defaults() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../dist-native/default-settings.json"
    ))?)
}

pub fn designs() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../dist-native/design-catalog.json"
    ))?)
}

pub fn blueprints() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../dist-native/blueprint-catalog.json"
    ))?)
}

pub fn templates() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../dist-native/templates.json"
    ))?)
}

pub fn prepare(root: &Path) -> Result<Resources> {
    let skills = safe_path(root, "native-resources/skills")?;
    fs::create_dir_all(&skills)?;
    for (relative, content) in [
        (
            "godot-production/SKILL.md",
            include_str!("../../../resources/skills/godot-production/SKILL.md"),
        ),
        (
            "blender-production/SKILL.md",
            include_str!("../../../resources/skills/blender-production/SKILL.md"),
        ),
        (
            "blender-production/references/anime-npr-character.md",
            include_str!(
                "../../../resources/skills/blender-production/references/anime-npr-character.md"
            ),
        ),
        (
            "beaver-workflows/SKILL.md",
            include_str!("../../../resources/skills/beaver-workflows/SKILL.md"),
        ),
    ] {
        let path = safe_path(&skills, relative)?;
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid skill path"))?,
        )?;
        fs::write(path, content)?;
    }
    Ok(Resources {
        skills,
        media: std::env::current_exe()?,
        catalog: blueprints()?,
        ask_user_tool: serde_json::from_str(include_str!(
            "../../../dist-native/ask-user-tool.json"
        ))?,
    })
}
