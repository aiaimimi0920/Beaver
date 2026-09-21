use super::*;
use beaver_core::{
    data_backup, project_derivation_assembly as assembly, project_derivation_copy as copy,
    project_derivation_identity::Target, project_storage::ProjectStore,
};
use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    preparation: PathBuf,
    destination: PathBuf,
    host: PathBuf,
    store: Arc<Mutex<Store>>,
    router: ProjectStorageRouter,
    task: String,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        fs::create_dir(&source)?;
        fs::write(source.join("project.godot"), "config_version=5\n")?;
        let storage = ProjectStore::initialize(&source, "original")?;
        storage.store().put(
            "project",
            "original",
            &json!({"id":"original","path":source}),
        )?;
        fs::create_dir(source.join(".beaver/workspaces/source-task"))?;
        storage.store().put("task", "source-task", &json!({"id":"source-task",
            "projectId":"original","status":"running","workspace":".beaver/workspaces/source-task"}))?;
        drop(storage);
        let preparation = temp.path().join("preparation");
        let prepared = copy::prepare(
            copy::Request {
                request_id: "derive".into(),
                source: source.clone(),
                source_project_id: "original".into(),
                target_project_id: "derived".into(),
            },
            &preparation,
        )?;
        let task = prepared
            .identities
            .entities
            .iter()
            .find_map(|entity| {
                if entity.source.kind != "task" {
                    return None;
                }
                match &entity.target {
                    Target::Remap { key } => Some(key.id.clone()),
                    Target::Archive => None,
                }
            })
            .context("derived task mapping")?;
        let destination = temp.path().join("assembly");
        assembly::create(&preparation, &destination)?;
        let host = temp.path().join("host");
        let store = Arc::new(Mutex::new(Store::open(&host)?));
        let router = ProjectStorageRouter::new(store.clone());
        Ok(Self {
            _temp: temp,
            source,
            preparation,
            destination,
            host,
            store,
            router,
            task,
        })
    }

    fn request(&self) -> Value {
        json!({"preparation":self.preparation,"destination":self.destination})
    }

    fn activate(&self) -> Result<()> {
        let value =
            super::super::execute(&self.host, "migration.activateAssembly", &self.request())?;
        assert_eq!(value["host_registration_changed"], false);
        assert!(self
            .store
            .lock()
            .unwrap()
            .get::<Value>("project", "derived")?
            .is_none());
        Ok(())
    }

    fn register(&self) -> Result<Outcome> {
        register(
            &self.host,
            &self.router,
            &self.request(),
            crate::project_runtime_lifecycle::recover,
        )
    }
}

#[test]
fn migration_registration_recovers_routes_and_preserves_commit_when_wake_fails() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    let source = data_backup::inventory(&f.source)?;
    let prepared = data_backup::inventory(&f.preparation)?;
    let notified = Cell::new(0);
    let outcome = f.register()?;
    assert!(outcome.registration_committed && outcome.runtime_ready);
    let value = outcome.publish(
        || notified.set(notified.get() + 1),
        || Err("scheduler closed".into()),
    );
    assert_eq!(notified.get(), 1);
    assert_eq!(value["registrationCommitted"], true);
    assert_eq!(value["hostRegistrationChanged"], true);
    assert_eq!(value["runtimeReady"], true);
    assert_eq!(value["schedulerWakeError"], "scheduler closed");
    let runtime = f.router.runtime_for_task(&f.task)?;
    assert_eq!(runtime.project_id(), "derived");
    let store = runtime.store();
    let mut task = store
        .lock()
        .unwrap()
        .get::<Value>("task", &f.task)?
        .unwrap();
    assert_eq!(task["status"], "interrupted");
    task["status"] = json!("running");
    store.lock().unwrap().put("task", &f.task, &task)?;
    let replay = register(&f.host, &f.router, &f.request(), |_, _| {
        panic!("an already-open runtime must not be recovered again")
    })?;
    assert_eq!(replay.value["hostRegistrationChanged"], false);
    assert_eq!(
        store
            .lock()
            .unwrap()
            .get::<Value>("task", &f.task)?
            .unwrap()["status"],
        "running"
    );
    assert_eq!(data_backup::inventory(&f.source)?, source);
    assert_eq!(data_backup::inventory(&f.preparation)?, prepared);
    Ok(())
}

#[test]
fn migration_registration_reports_committed_recovery_failure_and_retry() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    let outcome = register(&f.host, &f.router, &f.request(), |store, files| {
        crate::project_runtime_lifecycle::recover(store, files)?;
        store.put(
            "task",
            "late-task",
            &json!({"id":"late-task","projectId":"derived","status":"running"}),
        )?;
        anyhow::bail!("recovery failed after a durable write")
    })?;
    assert!(outcome.registration_committed && !outcome.runtime_ready);
    let notified = Cell::new(false);
    let value = outcome.publish(
        || notified.set(true),
        || panic!("failed recovery must not wake scheduling"),
    );
    assert!(notified.get());
    assert_eq!(value["registrationCommitted"], true);
    assert_eq!(value["hostRegistrationChanged"], true);
    assert_eq!(value["runtimeReady"], false);
    assert_eq!(value["ok"], false);
    assert!(value["runtimeError"]
        .as_str()
        .unwrap()
        .contains("durable write"));
    assert!(f.router.runtime_for_project("derived").is_err());
    assert!(f.router.runtime_for_task(&f.task).is_err());
    assert!(f
        .store
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_some());
    assert!(assembly::inspect_activated(&f.preparation, &f.destination).is_err());
    let retry = f.register()?;
    assert!(retry.runtime_ready);
    assert_eq!(retry.value["hostRegistrationChanged"], false);
    assert_eq!(retry.value["runtimeError"], Value::Null);
    let runtime = f.router.runtime_for_task("late-task")?;
    assert_eq!(
        runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "late-task")?
            .unwrap()["status"],
        "interrupted"
    );
    Ok(())
}

#[test]
fn migration_registration_requires_explicit_activation_and_absolute_paths() -> Result<()> {
    let f = Fixture::new()?;
    let before = data_backup::inventory(&f.destination)?;
    assert!(f.register().is_err());
    assert_eq!(data_backup::inventory(&f.destination)?, before);
    f.activate()?;
    for request in [
        json!({"preparation":f.preparation}),
        json!({"preparation":"relative","destination":f.destination}),
        json!({"preparation":f.host,"destination":f.destination}),
    ] {
        assert!(register(
            &f.host,
            &f.router,
            &request,
            crate::project_runtime_lifecycle::recover
        )
        .is_err());
    }
    assert!(f
        .store
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_none());
    let method = "migration.registerAssembly";
    assert!(crate::business_catalog::validate(method, &f.request()).is_ok());
    for invalid in [
        json!({}),
        json!({"preparation":"x","destination":1}),
        json!({"preparation":"x","destination":"y","force":true}),
    ] {
        assert!(crate::business_catalog::validate(method, &invalid).is_err());
    }
    Ok(())
}
