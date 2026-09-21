use anyhow::Result;
use beaver_core::{project_storage::ProjectStore, store::Store, validation::task_completion};
use serde_json::{json, Value};
use std::{fs, path::Path};

const REPAIR: &str =
    r#"{"engineVersion":"4.4.1","snapshotId":"snapshot","code":"gut","error":"failed"}"#;

fn failed_parent(project_id: &str) -> Result<Value> {
    Ok(json!({
        "id":"parent","projectId":project_id,"title":"Story","prompt":"Write a story",
        "status":"failed","integrationValidation":true,"autoAccept":false,
        "subtaskIds":[],"validationRepair":serde_json::from_str::<Value>(REPAIR)?
    }))
}

fn initialize_project(parent: &Path, id: &str) -> Result<std::path::PathBuf> {
    let root = parent.join(id);
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(&root, id)?.into_runtime();
    let handle = runtime.store();
    let store = handle.lock().unwrap();
    store.put("project", id, &json!({"id":id,"path":root}))?;
    store.put("task", "parent", &failed_parent(id)?)?;
    Ok(root)
}

fn task_ids(store: &Store) -> Result<Vec<String>> {
    Ok(store
        .list_with_ids::<Value>("task")?
        .into_iter()
        .map(|(id, _)| id)
        .collect())
}

#[test]
fn repair_child_and_events_stay_in_the_project_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    host.put(
        "project",
        "p",
        &json!({"id":"p","path":temp.path().join("p")}),
    )?;
    let mut shadow = failed_parent("p")?;
    shadow["status"] = json!("queued");
    host.put("task", "parent", &shadow)?;
    let root = initialize_project(temp.path(), "p")?;
    let sibling_root = initialize_project(temp.path(), "q")?;

    let child_id = {
        let runtime = ProjectStore::open(&root, "p")?.into_runtime();
        let files = runtime.files();
        let handle = runtime.store();
        let mut store = handle.lock().unwrap();
        let mut parent: Value = store.get("task", "parent")?.unwrap();
        task_completion::repair(&mut store, &files, &mut parent)?;
        let child_id = parent["subtaskIds"][0].as_str().unwrap().to_owned();
        let child: Value = store.get("task", &child_id)?.unwrap();
        assert_eq!(child["projectId"], "p");
        assert_eq!(child["workspace"], format!(".beaver/workspaces/{child_id}"));
        assert_eq!(
            store.get::<Value>("task", "parent")?.unwrap()["status"],
            "waitingChildren"
        );
        let child_events = store.events(&child_id)?;
        assert_eq!(child_events.len(), 1);
        assert_eq!(child_events[0].kind, "system");
        let parent_events = store.events("parent")?;
        assert_eq!(parent_events.len(), 1);
        assert_eq!(parent_events[0].kind, "validationRepair");
        child_id
    };

    assert_eq!(task_ids(&host)?, vec!["parent".to_owned()]);
    assert_eq!(host.get::<Value>("task", "parent")?.unwrap(), shadow);
    assert!(host.events("parent")?.is_empty());
    assert!(host.events(&child_id)?.is_empty());
    assert!(!temp.path().join("host/workspaces").join(&child_id).exists());

    let sibling = ProjectStore::open(&sibling_root, "q")?.into_runtime();
    let sibling_store = sibling.store();
    let sibling_store = sibling_store.lock().unwrap();
    assert_eq!(task_ids(&sibling_store)?, vec!["parent".to_owned()]);
    assert_eq!(
        sibling_store.get::<Value>("task", "parent")?.unwrap()["status"],
        "failed"
    );
    assert!(sibling_store.events("parent")?.is_empty());
    drop(sibling_store);
    drop(sibling);

    let reopened = ProjectStore::open(&root, "p")?.into_runtime();
    let reopened_store = reopened.store();
    let reopened_store = reopened_store.lock().unwrap();
    assert!(reopened_store.get::<Value>("task", &child_id)?.is_some());
    assert_eq!(reopened_store.events(&child_id)?.len(), 1);
    Ok(())
}

#[test]
fn steering_child_stays_in_the_project_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    host.put("task", "parent", &failed_parent("p")?)?;
    let root = initialize_project(temp.path(), "p")?;

    let runtime = ProjectStore::open(&root, "p")?.into_runtime();
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle.lock().unwrap();
    let mut parent: Value = store.get("task", "parent")?.unwrap();
    task_completion::steered(&mut store, &files, &mut parent, "Add another chapter")?;
    let child_id = parent["subtaskIds"][0].as_str().unwrap().to_owned();
    let child: Value = store.get("task", &child_id)?.unwrap();
    assert_eq!(child["origin"], "user");
    assert_eq!(child["prompt"], "Add another chapter");
    assert_eq!(store.events("parent")?[0].kind, "validationSteering");

    assert_eq!(task_ids(&host)?, vec!["parent".to_owned()]);
    assert_eq!(
        host.get::<Value>("task", "parent")?.unwrap()["status"],
        "failed"
    );
    assert!(host.events("parent")?.is_empty());
    Ok(())
}

#[test]
fn integration_and_coverage_read_and_write_only_the_project_store() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    let root = initialize_project(temp.path(), "p")?;
    let runtime = ProjectStore::open(&root, "p")?.into_runtime();
    let handle = runtime.store();
    let store = handle.lock().unwrap();

    let mut parent: Value = store.get("task", "parent")?.unwrap();
    parent["subtaskIds"] = json!(["child"]);
    parent["plan"] = json!({"summary":"Integrated story"});
    let child = json!({
        "id":"child","projectId":"p","parentTaskId":"parent",
        "status":"completed","accepted":true
    });
    store.put("task", "child", &child)?;
    let mut shadow_child = child.clone();
    shadow_child["status"] = json!("failed");
    shadow_child["accepted"] = json!(false);
    host.put("task", "child", &shadow_child)?;

    task_completion::integrated(&store, &mut parent)?;
    assert_eq!(parent["status"], "completed");
    assert!(parent["report"]
        .as_str()
        .unwrap()
        .contains("1 个子任务已合入"));
    let mut host_parent = parent.clone();
    host_parent["status"] = json!("failed");
    assert!(task_completion::integrated(&host, &mut host_parent).is_err());

    let delivered = json!({
        "id":"delivered","projectId":"p","title":"Deliver","capability":"code",
        "validationVersion":1,"changes":[],
        "codeValidation":{"status":"completed","runId":"run","snapshotId":"snapshot"}
    });
    task_completion::delivered(&store, &delivered)?;
    let coverage: Value = store.get("validationCoverage", "delivered")?.unwrap();
    assert_eq!(coverage["projectId"], "p");
    assert_eq!(coverage["status"], "pending");
    assert!(host
        .get::<Value>("validationCoverage", "delivered")?
        .is_none());
    Ok(())
}
