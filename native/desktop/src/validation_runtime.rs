use crate::{project_runtime_lifecycle, Backend};
use anyhow::{bail, ensure, Context, Result};
use beaver_core::{
    files::Files,
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

fn handles(
    backend: &Backend,
    project_id: &str,
) -> Result<crate::business_routing::TaskRuntimeHandles> {
    crate::business_routing::project_runtime_handles(
        &backend.project_storage,
        backend.store.clone(),
        &backend.root,
        project_id,
    )
    .map_err(anyhow::Error::msg)
}

fn refresh_open_task_routes(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
) -> Result<()> {
    let project_ids = router
        .runtimes()?
        .into_iter()
        .map(|runtime| runtime.project_id().to_owned())
        .collect::<Vec<_>>();
    for project_id in project_ids {
        project_runtime_lifecycle::open_registered(router, &project_id)?;
    }
    Ok(())
}

fn feedback_task_to_index(store: &Store, result: &Result<Value>) -> Result<Option<Value>> {
    let value = match result {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let task_id = value["taskId"]
        .as_str()
        .context("验证反馈成功结果缺少任务 ID")?;
    store
        .get::<Value>("task", task_id)?
        .with_context(|| format!("验证反馈任务不存在：{task_id}"))
        .map(Some)
}

fn ensure_host_media_fallback_allowed(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    project_id: &str,
) -> Result<()> {
    if router.runtime_for_project(project_id).is_ok() {
        bail!("项目本地存储已打开，禁止从宿主读取验证媒体：{project_id}");
    }
    let registered = router
        .registered_project_ids()?
        .into_iter()
        .any(|registered_id| registered_id == project_id);
    ensure!(
        registered,
        "项目未登记，禁止从宿主读取验证媒体：{project_id}"
    );
    ensure!(
        !router.registered_project_uses_local_storage(project_id)?,
        "项目本地存储未打开，禁止从宿主读取验证媒体：{project_id}"
    );
    Ok(())
}

pub fn start(
    app: tauri::AppHandle,
    root: &Path,
    host: Arc<Mutex<Store>>,
    store: Arc<Mutex<Store>>,
    project_storage: Arc<beaver_core::project_storage_router::ProjectStorageRouter>,
    scheduler: beaver_core::scheduler::Scheduler,
) -> Result<validation::service::Service> {
    let files = Arc::new(Files::new(root.into()));
    let fallback = validation::service::Storage {
        store: store.clone(),
        files: files.clone(),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    let storage = {
        let router = project_storage.clone();
        let fallback = fallback.clone();
        Arc::new(move |project_id: &str| {
            if let Ok(runtime) = router.runtime_for_project(project_id) {
                return Ok(validation::service::Storage {
                    store: runtime.store(),
                    files: runtime.files(),
                    project_id: Some(project_id.to_owned()),
                    draining: false,
                    work_gate: router.work_gate(project_id),
                });
            }
            if router.registered_project_uses_local_storage(project_id)? {
                let runtime = project_runtime_lifecycle::open_registered(&router, project_id)?;
                return Ok(validation::service::Storage {
                    store: runtime.store(),
                    files: runtime.files(),
                    project_id: Some(project_id.to_owned()),
                    draining: false,
                    work_gate: router.work_gate(project_id),
                });
            }
            ensure_host_media_fallback_allowed(&router, project_id)?;
            Ok(fallback.clone())
        })
    };
    let route_refresh = project_storage.clone();
    let enumerate = {
        let router = project_storage;
        let fallback = fallback.clone();
        Arc::new(move || {
            let runtimes = project_runtime_lifecycle::open_registered_local(&router)?;
            let registered = router.registered_project_ids()?;
            let mut storages = runtimes
                .into_iter()
                .map(|runtime| validation::service::Storage {
                    project_id: Some(runtime.project_id().to_owned()),
                    draining: !registered.iter().any(|id| id == runtime.project_id()),
                    work_gate: router.work_gate(runtime.project_id()),
                    store: runtime.store(),
                    files: runtime.files(),
                })
                .collect::<Vec<_>>();
            storages.push(fallback.clone());
            Ok(storages)
        })
    };
    validation::service::Service::start_with_routing(
        store.clone(),
        files,
        storage,
        enumerate,
        Arc::new(move |context| {
            let preferences = {
                let host = host
                    .lock()
                    .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
                preferences::read(
                    &host,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))?,
                )?
            };
            let configured = if context.engine == "godot" {
                beaver_core::workflows::configured_engine(
                    &context.project,
                    preferences["tools"]["godot"].as_str().unwrap_or(""),
                )?
            } else {
                preferences["tools"]["blender"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned()
            };
            Ok(validation::service::Tools {
                engine: tools::find(context.engine, &configured)?,
                ffmpeg: ffmpeg(&context.settings.ffmpeg),
            })
        }),
        Arc::new(move || {
            if let Err(error) = refresh_open_task_routes(&route_refresh) {
                eprintln!("Validation task routing: {error}");
            }
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
        if method.starts_with("validation.preview.") {
            return backend.validation.preview(method, &input);
        }
        if is_query(method) {
            return query(backend, method, &input);
        }
        let _operation = backend
            .operations
            .lock()
            .map_err(|_| anyhow::anyhow!("Operation lock unavailable"))?;
        let project_id = operations::string(&input, "projectId")?.to_owned();
        let handles = handles(backend, &project_id)?;
        let mut store = handles
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let files = handles.files;
        let result = match method {
            "validation.flow.save" => operations::save_flow(&mut store, &input),
            "validation.flow.explore" => operations::explore(&mut store, &input),
            "validation.code.run"
            | "validation.flow.run"
            | "validation.run.all"
            | "validation.run.rerun" => operations::enqueue(&mut store, &files, method, &input),
            "validation.evidence.confirm" => {
                let run = operations::owned_run(&store, &input)?;
                let ids: Vec<String> = serde_json::from_value(input["evidenceIds"].clone())?;
                let confirmed = validation::confirmation::confirm(
                    &files,
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
                &files,
                &input,
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
                source,
            ),
            "validation.settings.save" => validation::settings::save(&mut store, &input),
            "validation.release.start" => validation::release::start(&mut store, &files, &input),
            _ => bail!("Unknown validation operation"),
        };
        let task_to_index = if method == "validation.feedback.create" {
            feedback_task_to_index(&store, &result)?
        } else {
            None
        };
        drop(store);
        if let Some(task) = task_to_index {
            backend.project_storage.index_task(&task)?;
        }
        result
    })()
    .map_err(|error| error.to_string())
}

pub fn is_query(method: &str) -> bool {
    matches!(
        method,
        "validation.list"
            | "validation.flow.list"
            | "validation.run.get"
            | "validation.objectReport.get"
            | "validation.source"
            | "validation.release.get"
    )
}

pub(crate) fn object_report(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    input: &Value,
) -> Result<Value> {
    let request: validation::object_report::Request = serde_json::from_value(input.clone())?;
    let runtime = router.runtime_for_project(&request.project_id)?;
    Ok(serde_json::to_value(validation::object_report::get(
        &runtime, &request,
    )?)?)
}

fn query(backend: &Backend, method: &str, input: &Value) -> Result<Value> {
    if method == "validation.objectReport.get" {
        return object_report(&backend.project_storage, input);
    }
    let project = operations::string(input, "projectId")?;
    let handles = handles(backend, project)?;
    let files = handles.files;
    let store = handles
        .store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    match method {
        "validation.list" | "validation.flow.list" => {
            let mut value = operations::list(&store, project)?;
            let releases = validation::release_display::list(&store, project)?;
            drop(store);
            value["releases"] = json!(releases
                .into_iter()
                .map(|r| r.finish(&files))
                .collect::<Vec<_>>());
            Ok(value)
        }
        "validation.source" => operations::source(&store, &files, input),
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
            Ok(prepared.finish(&files))
        }
        "validation.run.get" => {
            let run = operations::owned_run(&store, input)?;
            let baseline = validation::comparison::baseline_run(&store, &run);
            drop(store);
            let mut result = operations::public_run(&run)?;
            result["sourcePaths"] = json!(run.snapshot.keys().collect::<Vec<_>>());
            match baseline {
                Ok(Some((record, previous))) => {
                    let integrity = validation::evidence::validate(&files, &previous).err();
                    result["baseline"] = json!({"record":record,"run":operations::public_run(&previous)?,
                        "integrityError":integrity.map(|error| error.to_string())});
                }
                Err(error) => result["baselineError"] = json!(error.to_string()),
                Ok(None) => (),
            }
            if run.status == "completed" {
                let integrity = if run.kind == "visual" {
                    validation::evidence::validate(&files, &run)
                } else {
                    validation::code::validate(&files, &run)
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
    let mut project_match = None;
    for runtime in backend.project_storage.runtimes()? {
        let store = runtime.store();
        let store = store
            .lock()
            .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
        if let Some(run) = store.get::<validation::model::Run>("validationRun", run_id)? {
            ensure!(
                run.project_id == runtime.project_id(),
                "Run belongs to another project"
            );
            ensure!(
                project_match.is_none(),
                "Validation run ID is duplicated across projects"
            );
            project_match = Some((run, runtime.files()));
        }
    }
    let (run, files) = if let Some(found) = project_match {
        found
    } else {
        let store = backend
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let run = repository::get::<validation::model::Run>(&store, "validationRun", run_id)?;
        ensure_host_media_fallback_allowed(&backend.project_storage, &run.project_id)?;
        (run, Arc::new(Files::new(backend.root.clone())))
    };
    validation::evidence::media_path(&files, &run, evidence_id)
}

#[cfg(test)]
#[path = "validation_runtime_tests.rs"]
mod tests;
