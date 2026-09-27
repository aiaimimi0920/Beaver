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

#[test]
fn object_scene_preview_dispatch_is_durable_and_project_local() -> Result<()> {
    let f = Fixture::new()?;
    fs::write(
        f.temp.path().join("p/scene.tscn"),
        "[gd_scene format=3]\n[node name=\"Root\" type=\"Node\"]\n",
    )?;
    let registered = f.call("object.register", &json!({"projectId":"p","requestId":"scene-register","name":"Scene","files":[{"path":"scene.tscn","role":"source"}]}))?;
    let captured = f.call("object.captureVersion", &json!({"projectId":"p","requestId":"scene-capture","objectId":registered["object"]["id"],"expectedRevision":0}))?;
    let version = &captured["object"]["versions"][0];
    let target = json!({"projectId":"p","objectId":registered["object"]["id"],"versionId":version["versionId"],"path":"scene.tscn","sha256":version["manifest"]["files"][0]["sha256"]});
    fs::remove_file(f.temp.path().join("p/scene.tscn"))?;
    assert!(f.call("object.scenePreview.get", &target)?.is_null());
    let input =
        json!({"projectId":"p","requestId":"scene-render","target":target,"resolution":"1080p"});
    let receipt = f.call("object.scenePreview.run", &input)?;
    assert_eq!(f.call("object.scenePreview.run", &input)?, receipt);
    let result = f.call("object.scenePreview.get", &target)?;
    assert_eq!(result["run"]["id"], receipt["runId"]);
    assert_eq!(result["run"]["status"], "queued");
    assert_eq!(result["resolution"], json!({"width":1920,"height":1080}));
    let mut wrong = input.clone();
    wrong["projectId"] = json!("other");
    assert!(f.call("object.scenePreview.run", &wrong).is_err());
    wrong["target"]["projectId"] = json!("other");
    assert!(f.call("object.scenePreview.run", &wrong).is_err());
    for kind in ["objectScenePreview", "validationRun"] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        assert!(f
            .router
            .runtime_for_project("other")?
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
        assert_eq!(
            f.router
                .runtime_for_project("p")?
                .store()
                .lock()
                .unwrap()
                .list::<Value>(kind)?
                .len(),
            1
        );
    }
    Ok(())
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let router = ProjectStorageRouter::new(host.clone());
        for id in ["p", "other", "closed"] {
            let root = temp.path().join(id);
            fs::create_dir(&root)?;
            fs::write(root.join("project.godot"), "config_version=5\n")?;
            let project = ProjectStore::initialize(&root, id)?;
            project.store().put("project", id, &json!({"id":id}))?;
            drop(project);
            host.lock()
                .unwrap()
                .put("project", id, &json!({"id":id,"path":root}))?;
            if id != "closed" {
                drop(router.open_registered(id)?);
            }
        }
        host.lock()
            .unwrap()
            .put("project", "legacy", &json!({"id":"legacy"}))?;
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
        .expect("catalog method")
        .map_err(anyhow::Error::msg)
    }
}

#[test]
fn object_catalog_dispatch_registers_edits_captures_and_replays_original_receipts() -> Result<()> {
    let f = Fixture::new()?;
    let input = json!({"projectId":"p","requestId":"register","name":"Empty"});
    let registered = f.call("object.register", &input)?;
    let id = &registered["object"]["id"];
    assert_eq!(registered["object"]["revision"], 0);
    assert_eq!(registered["object"]["versions"], json!([]));
    assert!(registered["versionId"].is_null());
    let edit = json!({"projectId":"p","requestId":"edit","objectId":id,"expectedRevision":0,"name":"Hero","components":[{"id":"scene","kind":"scene","name":"Hero scene"}],"files":[{"path":"hero.txt","role":"source"}],"references":[]});
    let edited = f.call("object.updateRegistration", &edit)?;
    assert_eq!(edited["object"]["revision"], 1);
    assert_eq!(f.call("object.updateRegistration", &edit)?, edited);
    let path = f.temp.path().join("p/hero.txt");
    fs::write(&path, "frozen hero")?;
    let capture = json!({"projectId":"p","requestId":"capture","objectId":id,"expectedRevision":1});
    let captured = f.call("object.captureVersion", &capture)?;
    assert_eq!(captured["object"]["revision"], 2);
    let version = &captured["object"]["versions"][0];
    assert_eq!(version["versionId"], captured["versionId"]);
    assert_eq!(version["manifest"]["status"], "captured");
    assert_eq!(version["manifest"]["files"][0]["bytes"], 11);
    fs::remove_file(&path)?;
    let file_request = json!({"projectId":"p","objectId":id,"versionId":version["versionId"],
        "path":"hero.txt","sha256":version["manifest"]["files"][0]["sha256"]});
    let preview = f.call("object.versionFile", &file_request)?;
    assert_eq!(preview["request"], file_request);
    assert_eq!(
        preview["content"],
        json!({"kind":"text","text":"frozen hero"})
    );
    let mut foreign = file_request.clone();
    foreign["projectId"] = json!("other");
    assert!(f.call("object.versionFile", &foreign).is_err());
    assert_eq!(f.call("object.captureVersion", &capture)?, captured);
    assert_eq!(f.call("object.register", &input)?, registered);
    assert_eq!(
        f.call("object.list", &json!({"projectId":"p","query":"hero.txt"}))?,
        json!([captured["object"]])
    );
    assert_eq!(
        f.call("object.get", &json!({"projectId":"p","objectId":id}))?,
        captured["object"]
    );
    assert!(f
        .call("object.get", &json!({"projectId":"other","objectId":id}))?
        .is_null());
    let snapshot = f
        .router
        .inspect_object_source(&f.temp.path().join("p"), Some("p"), None)?;
    assert_eq!(snapshot.import_versions.len(), 1);
    assert!(snapshot.import_versions[0].source_digest.is_none());
    assert!(snapshot.import_versions[0]
        .blocker
        .as_deref()
        .unwrap()
        .contains("IMPORT_VERSION_NOT_ACCEPTED"));
    let runtime = f.router.runtime_for_project("p")?;
    let handle = runtime.store();
    let store = handle.lock().unwrap();
    assert_eq!(store.list::<Value>("object_command_receipt")?.len(), 3);
    assert_eq!(store.list::<Value>("object_version")?.len(), 1);
    assert!(store.list::<Value>("task")?.is_empty());
    assert!(f.host.lock().unwrap().list::<Value>("object")?.is_empty());
    Ok(())
}

#[test]
fn object_catalog_mutations_never_fall_back_to_host_or_another_project() -> Result<()> {
    let f = Fixture::new()?;
    let object = f.call(
        "object.register",
        &json!({"projectId":"p","requestId":"r","name":"Owned"}),
    )?;
    for project in ["legacy", "closed", "missing", "other"] {
        let input = json!({"projectId":project,"requestId":"r","objectId":object["object"]["id"],"expectedRevision":0});
        assert!(f.call("object.captureVersion", &input).is_err());
        let mut update = input;
        for (key, value) in [
            ("name", json!("Wrong")),
            ("components", json!([])),
            ("files", json!([])),
            ("references", json!([])),
        ] {
            update[key] = value;
        }
        assert!(f.call("object.updateRegistration", &update).is_err());
        if project != "other" {
            assert!(f
                .call(
                    "object.register",
                    &json!({"projectId":project,"requestId":"r","name":"Wrong"})
                )
                .is_err());
        }
    }
    for kind in ["object", "object_version", "object_command_receipt", "task"] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        let runtime = f.router.runtime_for_project("other")?;
        assert!(runtime
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
    }
    assert!(f.router.runtime_for_project("closed").is_err());
    Ok(())
}

#[test]
fn object_catalog_dispatch_rejects_unknown_and_unpinned_nested_fields() -> Result<()> {
    let f = Fixture::new()?;
    for (key, value) in [
        ("versions", json!([])),
        (
            "components",
            json!([{"id":"c","kind":"scene","name":"C","accepted":true}]),
        ),
        (
            "files",
            json!([{"path":"hero.txt","role":"source","owner":"other"}]),
        ),
        (
            "references",
            json!([{"projectId":"p","objectId":"other","versionId":null}]),
        ),
        (
            "references",
            json!([{"path":"hero.txt","note":"task reference"}]),
        ),
    ] {
        let mut input = json!({"projectId":"p","requestId":"r","name":"Invalid"});
        input[key] = value;
        assert!(dispatch(
            &f.router,
            &f.host,
            f.temp.path(),
            "object.register",
            Some(&input)
        )
        .unwrap()
        .is_err());
    }
    for method in [
        "object.register",
        "object.updateRegistration",
        "object.captureVersion",
        "object.list",
        "object.get",
    ] {
        assert!(dispatch(&f.router, &f.host, f.temp.path(), method, None)
            .unwrap()
            .is_err());
    }
    assert!(dispatch(
        &f.router,
        &f.host,
        f.temp.path(),
        "object.prepareImport",
        None
    )
    .is_none());
    assert_eq!(f.call("object.list", &json!({"projectId":"p"}))?, json!([]));
    Ok(())
}

#[test]
fn object_catalog_dispatch_accepts_requested_version_and_replays_receipt() -> Result<()> {
    let f = Fixture::new()?;
    let registered = f.call(
        "object.register",
        &json!({"projectId":"p","requestId":"register-accept","name":"Acceptable"}),
    )?;
    let object_id = registered["object"]["id"].as_str().unwrap();
    let path = f.temp.path().join("p/accept.txt");
    fs::write(&path, "accepted content")?;
    let captured = f.call(
        "object.captureVersion",
        &json!({"projectId":"p","requestId":"capture-accept","objectId":object_id,"expectedRevision":0}),
    )?;
    let version_id = captured["versionId"].as_str().unwrap();
    let request = json!({
        "projectId":"p",
        "requestId":"accept-version",
        "objectId":object_id,
        "versionId":version_id,
        "expectedRevision":1
    });
    let accepted = f.call("object.acceptVersion", &request)?;
    assert_eq!(accepted["object"]["revision"], 2);
    assert_eq!(accepted["versionId"], version_id);
    assert_eq!(
        accepted["object"]["versions"][0]["manifest"]["status"],
        "accepted"
    );
    assert_eq!(accepted["object"]["versions"].as_array().unwrap().len(), 1);
    assert_eq!(f.call("object.acceptVersion", &request)?, accepted);
    assert!(f
        .router
        .runtime_for_project("p")?
        .store()
        .lock()
        .unwrap()
        .list::<Value>("task")?
        .is_empty());
    Ok(())
}
#[test]
fn attempt_scene_preview_dispatch_rejects_invalid_identity() -> Result<()> {
    let f = Fixture::new()?;
    let target = json!({"projectId":"p","runId":"r","attemptId":"missing","checkpoint":"output","path":"scene.tscn","sha256":"a".repeat(64)});
    assert!(f.call("object.attemptScenePreview.get", &target)?.is_null());
    let input = json!({"projectId":"p","requestId":"attempt-preview","target":target});
    assert!(f.call("object.attemptScenePreview.run", &input).is_err());
    let mut invalid = target.clone();
    invalid["checkpoint"] = json!("live");
    assert!(f.call("object.attemptScenePreview.get", &invalid).is_err());
    assert!(f
        .router
        .runtime_for_project("p")?
        .store()
        .lock()
        .unwrap()
        .list::<Value>("validationRun")?
        .is_empty());
    Ok(())
}
