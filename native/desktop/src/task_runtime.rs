use beaver_core::{
    files::{safe_path, Files},
    launch::{self, Resources},
    preferences,
    scheduler::{Factory, Scheduler},
    store::Store,
    tools,
};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::Emitter;

pub fn start(
    app: tauri::AppHandle,
    root: &Path,
    store: Arc<Mutex<Store>>,
) -> anyhow::Result<Scheduler> {
    let directory = safe_path(root, "native-resources/skills")?;
    fs::create_dir_all(&directory)?;
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
        let path = safe_path(&directory, relative)?;
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("Skill 路径无效"))?,
        )?;
        fs::write(path, content)?;
    }
    let executable = std::env::current_exe()?;
    let resources = Resources {
        skills: directory,
        media: executable,
        catalog: serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
        ask_user_tool: serde_json::from_str(include_str!(
            "../../../dist-native/ask-user-tool.json"
        ))?,
    };
    let factory_store = store.clone();
    let factory_root = root.to_path_buf();
    let factory: Factory = Arc::new(move |task| {
        (|| -> anyhow::Result<_> {
            let store = factory_store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            let mut settings = preferences::read(
                &store,
                serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
            )?;
            let workspace = Path::new(
                task["workspace"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid workspace"))?,
            );
            if let Some(runtime) = beaver_core::workflows::runtime(workspace)? {
                settings["tools"]["godot"] = runtime["godot"].clone();
                settings["mcp"]["godot"] = serde_json::json!(true);
                settings["mcp"]["blender"] = serde_json::json!(task["decompose"] != true);
            }
            drop(store);
            let mut resolved = BTreeMap::new();
            let required: &[&str] = if beaver_core::validation::task_gate::validation_only(task) {
                &["godot"]
            } else {
                &["codex", "godot", "blender", "node"]
            };
            for &name in required {
                match tools::find(name, settings["tools"][name].as_str().unwrap_or("")) {
                    Ok(path) => {
                        resolved.insert(name.to_owned(), path.to_string_lossy().into_owned());
                    }
                    Err(error) if name == "codex" => return Err(error),
                    Err(_) => {
                        resolved.insert(name.to_owned(), "not installed".into());
                    }
                }
            }
            let store = factory_store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            launch::prepare(
                &factory_root,
                &store,
                &preferences::SystemVault,
                &settings,
                task,
                &resources,
                &resolved,
                std::env::vars_os().collect(),
            )
        })()
        .map_err(|error| error.to_string())
    });
    let files = Arc::new(Files::new(root.to_path_buf()));
    Ok(Scheduler::start(
        store,
        files,
        factory,
        Arc::new(move || {
            let _ = app.emit("beaver:changed", ());
        }),
    ))
}
