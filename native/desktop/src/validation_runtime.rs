use crate::Backend;
use anyhow::{bail, ensure, Context, Result};
use beaver_core::{
    preferences,
    store::Store,
    tools,
    validation::{self, operations, repository},
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc, Mutex},
};
use tauri::Emitter;

fn ffmpeg(configured: &str) -> Option<PathBuf> {
    if !configured.trim().is_empty() {
        return Path::new(configured)
            .is_file()
            .then(|| PathBuf::from(configured));
    }
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

pub fn start(
    app: tauri::AppHandle,
    root: &Path,
    store: Arc<Mutex<Store>>,
    scheduler: beaver_core::scheduler::Scheduler,
) -> Result<validation::service::Service> {
    validation::service::Service::start(
        store,
        root.into(),
        Arc::new(|store, run| {
            let project = repository::project(store, &run.project_id)?;
            let preferences = preferences::read(
                store,
                serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
            )?;
            let configured = beaver_core::workflows::configured_engine(
                &project,
                preferences["tools"]["godot"].as_str().unwrap_or(""),
            )?;
            let settings = validation::settings::read(store, &run.project_id)?;
            Ok(validation::service::Tools {
                godot: tools::find("godot", &configured)?,
                ffmpeg: ffmpeg(&settings.ffmpeg),
            })
        }),
        Arc::new(move || {
            let _ = scheduler.wake();
            let _ = app.emit("beaver:changed", ());
        }),
    )
}

pub fn call(backend: &Backend, method: &str, input: Value, source: &str) -> Result<Value, String> {
    (|| -> Result<Value> {
        ensure!(
            !backend.closing.load(Ordering::SeqCst),
            "Application is closing"
        );
        if method == "validation.run.cancel" {
            return backend.validation.cancel(&input);
        }
        if is_query(method) {
            return query(backend, method, &input);
        }
        let _operation = backend
            .operations
            .lock()
            .map_err(|_| anyhow::anyhow!("Operation lock unavailable"))?;
        let mut store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        match method {
            "validation.flow.save" => operations::save_flow(&mut store, &input),
            "validation.flow.explore" => operations::explore(&mut store, &input),
            "validation.code.run"
            | "validation.flow.run"
            | "validation.run.all"
            | "validation.run.rerun" => {
                operations::enqueue(&mut store, &backend.root, method, &input)
            }
            "validation.evidence.confirm" => {
                let run = operations::owned_run(&store, &input)?;
                let ids: Vec<String> = serde_json::from_value(input["evidenceIds"].clone())?;
                let confirmed = validation::confirmation::confirm(
                    &backend.root,
                    &mut store,
                    &run.id,
                    operations::string(&input, "snapshotId")?,
                    &ids,
                    operations::string(&input, "requestId")?,
                    source,
                )?;
                operations::public_run(&confirmed)
            }
            "validation.feedback.create" => validation::feedback::create(
                &mut store,
                &backend.root,
                &input,
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
                source,
            ),
            "validation.settings.save" => validation::settings::save(&mut store, &input),
            "validation.release.start" => {
                validation::release::start(&mut store, &backend.root, &input)
            }
            _ => bail!("Unknown validation operation"),
        }
    })()
    .map_err(|error| error.to_string())
}

pub fn is_query(method: &str) -> bool {
    matches!(
        method,
        "validation.list"
            | "validation.flow.list"
            | "validation.run.get"
            | "validation.source"
            | "validation.release.get"
    )
}

fn query(backend: &Backend, method: &str, input: &Value) -> Result<Value> {
    let store = backend
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    let project = operations::string(input, "projectId")?;
    match method {
        "validation.list" | "validation.flow.list" => {
            let mut value = operations::list(&store, project)?;
            let releases = validation::release_display::list(&store, project)?;
            drop(store);
            value["releases"] = json!(releases
                .into_iter()
                .map(|r| r.finish(&backend.root))
                .collect::<Vec<_>>());
            Ok(value)
        }
        "validation.source" => operations::source(&store, &backend.root, input),
        "validation.release.get" => {
            let release: validation::model::Release = repository::get(
                &store,
                "validationRelease",
                operations::string(input, "releaseCheckId")?,
            )?;
            ensure!(
                release.project_id == project,
                "Release belongs to another project"
            );
            let prepared = validation::release_display::prepare(&store, &release)?;
            drop(store);
            Ok(prepared.finish(&backend.root))
        }
        "validation.run.get" => {
            let run = operations::owned_run(&store, input)?;
            let baseline = validation::comparison::baseline_run(&store, &run);
            drop(store);
            let mut result = operations::public_run(&run)?;
            result["sourcePaths"] = json!(run.snapshot.keys().collect::<Vec<_>>());
            match baseline {
                Ok(Some((record, previous))) => {
                    let integrity = validation::evidence::validate(&backend.root, &previous).err();
                    result["baseline"] = json!({"record":record,"run":operations::public_run(&previous)?,
                        "integrityError":integrity.map(|error| error.to_string())});
                }
                Err(error) => result["baselineError"] = json!(error.to_string()),
                Ok(None) => (),
            }
            if run.status == "completed" {
                let integrity = if run.kind == "visual" {
                    validation::evidence::validate(&backend.root, &run)
                } else {
                    validation::code::validate(&backend.root, &run)
                };
                if let Err(error) = integrity {
                    result["integrityError"] = json!(error.to_string());
                }
            }
            Ok(result)
        }
        _ => bail!("Unknown validation query"),
    }
}

pub fn media(backend: &Backend, uri: &str) -> Result<PathBuf> {
    let path = uri
        .strip_prefix("/validation/")
        .context("Invalid evidence URL")?;
    let (run_id, evidence_id) = path.split_once('/').context("Missing evidence ID")?;
    let run = {
        let store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        repository::get::<validation::model::Run>(&store, "validationRun", run_id)?
    };
    validation::evidence::media_path(&backend.root, &run, evidence_id)
}
