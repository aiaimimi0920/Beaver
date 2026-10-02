use super::*;
use beaver_core::{
    data_backup, project_storage::ProjectStore, project_storage_router::ProjectStorageRouter,
    store::Store,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[path = "migration_derivation_object_tests.rs"]
mod objects;

#[path = "migration_derivation_plan_tests.rs"]
mod plans;

#[path = "migration_derivation_declaration_tests.rs"]
mod declarations;
#[path = "migration_derivation_execution_tests.rs"]
mod execution;
#[path = "migration_derivation_planning_tests.rs"]
mod planning;
#[path = "migration_derivation_queue_tests.rs"]
mod queue;

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    preparation: PathBuf,
    destination: PathBuf,
    host: PathBuf,
    store: Arc<Mutex<Store>>,
    router: ProjectStorageRouter,
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
            &json!({"id":"original","path":source,"name":"Preserved"}),
        )?;
        for status in ["running", "queued", "waitingChildren"] {
            storage.store().put(
                "task",
                status,
                &json!({"id":status,"projectId":"original","status":status}),
            )?;
        }
        drop(storage);
        let host = temp.path().join("host");
        let store = Arc::new(Mutex::new(Store::open(&host)?));
        let router = ProjectStorageRouter::new(store.clone());
        Ok(Self {
            source,
            preparation: temp.path().join("preparation"),
            destination: temp.path().join("assembly"),
            host,
            store,
            router,
            _temp: temp,
        })
    }

    fn api(&self, method: &str, input: Value) -> Result<Value> {
        Ok(super::super::call(&self.host, &self.router, method, &input)?.value)
    }

    fn paths(&self) -> Value {
        json!({"preparation":self.preparation,"destination":self.destination})
    }
}

#[test]
fn migration_derivation_api_closes_offline_copy_activation_registration_and_replay() -> Result<()> {
    let _test_operation = super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    {
        let _operation = super::super::OPERATION.lock().unwrap();
        let error = f
            .api(
                "migration.inspectDerivationSource",
                json!({"source":f.source}),
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("another migration operation is in progress"));
    }
    let before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let id = inspection["request"]["targetProjectId"].as_str().unwrap();
    assert_ne!(id, "original");
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    let preparation = f.api("migration.prepareDerivation", request.clone())?;
    assert_eq!(preparation, inspection);
    assert!(f.api("migration.prepareDerivation", request).is_err());
    let prepared_bytes = data_backup::inventory(&f.preparation)?;
    fs::rename(&f.source, f._temp.path().join("offline"))?;
    assert_eq!(
        f.api(
            "migration.inspectDerivation",
            json!({"preparation":f.preparation})
        )?,
        inspection
    );
    let assembled = f.api("migration.assembleDerivation", f.paths())?;
    assert_eq!(assembled["assembly"]["projectId"], id);
    assert_eq!(assembled["assembly"]["tasksInterrupted"], 2);
    assert_eq!(assembled["activated"], false);
    assert_eq!(f.api("migration.inspectAssembly", f.paths())?, assembled);
    assert!(f.api("migration.registerAssembly", f.paths()).is_err());
    assert!(f
        .store
        .lock()
        .unwrap()
        .get::<Value>("project", id)?
        .is_none());
    let activation = f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(activation["host_registration_changed"], false);
    assert_eq!(
        f.api("migration.inspectAssembly", f.paths())?["activated"],
        true
    );
    // A lost activation response is recoverable before ordinary project use.
    assert_eq!(f.api("migration.activateAssembly", f.paths())?, activation);
    let registered = f.api("migration.registerAssembly", f.paths())?;
    assert_eq!(registered["registrationCommitted"], true);
    assert_eq!(registered["runtimeReady"], true);
    let runtime = f.router.runtime_for_project(id)?;
    let store = runtime.store();
    let tasks = store.lock().unwrap().list::<Value>("task")?;
    assert_eq!(
        tasks
            .iter()
            .filter(|task| task["status"] == "interrupted")
            .count(),
        2
    );
    assert!(tasks
        .iter()
        .any(|task| task["status"] == "waitingChildren" && task["planPaused"] == true));
    assert!(tasks.iter().all(|task| task["projectId"] == id));
    let mut project = store.lock().unwrap().get::<Value>("project", id)?.unwrap();
    assert_eq!(project["name"], "Preserved");
    project["name"] = json!("Edited after registration");
    store.lock().unwrap().put("project", id, &project)?;
    let replay = f.api("migration.registerAssembly", f.paths())?;
    assert_eq!(replay["runtimeReady"], true);
    assert_eq!(replay["hostRegistrationChanged"], false);
    assert_eq!(
        store.lock().unwrap().get::<Value>("project", id)?.unwrap()["name"],
        "Edited after registration"
    );
    assert_eq!(data_backup::inventory(&f.preparation)?, prepared_bytes);
    assert_eq!(
        data_backup::inventory(&f._temp.path().join("offline"))?,
        before
    );
    Ok(())
}

#[test]
fn derivation_public_schemas_paths_and_damaged_receipts_fail_without_overwrite() -> Result<()> {
    let f = Fixture::new()?;
    let host = fs::canonicalize(&f.host)?;
    let methods = [
        (
            "migration.inspectDerivationSource",
            json!({"source":f.source}),
            true,
        ),
        (
            "migration.prepareDerivation",
            json!({"source":f.source,"sourceProjectId":"original","targetProjectId":"derived","requestId":"request","preparation":f.preparation}),
            false,
        ),
        (
            "migration.inspectDerivation",
            json!({"preparation":f.preparation}),
            true,
        ),
        ("migration.assembleDerivation", f.paths(), false),
        ("migration.inspectAssembly", f.paths(), true),
    ];
    let tools = crate::business_catalog::tools();
    for (method, input, readonly) in &methods {
        assert!(crate::business_catalog::validate(method, input).is_ok());
        let mut extra = input.clone();
        extra["force"] = json!(true);
        assert!(crate::business_catalog::validate(method, &extra).is_err());
        assert_eq!(
            tools.iter().find(|tool| tool["name"] == *method).unwrap()["annotations"]
                ["readOnlyHint"],
            *readonly
        );
    }
    let inspect = "migration.inspectDerivationSource";
    for source in [json!("relative"), json!(f.host), json!(f._temp.path())] {
        assert!(execute(&host, inspect, &json!({"source":source})).is_err());
    }
    let method = "migration.prepareDerivation";
    let mut input = methods[1].1.clone();
    input["preparation"] = json!(f.host.join("nested"));
    assert!(execute(&host, method, &input).is_err());
    assert!(!f.host.join("nested").exists());
    execute(&host, method, &methods[1].1)?;
    fs::write(f.preparation.join("DERIVATION-COPY.json"), "incomplete")?;
    let before = data_backup::inventory(&f.preparation)?;
    assert!(execute(&host, "migration.inspectDerivation", &methods[2].1).is_err());
    assert!(execute(&host, "migration.assembleDerivation", &f.paths()).is_err());
    assert!(execute(&host, method, &methods[1].1).is_err());
    assert!(!f.destination.exists());
    assert_eq!(data_backup::inventory(&f.preparation)?, before);
    Ok(())
}
