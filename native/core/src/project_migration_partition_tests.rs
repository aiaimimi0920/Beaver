use super::{file, source, task, Fixture};
use crate::{
    data_backup, migration_bundle, project_migration_partition, project_storage::ProjectStore,
};
use anyhow::Result;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub(super) fn session_index(fixture: &Fixture, task: &str) -> Result<()> {
    let home = fixture.data.join("codex").join(task);
    fs::create_dir_all(home.join("sessions"))?;
    fs::write(home.join("sessions/old.jsonl"), "opaque history\n")?;
    let connection = Connection::open(home.join("state_5.sqlite"))?;
    connection.execute_batch(
        "CREATE TABLE threads(id TEXT, rollout_path, cwd, unknown TEXT);
         CREATE TABLE project_roots(path TEXT);",
    )?;
    connection.execute(
        "INSERT INTO threads VALUES('thread',?,?,'opaque')",
        params![
            source(fixture, &format!("codex/{task}/sessions/old.jsonl")).to_string_lossy(),
            source(fixture, &format!("workspaces/{task}")).to_string_lossy()
        ],
    )?;
    connection.execute(
        "INSERT INTO project_roots VALUES(?)",
        [fixture.temp.path().join("game-0").to_string_lossy()],
    )?;
    Ok(())
}

pub(super) fn restore_and_partition(fixture: &Fixture) -> Result<(Value, std::path::PathBuf)> {
    let backup = fixture.bundle()?;
    let before = data_backup::inventory(&backup)?;
    let restored = fixture.temp.path().join("restored");
    migration_bundle::restore(&backup, &restored)?;
    let receipt = migration_bundle::command(vec![
        "partition-projects".into(),
        backup.clone().into_os_string(),
        restored.clone().into_os_string(),
    ])?;
    assert_eq!(data_backup::inventory(&backup)?, before);
    assert_eq!(receipt["readyToActivate"], false);
    assert!(restored.join(".beaver-migration-pending").is_file());
    assert!(restored.join("PROJECT-MIGRATION.json").is_file());
    Ok((receipt, restored))
}

fn open_partition(restored: &Path, id: &str) -> Result<ProjectStore> {
    // Activation is not implemented here; clearing the marker only proves the partition is a
    // complete project store.
    let marker = restored.join(".beaver-migration-pending");
    if marker.exists() {
        fs::remove_file(marker)?;
    }
    ProjectStore::open(&restored.join("projects").join(id), id)
}

fn count(connection: &Connection, sql: &str) -> Result<i64> {
    Ok(connection.query_row(sql, [], |row| row.get(0))?)
}

#[test]
fn partition_moves_owned_history_into_project_stores_and_retains_flagged_tasks() -> Result<()> {
    let fixture = Fixture::new()?;
    let [first, second] = fixture.projects.clone();
    let hash = file(&fixture, "content.tmp")?;
    file(&fixture, &format!("blobs/{hash}"))?;
    for (index, project) in [&first, &second].into_iter().enumerate() {
        let id = format!("task-{index}");
        let mut value = task(&fixture, &id, project)?;
        value["baseline"] = json!({"mesh.dat":hash});
        fixture.store.put("task", &id, &value)?;
        file(&fixture, &format!("workspaces/{id}/notes.txt"))?;
    }
    session_index(&fixture, "task-0")?;
    let image = file(&fixture, "asset-observer/task-0/references/ref.png")?;
    fixture.store.put(
        "asset-reference",
        "ref",
        &json!({"id":"ref","taskId":"task-0","projectId":first,
            "imagePath":source(&fixture,"asset-observer/task-0/references/ref.png"),"sha256":image}),
    )?;
    let log = file(&fixture, "validation/run-0/run.log")?;
    fixture.store.put(
        "validationRun",
        "run-0",
        &json!({"id":"run-0","projectId":first,"evidence":[{"file":"run.log","sha256":log}]}),
    )?;
    fixture.store.event("task-0", "t1", "system", "started")?;
    fixture.store.event("task-0", "t2", "system", "finished")?;
    fixture.store.event("task-1", "t3", "system", "started")?;
    fixture.call(
        "c0",
        Some("task-0"),
        None,
        json!({"id":"c0","taskId":"task-0"}),
    )?;
    fixture.call(
        "c1",
        None,
        Some(&second),
        json!({"id":"c1","projectId":second}),
    )?;
    fixture.call("c2", None, None, json!({"id":"c2"}))?;
    fixture.store.put(
        "task",
        "task-2",
        &json!({"id":"task-2","projectId":second,
            "workspace":fixture.temp.path().join("elsewhere/task-2")}),
    )?;
    fixture.store.event("task-2", "t4", "system", "orphaned")?;
    fixture
        .store
        .put("settings", "main", &json!({"tools":{}}))?;

    let (receipt, restored) = restore_and_partition(&fixture)?;
    let counts = &receipt["projects"][&first];
    assert_eq!(counts["entities"], 4);
    assert_eq!(counts["events"], 2);
    assert_eq!(counts["calls"], 1);
    assert_eq!(counts["fieldsRewritten"], 2);
    assert_eq!(counts["sessionPathsRewritten"], 3);
    assert_eq!(receipt["projects"][&second]["entities"], 2);
    assert_eq!(receipt["projects"][&second]["calls"], 1);
    let retained = receipt["retained"].as_array().unwrap();
    assert!(retained.iter().any(|item| item["table"] == "entities"
        && item["id"] == "task-2"
        && item["reason"] == "FILE_OUTSIDE_APPLICATION"));
    assert!(retained
        .iter()
        .any(|item| item["table"] == "events" && item["reason"] == "TASK_RETAINED"));
    assert!(receipt["activationBlockers"].as_array().unwrap().len() == 2);
    assert!(!receipt.to_string().contains("file-payload-must-not-escape"));

    let host = Connection::open_with_flags(
        restored.join("data/beaver.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    assert_eq!(
        count(&host, "SELECT count(*) FROM entities WHERE kind='task'")?,
        1
    );
    assert_eq!(
        count(&host, "SELECT count(*) FROM entities WHERE kind='project'")?,
        2
    );
    assert_eq!(
        count(&host, "SELECT count(*) FROM entities WHERE kind='settings'")?,
        1
    );
    assert_eq!(count(&host, "SELECT count(*) FROM events")?, 1);
    assert_eq!(count(&host, "SELECT count(*) FROM calls")?, 1);
    drop(host);
    assert!(!restored.join("data/workspaces/task-0").exists());
    assert!(!restored.join("data/codex/task-0").exists());
    assert!(!restored.join("data/validation/run-0").exists());
    assert!(restored.join(format!("data/blobs/{hash}")).is_file());

    let store = open_partition(&restored, &first)?;
    let task_0: Value = store.store().get("task", "task-0")?.unwrap();
    assert_eq!(task_0["workspace"], ".beaver/workspaces/task-0");
    assert_eq!(task_0["baseline"]["mesh.dat"], hash);
    let project: Value = store.store().get("project", &first)?.unwrap();
    assert_eq!(project["unknown"]["old"], true);
    let reference: Value = store.store().get("asset-reference", "ref")?.unwrap();
    assert_eq!(
        reference["imagePath"],
        ".beaver/evidence/asset-observer/task-0/references/ref.png"
    );
    assert_eq!(store.store().events("task-0")?.len(), 2);
    assert_eq!(
        count(&store.store().connection, "SELECT count(*) FROM calls")?,
        1
    );
    let root = store.project_root().to_path_buf();
    let runtime = store.into_runtime();
    let files = runtime.files();
    let workspace = files.resolve_workspace("task-0", Path::new(".beaver/workspaces/task-0"))?;
    assert!(workspace.join("notes.txt").is_file());
    assert!(files.blob(&hash)?.is_file());
    assert!(root
        .join(".beaver/evidence/asset-observer/task-0/references/ref.png")
        .is_file());
    assert!(root.join(".beaver/evidence/run-0/run.log").is_file());
    let index = Connection::open_with_flags(
        root.join(".beaver/workspaces/.codex/task-0/state_5.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let (rollout, cwd): (String, String) =
        index.query_row("SELECT rollout_path,cwd FROM threads", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
    assert_eq!(
        Path::new(&rollout),
        root.join(".beaver/workspaces/.codex/task-0/sessions/old.jsonl")
    );
    assert_eq!(Path::new(&cwd), workspace);
    let game_root: String =
        index.query_row("SELECT path FROM project_roots", [], |row| row.get(0))?;
    assert_eq!(Path::new(&game_root), root);
    drop(index);
    drop(runtime);

    let second_store = open_partition(&restored, &second)?;
    assert!(second_store
        .store()
        .get::<Value>("task", "task-2")?
        .is_none());
    assert!(second_store.store().events("task-2")?.is_empty());
    assert!(second_store
        .project_root()
        .join(format!(".beaver/content/blobs/{hash}"))
        .is_file());
    Ok(())
}

#[test]
fn partition_refuses_converted_or_repeated_copies() -> Result<()> {
    let fixture = Fixture::new()?;
    let (_, restored) = restore_and_partition(&fixture)?;
    let backup = fixture.temp.path().join("bundle");
    let repeated = project_migration_partition::partition(&backup, &restored)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(repeated.contains("already been partitioned"), "{repeated}");
    let prepared = fixture.temp.path().join("prepared");
    migration_bundle::restore(&backup, &prepared)?;
    let mut restore_receipt: Value =
        serde_json::from_slice(&fs::read(prepared.join("RESTORE.json"))?)?;
    restore_receipt["paths_rewritten"] = json!(true);
    fs::write(
        prepared.join("RESTORE.json"),
        serde_json::to_vec(&restore_receipt)?,
    )?;
    let converted = project_migration_partition::partition(&backup, &prepared)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(converted.contains("unconverted restore"), "{converted}");
    Ok(())
}
