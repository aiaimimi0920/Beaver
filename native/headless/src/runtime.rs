use crate::resources;
use anyhow::{Context, Result};
use beaver_core::{
    execution_settings::{self, ExecutionSettings},
    external_run_contract,
    external_runs::ExternalRegistry,
    files::Files,
    journal::Journal,
    launch,
    preferences::SystemVault,
    project_runtime::ProjectRuntime,
    project_storage_router::ProjectStorageRouter,
    scheduler::{RuntimeFactory, Scheduler},
    scheduler_runtime::{RuntimeSource, TaskRuntime},
    store::Store,
    tools,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

fn recover(store: &mut Store, files: &Files) -> Result<()> {
    Journal::new(store, files).recover()?;
    store.recover_tasks()?;
    beaver_core::validation::repository::recover(store)
}

pub fn open(router: &ProjectStorageRouter, id: &str) -> Result<ProjectRuntime> {
    // The router calls recovery only for a newly opened runtime, never for live queues.
    router.open_registered_with(id, recover)
}

pub fn open_all(router: &ProjectStorageRouter) -> Result<()> {
    for id in router.registered_project_ids()? {
        anyhow::ensure!(
            router.registered_project_uses_local_storage(&id)?,
            "Headless requires project-local storage; migrate legacy project first: {id}"
        );
        open(router, &id)?;
    }
    Ok(())
}

pub fn task(router: &ProjectStorageRouter, id: &str) -> Result<ProjectRuntime> {
    if let Ok(runtime) = router.runtime_for_task(id) {
        return Ok(runtime);
    }
    // Scheduler-created children need indexing, but already-open runtimes need no recovery.
    for runtime in router.runtimes()? {
        router.open_registered(runtime.project_id())?;
    }
    router.runtime_for_task(id)
}

pub fn start(
    root: &Path,
    host: Arc<Mutex<Store>>,
    router: Arc<ProjectStorageRouter>,
    external: ExternalRegistry,
) -> Result<Scheduler> {
    let resources = resources::prepare(root)?;
    let source: RuntimeSource = Arc::new(move || {
        router
            .runtimes()
            .map(|runtimes| {
                runtimes
                    .into_iter()
                    .map(|runtime| {
                        let gate = router.work_gate(runtime.project_id());
                        TaskRuntime::from_project(runtime).with_work_gate(gate)
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    });
    let factory_host = host.clone();
    let factory: RuntimeFactory = Arc::new(move |task, runtime| {
        (|| -> Result<_> {
            let validation_only = beaver_core::validation::task_gate::validation_only(task);
            let external_mode = external_run_contract::enabled(task);
            let capability = if validation_only || external_mode {
                None
            } else {
                Some(
                    task["capability"]
                        .as_str()
                        .context("Invalid task capability")?,
                )
            };
            let mut settings = {
                let host = factory_host
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Host store lock unavailable"))?;
                ExecutionSettings::read(&host, &SystemVault, resources::defaults()?, capability)?
            };
            let files = runtime.files();
            let workspace = files.resolve_workspace(
                task["id"].as_str().context("Invalid task ID")?,
                Path::new(task["workspace"].as_str().context("Invalid workspace")?),
            )?;
            if let Some(config) = beaver_core::workflows::runtime(&workspace)? {
                settings.tools["godot"] = config["godot"].clone();
                settings.mcp["godot"] = json!(true);
                settings.mcp["blender"] = json!(beaver_core::task_workflows::blender_enabled(
                    task,
                    settings.mcp["blender"] == true
                ));
            }
            let mut resolved = BTreeMap::new();
            let required: &[&str] = if validation_only {
                &["godot"]
            } else if external_mode {
                &["godot", "blender"]
            } else {
                &["codex", "godot", "blender", "node"]
            };
            for &name in required {
                match tools::find(name, settings.tools[name].as_str().unwrap_or("")) {
                    Ok(path) => {
                        resolved.insert(name.into(), path.to_string_lossy().into_owned());
                    }
                    Err(error) if name == "codex" => return Err(error),
                    Err(_) => {
                        resolved.insert(name.into(), "not installed".into());
                    }
                }
            }
            let store_handle = runtime.store();
            let store = store_handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
            if external_mode && !validation_only {
                return launch::prepare_external(
                    &files,
                    &store,
                    task,
                    &resources,
                    &resolved,
                    external.clone(),
                );
            }
            launch::prepare(
                &files,
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
    Ok(Scheduler::start_with_runtimes(
        source,
        factory,
        Arc::new(|| {}),
        Arc::new(move || {
            let host = host
                .lock()
                .map_err(|_| "Host store lock unavailable".to_owned())?;
            execution_settings::parallel_limit(&host).map_err(|error| error.to_string())
        }),
    ))
}
