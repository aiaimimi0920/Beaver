use crate::project_runtime_lifecycle;
use beaver_core::{
    execution_settings::{self, ExecutionSettings},
    files::{safe_path, Files},
    launch::{self, Resources},
    object_attempt_launch, preferences,
    project_storage_router::ProjectStorageRouter,
    scheduler::{RuntimeFactory, Scheduler},
    scheduler_runtime::{RuntimeSource, TaskRuntime},
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
    host: Arc<Mutex<Store>>,
    store: Arc<Mutex<Store>>,
    project_storage: Arc<ProjectStorageRouter>,
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
    let factory_host = host.clone();
    let object_host = host.clone();
    let limit_host = host.clone();
    let files = Arc::new(Files::new(root.to_path_buf()));
    let source_store = store.clone();
    let source_files = files.clone();
    let source_router = project_storage.clone();
    let runtimes: RuntimeSource = Arc::new(move || {
        let mut runtimes = project_runtime_lifecycle::scheduler_runtimes(&source_router)
            .map_err(|error| error.to_string())?;
        runtimes.push(TaskRuntime::host(
            source_store.clone(),
            source_files.clone(),
        ));
        Ok(runtimes)
    });
    let factory: RuntimeFactory = Arc::new(move |task, runtime| {
        (|| -> anyhow::Result<_> {
            let runtime_store = runtime.store();
            let runtime_files = runtime.files();
            let validation_only = beaver_core::validation::task_gate::validation_only(task);
            let capability = if validation_only {
                None
            } else {
                Some(
                    task["capability"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("任务能力无效"))?,
                )
            };
            let mut settings = {
                let host = factory_host
                    .lock()
                    .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
                ExecutionSettings::read(
                    &host,
                    &preferences::SystemVault,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))?,
                    capability,
                )?
            };
            let workspace = runtime_files.resolve_workspace(
                task["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid task ID"))?,
                Path::new(
                    task["workspace"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Invalid workspace"))?,
                ),
            )?;
            if let Some(runtime) = beaver_core::workflows::runtime(&workspace)? {
                settings.tools["godot"] = runtime["godot"].clone();
                settings.mcp["godot"] = serde_json::json!(true);
                settings.mcp["blender"] = serde_json::json!(task["decompose"] != true);
            }
            let mut resolved = BTreeMap::new();
            let required: &[&str] = if validation_only {
                &["godot"]
            } else {
                &["codex", "godot", "blender", "node"]
            };
            for &name in required {
                match tools::find(name, settings.tools[name].as_str().unwrap_or("")) {
                    Ok(path) => {
                        resolved.insert(name.to_owned(), path.to_string_lossy().into_owned());
                    }
                    Err(error) if name == "codex" => return Err(error),
                    Err(_) => {
                        resolved.insert(name.to_owned(), "not installed".into());
                    }
                }
            }
            let store = runtime_store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            launch::prepare(
                &runtime_files,
                &store,
                &settings,
                task,
                &resources,
                &resolved,
                std::env::vars_os().collect(),
            )
        })()
        .map_err(|error| error.to_string())
    });
    let object_factory: object_attempt_launch::Factory = Arc::new(move |attempt, runtime| {
        (|| -> anyhow::Result<_> {
            let settings = {
                let host = object_host
                    .lock()
                    .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
                ExecutionSettings::read(
                    &host,
                    &preferences::SystemVault,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))?,
                    Some("code"),
                )?
            };
            let codex = tools::find("codex", settings.tools["codex"].as_str().unwrap_or(""))?;
            object_attempt_launch::prepare(
                runtime,
                attempt,
                &settings,
                &codex,
                std::env::vars_os().collect(),
            )
        })()
        .map_err(|error| error.to_string())
    });
    Ok(Scheduler::start_with_objects(
        runtimes,
        factory,
        object_factory,
        Arc::new(move || {
            let _ = app.emit("beaver:changed", ());
        }),
        Arc::new(move || {
            let host = limit_host
                .lock()
                .map_err(|_| "宿主数据库锁不可用".to_string())?;
            execution_settings::parallel_limit(&host).map_err(|error| error.to_string())
        }),
    ))
}
