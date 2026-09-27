use super::{call_log_context, CallLogContext};
use anyhow::Result;
use beaver_core::{
    call_log, project_storage::ProjectStore, project_storage_router::ProjectStorageRouter,
    store::Store,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct Fixture {
    router: ProjectStorageRouter,
    host: Arc<Mutex<Store>>,
    source: PathBuf,
    temp: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let source = temp.path().join("source-1");
        for id in ["source-1", "target-1"] {
            let root = temp.path().join(id);
            fs::create_dir(&root)?;
            fs::write(root.join("project.godot"), "config_version=5\n")?;
            let project = ProjectStore::initialize(&root, id)?;
            project.store().put("project", id, &json!({"id":id}))?;
            if id == "source-1" {
                project.store().put(
                    "task",
                    "source-task",
                    &json!({
                        "id":"source-task", "projectId":id, "status":"running"
                    }),
                )?;
            }
        }
        Ok(Self {
            router: ProjectStorageRouter::new(host.clone()),
            host,
            source,
            temp,
        })
    }

    fn register(&self, id: &str) -> Result<()> {
        self.host.lock().unwrap().put(
            "project",
            id,
            &json!({
                "id":id, "path":self.temp.path().join(id)
            }),
        )
    }

    fn begin(&self, method: &str, input: &Value) -> Result<(CallLogContext, String)> {
        let context = call_log_context(
            &self.router,
            self.host.clone(),
            self.temp.path(),
            method,
            input,
        )
        .map_err(anyhow::Error::msg)?;
        let id = call_log::begin(
            &context.handles.store.lock().unwrap(),
            "test",
            method,
            context.task_id.as_deref(),
            context.project_id.as_deref(),
            input,
        )?;
        Ok((context, id))
    }
}

fn records(store: &Arc<Mutex<Store>>) -> Result<Vec<Value>> {
    Ok(
        call_log::query(&store.lock().unwrap(), &json!({}))?["records"]
            .as_array()
            .unwrap()
            .clone(),
    )
}

#[test]
fn object_import_inspection_logs_only_to_host_for_all_source_states() -> Result<()> {
    let f = Fixture::new()?;
    for state in ["unregistered", "closed", "open"] {
        if state == "closed" {
            f.register("source-1")?;
        } else if state == "open" {
            drop(f.router.open_registered("source-1")?);
        }
        for expected in [None, Some("source-1")] {
            let mut input = json!({"path":f.source, "taskId":"source-task"});
            if let Some(project_id) = expected {
                input["projectId"] = json!(project_id);
            }
            let (context, id) = f.begin("object.inspectExternal", &input)?;
            assert!(Arc::ptr_eq(&context.handles.store, &f.host));
            assert!(!context.handles.project_routed);
            assert_eq!(context.project_id, None);
            assert_eq!(context.task_id, None);
            let snapshot = f.router.inspect_object_source(&f.source, expected, None)?;
            assert_eq!(snapshot.project["id"], "source-1");
            call_log::finish(
                &context.handles.store.lock().unwrap(),
                &id,
                "succeeded",
                1,
                &serde_json::to_value(snapshot)?,
            )?;
        }
        if state != "open" {
            assert!(f.router.runtimes()?.is_empty());
        }
    }
    let calls = records(&f.host)?;
    assert_eq!(calls.len(), 6);
    assert!(calls.iter().all(|call| call["status"] == "succeeded"
        && call["projectId"].is_null()
        && call["taskId"].is_null()));
    let source = f.router.runtime_for_project("source-1")?;
    assert!(records(&source.store())?.is_empty());
    assert_eq!(
        source
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "source-task")?
            .unwrap()["status"],
        "running"
    );
    Ok(())
}

#[test]
fn object_import_preparation_logs_to_bound_target_ignoring_unrelated_routing_fields() -> Result<()>
{
    let f = Fixture::new()?;
    for id in ["source-1", "target-1"] {
        f.register(id)?;
        drop(f.router.open_registered(id)?);
    }
    let input = json!({
        "targetProjectId":"target-1", "source":{"path":f.source,"projectId":"source-1"},
        "projectId":"source-1", "taskId":"source-task"
    });
    let mut expected = Vec::new();
    for method in [
        "object.prepareImport",
        "object.prepareFileImport",
        "object.getFileImportPreparation",
    ] {
        let (context, id) = f.begin(method, &input)?;
        assert!(context.handles.project_routed);
        assert_eq!(context.project_id.as_deref(), Some("target-1"));
        assert_eq!(context.task_id, None);
        call_log::finish(
            &context.handles.store.lock().unwrap(),
            &id,
            "succeeded",
            1,
            &json!({"readyToCommit":false}),
        )?;
        expected.push((id, method));
    }
    let target = f.router.runtime_for_project("target-1")?;
    let calls = records(&target.store())?;
    assert_eq!(calls.len(), expected.len());
    for (id, method) in expected {
        let call = calls.iter().find(|call| call["id"] == id).unwrap();
        assert_eq!(call["method"], method);
        assert_eq!(call["projectId"], "target-1");
        assert!(call["taskId"].is_null());
    }
    let source = f.router.runtime_for_project("source-1")?;
    assert!(records(&source.store())?.is_empty());
    assert!(records(&f.host)?.is_empty());
    Ok(())
}

#[test]
fn object_import_file_inspection_ignores_project_and_task_log_context() -> Result<()> {
    let f = Fixture::new()?;
    f.register("source-1")?;
    let source = f.router.open_registered("source-1")?;
    let (context, _) = f.begin(
        "object.inspectFiles",
        &json!({
            "paths":[f.source], "projectId":"source-1", "taskId":"source-task"
        }),
    )?;
    assert!(Arc::ptr_eq(&context.handles.store, &f.host));
    assert_eq!(context.project_id, None);
    assert_eq!(context.task_id, None);
    assert_eq!(records(&f.host)?.len(), 1);
    assert!(records(&source.store())?.is_empty());
    Ok(())
}

#[test]
fn object_import_self_target_is_rejected_before_any_call_log_write() -> Result<()> {
    let f = Fixture::new()?;
    f.register("source-1")?;
    let source = f.router.open_registered("source-1")?;
    let input = json!({
        "targetProjectId":"source-1", "source":{"path":f.source,"projectId":"source-1"}
    });
    let error = f
        .begin("object.prepareImport", &input)
        .err()
        .expect("self import must fail");
    assert!(error.to_string().contains("IMPORT_TARGET_EQUALS_SOURCE"));
    assert!(records(&source.store())?.is_empty());
    assert!(records(&f.host)?.is_empty());
    Ok(())
}
