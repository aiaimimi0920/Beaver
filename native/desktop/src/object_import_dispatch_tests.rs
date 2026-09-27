use super::*;
use anyhow::Result;
use beaver_core::project_storage::ProjectStore;
use serde_json::json;
use std::fs;

struct Fixture {
    router: ProjectStorageRouter,
    host: Arc<Mutex<Store>>,
    temp: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let router = ProjectStorageRouter::new(host.clone());
        for id in ["source-1", "target-1", "other-target"] {
            let root = temp.path().join(id);
            fs::create_dir(&root)?;
            fs::write(root.join("project.godot"), "config_version=5\n")?;
            let project = ProjectStore::initialize(&root, id)?;
            project.store().put("project", id, &json!({"id":id}))?;
            drop(project);
            if id != "source-1" {
                host.lock()
                    .unwrap()
                    .put("project", id, &json!({"id":id,"path":root}))?;
                drop(router.open_registered(id)?);
            }
        }
        Ok(Self { router, host, temp })
    }

    fn call(&self, method: &str, input: &Value) -> Result<Value> {
        crate::business_catalog::validate(method, input)
            .map_err(|(_, error)| anyhow::anyhow!(error))?;
        dispatch(
            &self.router,
            &self.host,
            self.temp.path(),
            method,
            Some(input),
        )
        .expect("method must be dispatched")
        .map_err(anyhow::Error::msg)
    }
}

#[test]
fn ordinary_file_preparation_dispatch_persists_only_to_its_bound_target() -> Result<()> {
    let f = Fixture::new()?;
    let path = f.temp.path().join("source-1/asset.txt");
    fs::write(&path, "ordinary asset")?;
    let modified = fs::metadata(&path)?.modified()?;
    let snapshot = f.call("object.inspectFiles", &json!({"paths":[path]}))?;
    assert_eq!(snapshot["files"].as_array().unwrap().len(), 1);
    let request = json!({
        "requestId":"ordinary-files", "targetProjectId":"target-1",
        "snapshot":snapshot, "groups":[]
    });
    let receipt = f.call("object.prepareFileImport", &request)?;
    assert_eq!(receipt["readyToCommit"], false);
    assert_eq!(receipt["targetProjectId"], "target-1");
    assert_eq!(receipt["source"], snapshot);
    assert_eq!(f.call("object.prepareFileImport", &request)?, receipt);
    assert_eq!(
        f.call(
            "object.getFileImportPreparation",
            &json!({
                "targetProjectId":"target-1", "preparationId":receipt["preparationId"]
            })
        )?,
        receipt
    );
    assert!(f
        .call(
            "object.getFileImportPreparation",
            &json!({
                "targetProjectId":"other-target", "preparationId":receipt["preparationId"]
            })
        )?
        .is_null());
    for id in ["target-1", "other-target"] {
        let runtime = f.router.runtime_for_project(id)?;
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        let stored = store.list::<Value>("file_object_import_preparation")?;
        assert_eq!(stored.len(), usize::from(id == "target-1"));
        for kind in ["object", "object_version", "task", "asset_task"] {
            assert!(store.list::<Value>(kind)?.is_empty(), "unexpected {kind}");
        }
        assert!(!runtime.project_root().join("asset.txt").exists());
    }
    assert!(f
        .host
        .lock()
        .unwrap()
        .list::<Value>("file_object_import_preparation")?
        .is_empty());
    assert_eq!(fs::read_to_string(&path)?, "ordinary asset");
    assert_eq!(fs::metadata(&path)?.modified()?, modified);
    fs::remove_file(&path)?;
    let page = f.call(
        "object.importPreparations",
        &json!({"projectId":"target-1"}),
    )?;
    assert_eq!(page["entries"].as_array().unwrap().len(), 1);
    assert_eq!(page["entries"][0]["kind"], "files");
    assert_eq!(
        page["entries"][0]["preparationId"],
        receipt["preparationId"]
    );
    assert_eq!(
        f.call(
            "object.getFileImportPreparation",
            &json!({
                "targetProjectId":"target-1", "preparationId":page["entries"][0]["preparationId"]
            })
        )?,
        receipt
    );
    assert!(f.call(
        "object.importPreparations",
        &json!({"projectId":"other-target"})
    )?["entries"]
        .as_array()
        .unwrap()
        .is_empty());
    Ok(())
}

#[test]
fn external_inspection_dispatch_discovers_identity_and_enforces_expected_identity() -> Result<()> {
    let f = Fixture::new()?;
    let path = f.temp.path().join("source-1");
    let registrations = f.host.lock().unwrap().list::<Value>("project")?;
    let snapshot = f.call("object.inspectExternal", &json!({"path":path}))?;
    assert_eq!(snapshot["project"]["id"], "source-1");
    assert!(snapshot["objects"].as_array().unwrap().is_empty());
    assert_eq!(
        f.call(
            "object.inspectExternal",
            &json!({"path":path,"projectId":"source-1"})
        )?,
        snapshot
    );
    for expected in [Value::Null, json!(1), json!(""), json!("other")] {
        let input = json!({"path":path, "projectId":expected});
        assert!(dispatch(
            &f.router,
            &f.host,
            f.temp.path(),
            "object.inspectExternal",
            Some(&input)
        )
        .unwrap()
        .is_err());
    }
    assert!(f.router.runtime_for_project("source-1").is_err());
    assert_eq!(
        f.host.lock().unwrap().list::<Value>("project")?,
        registrations
    );
    Ok(())
}

#[test]
fn dispatch_rejects_missing_import_input_and_leaves_other_methods_to_their_owner() -> Result<()> {
    let f = Fixture::new()?;
    for method in [
        "object.inspectExternal",
        "object.inspectFiles",
        "object.prepareImport",
        "object.getImportPreparation",
        "object.importPreparations",
        "object.prepareFileImport",
        "object.getFileImportPreparation",
        "object.commitFileImport",
        "object.fileImportOperation",
        "object.abortFileImport",
        "object.commitImport",
        "object.importOperation",
        "object.abortImport",
    ] {
        assert!(dispatch(&f.router, &f.host, f.temp.path(), method, None)
            .unwrap()
            .is_err());
    }
    assert!(dispatch(&f.router, &f.host, f.temp.path(), "object.list", None).is_none());
    Ok(())
}

#[test]
fn formal_file_import_routes_to_target_and_replays_without_source() -> Result<()> {
    let f = Fixture::new()?;
    let path = f.temp.path().join("source-1/asset.txt");
    fs::write(&path, "asset")?;
    let snapshot = f.call("object.inspectFiles", &json!({"paths":[path]}))?;
    let preparation = f.call("object.prepareFileImport", &json!({
        "requestId":"commit-files", "targetProjectId":"target-1", "snapshot":snapshot, "groups":[]
    }))?;
    let input = json!({"projectId":"target-1", "preparationId":preparation["preparationId"]});
    assert!(f.call("object.fileImportOperation", &input)?.is_null());
    let imported = f.call("object.commitFileImport", &input)?;
    assert_eq!(imported["state"], "importedPendingValidation", "{imported}");
    assert_eq!(fs::read_to_string(&path)?, "asset");
    fs::remove_file(path)?;
    assert_eq!(f.call("object.commitFileImport", &input)?, imported);
    assert_eq!(f.call("object.fileImportOperation", &input)?, imported);
    assert!(f.call("object.abortFileImport", &input).is_err());
    let foreign = json!({"projectId":"other-target", "preparationId":preparation["preparationId"]});
    assert!(f.call("object.commitFileImport", &foreign).is_err());
    assert!(f.call("object.fileImportOperation", &foreign).is_err());
    for (project, count) in [("target-1", 1), ("other-target", 0)] {
        assert_eq!(
            f.router
                .runtime_for_project(project)?
                .store()
                .lock()
                .unwrap()
                .list::<Value>("object")?
                .len(),
            count
        );
    }
    assert!(f.host.lock().unwrap().list::<Value>("object")?.is_empty());
    Ok(())
}

#[test]
fn formal_project_import_routes_to_target_and_replays_offline() -> Result<()> {
    let f = Fixture::new()?;
    let root = f.temp.path().join("source-1");
    let source = ProjectStore::open(&root, "source-1")?;
    let version = json!({"versionId":"v1","manifest":{
        "schemaVersion":1,"projectId":"source-1","objectId":"hero","versionId":"v1",
        "status":"accepted","name":"Hero","category":"其他","tags":[],
        "thumbnailPath":null,"parentObjectId":null,"components":[],"files":[],"references":[]
    }});
    source.store().put(
        "object",
        "hero",
        &json!({
            "id":"hero","projectId":"source-1","name":"Hero","components":[],
            "files":[],"references":[],"versions":[version]
        }),
    )?;
    source
        .store()
        .put("object_version", "v1", &json!(["hero", version]))?;
    drop(source);
    let snapshot = f.call("object.inspectExternal", &json!({"path":root}))?;
    let receipt = f.call(
        "object.prepareImport",
        &json!({
            "requestId":"project-commit","targetProjectId":"target-1",
            "source":{"path":root,"projectId":"source-1"},"objectId":"hero",
            "baseline":{"kind":"pinnedVersion","versionId":"v1"},
            "sourceDigest":snapshot["importVersions"][0]["sourceDigest"]
        }),
    )?;
    let input = json!({"projectId":"target-1","preparationId":receipt["preparationId"]});
    assert!(f.call("object.importOperation", &input)?.is_null());
    let imported = f.call("object.commitImport", &input)?;
    assert_eq!(imported["state"], "importedPendingValidation", "{imported}");
    fs::rename(&root, f.temp.path().join("offline"))?;
    assert_eq!(f.call("object.commitImport", &input)?, imported);
    assert_eq!(f.call("object.importOperation", &input)?, imported);
    assert!(f.call("object.abortImport", &input).is_err());
    let foreign = json!({"projectId":"other-target","preparationId":receipt["preparationId"]});
    assert!(f.call("object.commitImport", &foreign).is_err());
    assert!(f.call("object.importOperation", &foreign).is_err());
    assert!(f
        .router
        .runtime_for_project("other-target")?
        .store()
        .lock()
        .unwrap()
        .list::<Value>("object")?
        .is_empty());
    assert!(f.host.lock().unwrap().list::<Value>("object")?.is_empty());
    Ok(())
}
