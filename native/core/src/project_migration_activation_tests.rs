use super::{partition::restore_and_partition, partition::session_index, task, Fixture};
use crate::{
    project_migration_activation, project_storage::ProjectStore, store::Store, task_actions,
    task_plan,
};
use anyhow::Result;
use serde_json::{json, Value};

#[path = "project_migration_reopen_tests.rs"]
mod reopen;
#[path = "project_migration_routing_tests.rs"]
mod routing;

fn host(restored: &std::path::Path) -> Result<Store> {
    Store::open(&restored.join("data"))
}

#[test]
fn staged_activation_converts_partitions_and_quarantines_retained_history() -> Result<()> {
    let fixture = Fixture::new()?;
    let [first, second] = fixture.projects.clone();
    let mut running = task(&fixture, "task-0", &first)?;
    running["status"] = json!("running");
    fixture.store.put("task", "task-0", &running)?;
    session_index(&fixture, "task-0")?;
    task(&fixture, "task-1", &second)?;
    fixture.store.put(
        "task",
        "task-2",
        &json!({"id":"task-2","projectId":second,"status":"queued",
            "workspace":fixture.temp.path().join("elsewhere/task-2")}),
    )?;
    fixture.store.put(
        "settings",
        "main",
        &json!({"tools":{"codex":"codex","godot":"","node":"","blender":""},"mode":"local"}),
    )?;
    let (_, restored) = restore_and_partition(&fixture)?;
    let backup = fixture.temp.path().join("bundle");

    let staged = project_migration_activation::stage(&backup, &restored)?;
    let summary = staged.summary().clone();
    drop(staged);
    assert!(restored.join(".beaver-migration-pending").is_file());
    assert!(!restored.join("ACTIVATION.json").exists());
    let report = &summary["projects"][&first];
    assert_eq!(report["tasks"], 1);
    assert_eq!(report["sessionIndexes"], 1);
    assert_eq!(report["tasksInterrupted"], 1);
    assert_eq!(summary["projects"][&second]["tasks"], 1);
    assert_eq!(summary["retained_tasks_marked"], 1);
    assert_eq!(summary["host_tasks_interrupted"], 1);
    assert_eq!(summary["external_tools_to_check"], json!(["codex"]));
    assert_eq!(summary["credentials_converted"], 0);

    let mut store = host(&restored)?;
    for id in [&first, &second] {
        let project: Value = store.get("project", id)?.unwrap();
        assert_eq!(
            project["path"].as_str(),
            restored.join("projects").join(id).canonicalize()?.to_str()
        );
    }
    assert!(store.get::<Value>("task", "task-0")?.is_none());
    let retained: Value = store.get("task", "task-2")?.unwrap();
    assert_eq!(retained["status"], "interrupted");
    assert_eq!(
        retained["migrationRetained"]["reason"],
        "FILE_OUTSIDE_APPLICATION"
    );
    assert!(!task_plan::eligible(&retained, &[]));
    let refused = task_actions::continue_task(&mut store, "task-2", "", false)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(refused.contains("尚未转换"), "{refused}");
    drop(store);

    let partition = ProjectStore::open_partition(&restored.join("projects").join(&first), &first)?;
    let project: Value = partition.store().get("project", &first)?.unwrap();
    assert_eq!(project["path"].as_str(), partition.project_root().to_str());
    assert_eq!(project["unknown"]["old"], true);
    let converted: Value = partition.store().get("task", "task-0")?.unwrap();
    assert_eq!(converted["status"], "interrupted");
    assert_eq!(converted["workspace"], ".beaver/workspaces/task-0");
    drop(partition);

    // A rerun after an interruption before the receipt sees converted state and stays consistent.
    let again = project_migration_activation::stage(&backup, &restored)?;
    assert_eq!(again.summary()["retained_tasks_marked"], 1);
    assert_eq!(again.summary()["host_tasks_interrupted"], 0);
    assert_eq!(again.summary()["projects"][&first]["tasksInterrupted"], 0);
    drop(again);
    let store = host(&restored)?;
    let marker: Value = store.get("task", "task-2")?.unwrap();
    assert_eq!(marker["migrationRetained"], retained["migrationRetained"]);
    Ok(())
}

#[test]
fn partition_activation_refuses_foreign_archives_and_changed_registrations() -> Result<()> {
    let fixture = Fixture::new()?;
    let [first, _] = fixture.projects.clone();
    let (_, restored) = restore_and_partition(&fixture)?;
    let backup = fixture.temp.path().join("bundle");
    let store = host(&restored)?;
    let mut project: Value = store.get("project", &first)?.unwrap();
    project["path"] = json!(fixture.temp.path().join("moved-game"));
    store.put("project", &first, &project)?;
    drop(store);
    let changed = project_migration_activation::stage(&backup, &restored)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(changed.contains("registration changed"), "{changed}");
    assert!(restored.join(".beaver-migration-pending").is_file());

    let other = Fixture::new()?;
    let (_, other_restored) = restore_and_partition(&other)?;
    let foreign = project_migration_activation::stage(&backup, &other_restored)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(foreign.contains("does not match"), "{foreign}");
    Ok(())
}
