use crate::{business_routing, instance};
use beaver_core::store::Store;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub(crate) struct Backend {
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) project_storage: Arc<beaver_core::project_storage_router::ProjectStorageRouter>,
    pub(crate) scheduler: beaver_core::scheduler::Scheduler,
    pub(crate) validation: beaver_core::validation::service::Service,
    pub(crate) planning: beaver_core::object_task_planning_service::Service,
    pub(crate) titles: beaver_core::object_task_title_service::Service,
    pub(crate) players: beaver_core::game_play::Players,
    pub(crate) root: PathBuf,
    pub(crate) closing: AtomicBool,
    pub(crate) api_calls: Arc<tokio::sync::Semaphore>,
    pub(crate) operations: Mutex<()>,
    pub(crate) setup: beaver_core::tool_setup::Setup,
    pub(crate) setup_gate: Mutex<()>,
    pub(crate) template_gate: Mutex<()>,
    pub(crate) template_cancel: Mutex<Option<Arc<AtomicBool>>>,
    pub(crate) captures: Mutex<beaver_core::capture::Capture>,
    pub(crate) _instance: instance::Instance,
}

fn replicate_project_call_log(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    source_store: Arc<Mutex<Store>>,
    call_id: &str,
    method: &str,
    result: &Result<Value, String>,
) {
    if !matches!(method, "project.create" | "project.import") || result.is_err() {
        return;
    }
    let Some(project_id) = result.as_ref().ok().and_then(|value| value["id"].as_str()) else {
        return;
    };
    let uses_local_storage = match router.registered_project_uses_local_storage(project_id) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Beaver call-log destination check failed for {call_id}: {error}");
            return;
        }
    };
    if !uses_local_storage {
        return;
    }
    let runtime = match crate::project_runtime_lifecycle::open_registered(router, project_id) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Beaver call-log destination open failed for {call_id}: {error}");
            return;
        }
    };
    let destination_store = runtime.store();
    if Arc::ptr_eq(&source_store, &destination_store) {
        return;
    }
    let source = match source_store.lock() {
        Ok(store) => store,
        Err(_) => {
            eprintln!("Beaver call-log source lock failed for {call_id}");
            return;
        }
    };
    let destination = match destination_store.lock() {
        Ok(store) => store,
        Err(_) => {
            eprintln!("Beaver call-log destination lock failed for {call_id}");
            return;
        }
    };
    if let Err(error) = beaver_core::call_log::copy_record(&source, &destination, call_id) {
        eprintln!("Beaver call-log replication failed for {call_id}: {error}");
    }
}

pub(crate) async fn business_call(
    app: tauri::AppHandle,
    state: Arc<Backend>,
    method: String,
    input: Option<Value>,
    source: &str,
) -> Result<Value, String> {
    use beaver_core::call_log;
    if crate::asset_task_runtime::transient(&method) {
        return business_routing::call(app, state, method, input, source).await;
    }
    let started = std::time::Instant::now();
    let raw = input.clone().unwrap_or_else(|| json!({}));
    let log_context = business_routing::call_log_context(
        &state.project_storage,
        state.store.clone(),
        &state.root,
        &method,
        &raw,
    )?;
    let source_store = log_context.handles.store.clone();
    let id = {
        let store = log_context
            .handles
            .store
            .lock()
            .map_err(|_| "数据库锁不可用")?;
        call_log::begin(
            &store,
            source,
            &method,
            log_context.task_id.as_deref(),
            log_context.project_id.as_deref(),
            &raw,
        )
        .map_err(|e| e.to_string())?
    };
    let result = business_routing::call(app, state.clone(), method.clone(), input, source).await;
    if let Ok(store) = log_context.handles.store.lock() {
        let output = match &result {
            Ok(v) => v.clone(),
            Err(e) => json!({"error":e}),
        };
        if let Ok(value) = &result {
            let task_id = if method.starts_with("task.") && value.get("projectId").is_some() {
                value["id"].as_str()
            } else {
                None
            };
            let project_id = value["projectId"].as_str().or_else(|| {
                if matches!(method.as_str(), "project.create" | "project.import") {
                    value["id"].as_str()
                } else {
                    None
                }
            });
            let _ = call_log::link(&store, &id, task_id, project_id);
        }
        // Never turn an already committed mutation into an apparent retryable failure.
        if call_log::finish(
            &store,
            &id,
            if result.as_ref().is_ok_and(|v| v["ok"] != false) {
                "succeeded"
            } else {
                "failed"
            },
            started.elapsed().as_millis() as u64,
            &output,
        )
        .is_err()
        {
            eprintln!("Beaver call-log completion write failed for {id}");
        }
    }
    replicate_project_call_log(&state.project_storage, source_store, &id, &method, &result);
    result
}

#[cfg(test)]
mod tests {
    use super::replicate_project_call_log;
    use beaver_core::{
        call_log, project_storage::ProjectStore, project_storage_router::ProjectStorageRouter,
        store::Store,
    };
    use serde_json::{json, Value};
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    #[test]
    fn successful_project_creation_log_is_visible_in_local_runtime() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let project = json!({"id":"project-a","name":"Demo","path":root});
        ProjectStore::initialize(&root, "project-a")?.store().put(
            "project",
            "project-a",
            &project,
        )?;
        host.lock().map_err(|_| anyhow::anyhow!("host lock"))?.put(
            "project",
            "project-a",
            &project,
        )?;
        let call_id = {
            let store = host.lock().map_err(|_| anyhow::anyhow!("host lock"))?;
            let id = call_log::begin(
                &store,
                "api",
                "project.create",
                None,
                None,
                &json!({"name":"Demo"}),
            )?;
            call_log::link(&store, &id, None, Some("project-a"))?;
            call_log::finish(&store, &id, "succeeded", 3, &project)?;
            id
        };
        let router = ProjectStorageRouter::new(host.clone());
        let result: Result<Value, String> = Ok(json!({"id":"project-a"}));
        replicate_project_call_log(&router, host, &call_id, "project.create", &result);

        let runtime = router.runtime_for_project("project-a")?;
        let project_store = runtime.store();
        let store = project_store
            .lock()
            .map_err(|_| anyhow::anyhow!("project lock"))?;
        let page = call_log::query(&store, &json!({"projectId":"project-a"}))?;
        assert_eq!(page["records"].as_array().unwrap().len(), 1);
        assert_eq!(page["records"][0]["id"], call_id);
        Ok(())
    }

    #[test]
    fn successful_project_import_log_is_visible_in_local_runtime() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let project_id = "imported-project";
        let local = ProjectStore::initialize(&root, project_id)?;
        local.store().put(
            "project",
            project_id,
            &json!({
                "id": project_id,
                "name": "Imported",
                "path": root,
                "createdAt": "2026-09-19T00:00:00.000Z"
            }),
        )?;
        drop(local);

        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let project = json!({
            "id": project_id,
            "name": "Imported",
            "path": fs::canonicalize(&root)?
        });
        host.lock()
            .map_err(|_| anyhow::anyhow!("host lock"))?
            .put("project", project_id, &project)?;
        let call_id = {
            let store = host.lock().map_err(|_| anyhow::anyhow!("host lock"))?;
            let id = call_log::begin(
                &store,
                "api",
                "project.import",
                None,
                None,
                &json!({"path": root}),
            )?;
            call_log::link(&store, &id, None, Some(project_id))?;
            call_log::finish(&store, &id, "succeeded", 3, &project)?;
            id
        };
        let router = ProjectStorageRouter::new(host.clone());
        let result: Result<Value, String> = Ok(json!({"id":project_id}));
        replicate_project_call_log(&router, host, &call_id, "project.import", &result);

        let runtime = router.runtime_for_project(project_id)?;
        let store_handle = runtime.store();
        let store = store_handle
            .lock()
            .map_err(|_| anyhow::anyhow!("project lock"))?;
        let page = call_log::query(&store, &json!({"projectId":project_id}))?;
        assert_eq!(page["records"].as_array().unwrap().len(), 1);
        assert_eq!(page["records"][0]["id"], call_id);
        Ok(())
    }
}
