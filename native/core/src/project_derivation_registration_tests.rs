use super::*;
use crate::{
    data_backup, project_derivation_copy as copy, project_derivation_validation_records::Rewrite,
};

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    preparation: PathBuf,
    destination: PathBuf,
    root: PathBuf,
    data: PathBuf,
    host: Arc<Mutex<Store>>,
    router: ProjectStorageRouter,
    task: String,
}

fn local(root: &Path, id: &str, task: &str) -> Result<()> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let storage = ProjectStore::initialize(root, id)?;
    storage.store().put(
        "project",
        id,
        &json!({"id":id,"path":fs::canonicalize(root)?,"name":"Original"}),
    )?;
    fs::create_dir(root.join(".beaver/workspaces").join(task))?;
    storage.store().put(
        "task",
        task,
        &json!({"id":task,"projectId":id,"status":"queued",
        "workspace":format!(".beaver/workspaces/{task}")}),
    )?;
    Ok(())
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        local(&source, "original", "source-task")?;
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
        let task = Rewrite(&prepared.identities).key("task", "source-task")?.id;
        let destination = temp.path().join("assembly");
        let root = assembly::create(&preparation, &destination)?.binding;
        let data = temp.path().join("host");
        let host = Arc::new(Mutex::new(Store::open(&data)?));
        let router = ProjectStorageRouter::new(host.clone());
        Ok(Self {
            _temp: temp,
            source,
            preparation,
            destination,
            root,
            data,
            host,
            router,
            task,
        })
    }

    fn activate(&self) -> Result<()> {
        assembly::activate(&self.preparation, &self.destination)?;
        Ok(())
    }

    fn register(&self) -> Result<Value> {
        self.router
            .register_assembly(&self.preparation, &self.destination, &self.data)
    }
}

#[test]
fn host_write_failure_preserves_assembly_and_existing_routes_then_retries() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    let other = f._temp.path().join("other");
    local(&other, "other", "other-task")?;
    f.host
        .lock()
        .unwrap()
        .put("project", "other", &json!({"id":"other","path":other}))?;
    drop(f.router.open_registered("other")?);
    let source_before = data_backup::inventory(&f.source)?;
    let preparation_before = data_backup::inventory(&f.preparation)?;
    let before = data_backup::inventory(&f.destination)?;
    f.host
        .lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only=ON")?;
    assert!(f.register().is_err());
    assert!(f
        .host
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_none());
    assert!(f.router.runtime_for_project("derived").is_err());
    assert!(f.router.runtime_for_task(&f.task).is_err());
    assert_eq!(
        f.router.runtime_for_task("other-task")?.project_id(),
        "other"
    );
    assert_eq!(data_backup::inventory(&f.destination)?, before);
    f.host
        .lock()
        .unwrap()
        .connection
        .execute_batch("PRAGMA query_only=OFF")?;
    assert_eq!(f.register()?["hostRegistrationChanged"], true);
    assert_eq!(data_backup::inventory(&f.destination)?, before);
    assert_eq!(data_backup::inventory(&f.source)?, source_before);
    assert_eq!(data_backup::inventory(&f.preparation)?, preparation_before);
    let local = ProjectStore::read_project(&f.root, "derived")?;
    assert!(local.get("derivationAssembly").is_none());
    assert!(f.router.runtime_for_task(&f.task).is_err());
    let opened = f.router.open_registered_with("derived", |store, _| {
        assert_eq!(
            store.get::<Value>("task", &f.task)?.unwrap()["status"],
            "interrupted"
        );
        Ok(())
    })?;
    assert_eq!(opened.project_root(), f.root);
    assert_eq!(f.router.runtime_for_task(&f.task)?.project_id(), "derived");
    Ok(())
}

#[test]
fn registration_replay_survives_project_use_runtime_recovery_failure_and_restart() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    f.register()?;
    let failed = f.router.open_registered_with("derived", |store, _| {
        store.put(
            "task",
            "new-task",
            &json!({"id":"new-task","projectId":"derived","status":"completed"}),
        )?;
        anyhow::bail!("simulated lifecycle failure after a durable write")
    });
    assert!(failed.is_err());
    assert!(f.router.runtime_for_project("derived").is_err());
    assert!(f.router.runtime_for_task(&f.task).is_err());
    fs::write(f.root.join("new-content.txt"), "ordinary project edit")?;
    let mut host_project = f
        .host
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .unwrap();
    host_project["name"] = json!("Preserve host metadata");
    f.host
        .lock()
        .unwrap()
        .put("project", "derived", &host_project)?;
    assert!(assembly::inspect_activated(&f.preparation, &f.destination).is_err());
    assert_eq!(f.register()?["hostRegistrationChanged"], false);
    let runtime = f.router.open_registered("derived")?;
    assert!(f.router.runtime_for_task("new-task").is_ok());
    assert_eq!(f.register()?["project"], host_project);
    runtime.store().lock().unwrap().put(
        "task",
        "new-task",
        &json!({"id":"new-task","projectId":"derived","status":"running"}),
    )?;
    assert_eq!(f.register()?["hostRegistrationChanged"], false);
    assert_eq!(
        runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "new-task")?
            .unwrap()["status"],
        "running"
    );
    drop(runtime);
    f.router.close_all()?;
    let restarted = ProjectStorageRouter::new(f.host.clone());
    assert_eq!(
        restarted.register_assembly(&f.preparation, &f.destination, &f.data)?
            ["hostRegistrationChanged"],
        false
    );
    assert!(restarted.runtime_for_task("new-task").is_err());
    drop(restarted.open_registered("derived")?);
    assert!(restarted.runtime_for_task("new-task").is_ok());
    Ok(())
}

#[test]
fn registration_rejects_identity_path_and_task_conflicts_without_side_effects() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    let before = data_backup::inventory(&f.destination)?;
    let conflicting = json!({"id":"derived","path":f._temp.path().join("offline")});
    f.host
        .lock()
        .unwrap()
        .put("project", "derived", &conflicting)?;
    assert!(f.register().is_err());
    assert_eq!(
        f.host.lock().unwrap().get::<Value>("project", "derived")?,
        Some(conflicting)
    );
    f.host.lock().unwrap().remove("project", "derived")?;
    f.host
        .lock()
        .unwrap()
        .put("project", "other", &json!({"id":"other","path":f.root}))?;
    assert!(f.register().is_err());
    let other = f._temp.path().join("other");
    local(&other, "other", &f.task)?;
    f.host
        .lock()
        .unwrap()
        .put("project", "other", &json!({"id":"other","path":other}))?;
    drop(f.router.open_registered("other")?);
    assert!(f.register().unwrap_err().to_string().contains("任务 ID"));
    assert_eq!(f.router.runtime_for_task(&f.task)?.project_id(), "other");
    f.router.close("other")?;
    f.host
        .lock()
        .unwrap()
        .put("task", &f.task, &json!({"id":f.task,"projectId":"other"}))?;
    assert!(f.register().is_err());
    assert!(f
        .host
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_none());
    assert_eq!(data_backup::inventory(&f.destination)?, before);
    Ok(())
}

#[test]
fn registration_rejects_conflicting_draining_runtime() -> Result<()> {
    let f = Fixture::new()?;
    f.activate()?;
    let old = f._temp.path().join("old");
    local(&old, "derived", "old-task")?;
    let registration = json!({"id":"derived","path":old});
    f.host
        .lock()
        .unwrap()
        .put("project", "derived", &registration)?;
    let active = f.router.open_registered("derived")?;
    assert_eq!(
        f.router
            .unregister("derived", registration["path"].as_str().unwrap())?["draining"],
        true
    );
    assert!(f
        .register()
        .unwrap_err()
        .to_string()
        .contains("open runtime"));
    assert_eq!(
        f.router.runtime_for_task("old-task")?.project_root(),
        active.project_root()
    );
    assert!(f
        .host
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_none());
    drop(active);
    f.router.close_unregistered()?;
    assert_eq!(f.register()?["hostRegistrationChanged"], true);
    Ok(())
}

#[test]
fn registration_requires_explicit_activation_and_rejects_busy_or_changed_target() -> Result<()> {
    let f = Fixture::new()?;
    assert!(f.register().is_err());
    assert!(f.destination.join(".beaver-migration-pending").is_file());
    f.activate()?;
    let busy = ProjectStore::open(&f.root, "derived")?;
    assert!(f.register().is_err());
    drop(busy);
    fs::write(f.root.join("unexpected.txt"), "unregistered modification")?;
    let before = data_backup::inventory(&f.destination)?;
    assert!(f
        .register()
        .unwrap_err()
        .to_string()
        .contains("assembly changed"));
    assert!(f
        .host
        .lock()
        .unwrap()
        .get::<Value>("project", "derived")?
        .is_none());
    assert_eq!(data_backup::inventory(&f.destination)?, before);
    Ok(())
}
