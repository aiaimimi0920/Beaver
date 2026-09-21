use super::super::{partition::restore_and_partition, task, Fixture};
use crate::{
    data_backup, migration_activation, migration_bundle, project_migration_activation,
    project_storage_router::ProjectStorageRouter, store::Store, validation,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[test]
fn converted_copy_routes_project_writes_without_original_paths_or_host_fallback() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let [first, second] = fixture.projects.clone();
    let mut validation_requests = Vec::new();
    for (id, project) in [("task-0", &first), ("task-1", &second)] {
        let mut queued = task(&fixture, id, project)?;
        queued["status"] = json!("queued");
        fixture.store.put("task", id, &queued)?;
        let input = json!({"projectId":project,"requestId":"save-before-migration",
            "expectedRevision":0,"settings":{"visualRequired":false}});
        let result = validation::settings::save(&mut fixture.store, &input)?;
        let (kind, key, record) =
            validation::requests::Request::new("validation.settings.save", &input)?.record(&result);
        assert_eq!(kind, "validationRequest");
        validation_requests.push((input, result, key, record));
    }
    let (_, restored) = restore_and_partition(&fixture)?;
    let backup = fixture.temp.path().join("bundle");
    let archived = data_backup::inventory(&backup)?;
    let original_host = Arc::new(Mutex::new(Store::open(
        &fixture.temp.path().join("offline-host"),
    )?));
    let original_router = ProjectStorageRouter::new(original_host.clone());
    assert!(original_router.open_registered_local()?.is_empty());

    let staged = project_migration_activation::stage(&backup, &restored)?;
    let summary = staged.summary().clone();
    drop(staged);
    assert_eq!(summary["default_data_directory_changed"], false);
    assert_eq!(summary["live_model_request_made"], false);
    let copied_host = Arc::new(Mutex::new(Store::open(&restored.join("data"))?));
    let router = ProjectStorageRouter::new(copied_host.clone());
    assert!(migration_bundle::ensure_activated(&restored.join("data")).is_err());
    assert!(router.open_registered_local().is_err());
    assert!(router.runtimes()?.is_empty());
    assert!(router.runtime_for_task("task-0").is_err());

    // Exercise the receipt/startup boundary, not tool detection or desktop acceptance.
    migration_activation::write_receipt(
        &restored.join("ACTIVATION.json"),
        &restored.join(".beaver-migration-pending"),
        &summary,
    )?;
    migration_bundle::ensure_activated(&restored.join("data"))?;
    for index in 0..2 {
        fs::rename(
            fixture.temp.path().join(format!("game-{index}")),
            fixture
                .temp
                .path()
                .join(format!("unavailable-game-{index}")),
        )?;
    }
    assert_eq!(router.open_registered_local()?.len(), 2);
    for (input, result, key, record) in &validation_requests {
        let project = input["projectId"].as_str().unwrap();
        let runtime = router.runtime_for_project(project)?;
        let handle = runtime.store();
        let mut store = handle.lock().unwrap();
        assert_eq!(
            store.get::<Value>("validationRequest", key)?,
            Some(record.clone())
        );
        // The original expected revision is stale: success must replay the copied receipt.
        assert_eq!(validation::settings::save(&mut store, input)?, *result);
        assert_eq!(validation::settings::read(&store, project)?.revision, 1);
        assert!(copied_host
            .lock()
            .unwrap()
            .get::<Value>("validationRequest", key)?
            .is_none());
        assert_eq!(
            original_host
                .lock()
                .unwrap()
                .get::<Value>("validationRequest", key)?,
            Some(record.clone())
        );
        let other = if project == first { &second } else { &first };
        assert!(router
            .runtime_for_project(other)?
            .store()
            .lock()
            .unwrap()
            .get::<Value>("validationRequest", key)?
            .is_none());
    }
    for (id, project) in [("task-0", &first), ("task-1", &second)] {
        let runtime = router.runtime_for_task(id)?;
        assert_eq!(runtime.project_id(), project);
        assert_eq!(
            runtime.project_root(),
            restored.join("projects").join(project).canonicalize()?
        );
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        let mut converted: Value = store.get("task", id)?.unwrap();
        assert_eq!(converted["status"], "interrupted");
        assert_eq!(converted["workspace"], format!(".beaver/workspaces/{id}"));
        assert!(runtime
            .project_root()
            .join(converted["workspace"].as_str().unwrap())
            .is_dir());
        converted["routingProbe"] = json!(project);
        store.put("task", id, &converted)?;
        assert!(copied_host
            .lock()
            .unwrap()
            .get::<Value>("task", id)?
            .is_none());
        let original: Value = original_host.lock().unwrap().get("task", id)?.unwrap();
        assert_eq!(original["status"], "queued");
        assert!(original.get("routingProbe").is_none());
    }
    assert!(original_router.runtimes()?.is_empty());
    let (projects, tasks) = router.open_state_records()?;
    assert_eq!(projects.len(), 2);
    assert_eq!(tasks.len(), 2);
    assert!(tasks
        .iter()
        .all(|task| task["routingProbe"] == task["projectId"]));
    router.close_all()?;
    assert!(router.runtime_for_task("task-0").is_err());
    assert_eq!(router.open_registered_local()?.len(), 2);
    assert_eq!(router.open_state_records()?.1.len(), 2);
    router.close_all()?;
    assert_eq!(data_backup::inventory(&backup)?, archived);
    Ok(())
}
