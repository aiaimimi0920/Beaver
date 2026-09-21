#![cfg(windows)]
use crate::{data_backup, migration_bundle, project_migration_inventory, store::Store};
use anyhow::Result;
use rusqlite::params;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

#[path = "project_migration_closure_tests.rs"]
mod closure;
#[path = "project_migration_file_tests.rs"]
mod file_inventory;

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    projects: [String; 2],
    store: Store,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("host");
        let store = Store::open(&data)?;
        store
            .connection
            .execute_batch("PRAGMA wal_autocheckpoint=0;")?;
        let projects = std::array::from_fn(|_| uuid::Uuid::new_v4().to_string());
        for (index, id) in projects.iter().enumerate() {
            let path = temp.path().join(format!("game-{index}"));
            fs::create_dir(&path)?;
            fs::write(path.join("project.godot"), "config_version=5")?;
            store.put(
                "project",
                id,
                &json!({"id":id,"path":path,"unknown":{"old":true}}),
            )?;
            let task = format!("task-{index}");
            store.put("task", &task, &json!({"id":task,"projectId":id}))?;
        }
        Ok(Self {
            temp,
            data,
            projects,
            store,
        })
    }

    fn bundle(&self) -> Result<PathBuf> {
        // Fixture-only frozen copy: no writes occur while copying the committed WAL.
        let offline = self.temp.path().join("offline-host");
        fs::create_dir(&offline)?;
        let entries = data_backup::inventory(&self.data)?;
        data_backup::copy_entries(&self.data, &offline, &entries)?;
        assert!(fs::metadata(offline.join("beaver.sqlite-wal"))?.len() > 0);
        let backup = self.temp.path().join("bundle");
        migration_bundle::create(&offline, &backup)?;
        Ok(backup)
    }

    fn call(
        &self,
        id: &str,
        task: Option<&str>,
        project: Option<&str>,
        value: Value,
    ) -> Result<()> {
        self.store.connection.execute(
            "INSERT INTO calls(id,task,project,method,value) VALUES(?,?,?,?,?)",
            params![id, task, project, "example", value.to_string()],
        )?;
        Ok(())
    }
}

fn record<'a>(report: &'a Value, table: &str, kind: Option<&str>, id: &str) -> &'a Value {
    report["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["table"] == table && record["kind"].as_str() == kind && record["id"] == id
        })
        .expect("inventory row")
}

#[test]
fn full_history_wal_and_taskless_validation_stay_project_owned_without_exposing_payloads(
) -> Result<()> {
    let mut fixture = Fixture::new()?;
    let project = fixture.projects[0].clone();
    fixture
        .store
        .put("secret", "code", &"private-credential-sentinel")?;
    fixture.store.put(
        "settings",
        "main",
        &json!({"private":"host-setting-sentinel"}),
    )?;
    fixture
        .store
        .put("validationSettings", &project, &json!({"revision":1}))?;
    fixture.store.put(
        "validationRelease",
        "release",
        &json!({"projectId":project,"future":42}),
    )?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({"projectId":project,"taskId":null}),
    )?;
    fixture.store.transaction(|db| {
        for sequence in 1..=701 {
            db.execute("INSERT INTO events(seq,task,time,kind,text) VALUES(?,'task-0','now','user','event-secret-sentinel')", [sequence])?;
        }
        Ok(())
    })?;
    fixture.call(
        "task-call",
        Some("task-0"),
        None,
        json!({"id":"task-call","taskId":"task-0","projectId":null}),
    )?;
    fixture.call(
        "project-call",
        None,
        Some(&project),
        json!({"id":"project-call","taskId":null,"projectId":project}),
    )?;
    fixture.call(
        "host-call",
        None,
        None,
        json!({"id":"host-call","taskId":null,"projectId":null}),
    )?;
    let backup = fixture.bundle()?;
    let before = data_backup::inventory(&backup)?;
    for index in 0..2 {
        fs::rename(
            fixture.temp.path().join(format!("game-{index}")),
            fixture.temp.path().join(format!("unavailable-{index}")),
        )?;
    }
    let report = migration_bundle::command(vec![
        "inspect-projects".into(),
        backup.as_os_str().to_owned(),
    ])?;
    assert_eq!(report["projects"][&project]["events"], 701);
    assert_eq!(report["projects"][&project]["calls"], 2);
    assert_eq!(report["host"]["calls"], 1);
    assert_eq!(report["host"]["entities"], 2);
    assert_eq!(
        report["unresolved"],
        json!({"entities":0,"events":0,"calls":0})
    );
    assert_eq!(report["readyToActivate"], false);
    for kind in ["validationRun", "validationRelease", "validationSettings"] {
        let id = match kind {
            "validationRun" => "run",
            "validationRelease" => "release",
            _ => &project,
        };
        assert_eq!(
            record(&report, "entities", Some(kind), id)["projectId"],
            project
        );
    }
    let text = report.to_string();
    for secret in [
        "private-credential-sentinel",
        "host-setting-sentinel",
        "event-secret-sentinel",
    ] {
        assert!(!text.contains(secret));
    }
    assert_eq!(data_backup::inventory(&backup)?, before);
    Ok(())
}

#[test]
fn conflicting_or_unrecoverable_ownership_is_reported_and_never_guessed() -> Result<()> {
    let fixture = Fixture::new()?;
    let [project, other] = &fixture.projects;
    fixture.store.put(
        "task",
        "same-project-task",
        &json!({"id":"same-project-task","projectId":project}),
    )?;
    fixture.store.put(
        "asset-task",
        "task-0",
        &json!({"taskId":"task-0","projectId":other}),
    )?;
    fixture.store.put(
        "framework-check/task-0",
        "wrong-task",
        &json!({"taskId":"same-project-task"}),
    )?;
    fixture.store.put("operation", "operation", &json!({"taskId":"task-0","projectId":project,"taskAfter":{"id":"task-0","projectId":other}}))?;
    fixture.store.put(
        "validationRequest",
        "legacy-receipt",
        &json!({"hash":"opaque","result":{"revision":1}}),
    )?;
    fixture
        .store
        .put("future-record", "unknown", &json!({"projectId":project}))?;
    fixture.store.put(
        "asset-reference",
        "dangling",
        &json!({"taskId":"missing","projectId":project}),
    )?;
    fixture.store.put(
        "feature",
        &format!("{other}:feature"),
        &json!({"taskId":"task-0"}),
    )?;
    fixture.store.put(
        "validationFlowRevision",
        "bogus",
        &json!({"id":"flow-digest","projectId":project,"revision":4}),
    )?;
    fixture
        .store
        .connection
        .execute("INSERT INTO entities VALUES('broken','json','{')", [])?;
    fixture.store.connection.execute(
        "INSERT INTO events(task,time,kind,text) VALUES('missing','now','user','payload')",
        [],
    )?;
    fixture.call(
        "columns",
        Some("task-0"),
        None,
        json!({"id":"columns","taskId":"task-1","projectId":null}),
    )?;
    fixture.call(
        "owners",
        Some("task-0"),
        Some(other),
        json!({"id":"owners","taskId":"task-0","projectId":other}),
    )?;
    let backup = fixture.bundle()?;
    let before = data_backup::inventory(&backup)?;
    let report = serde_json::to_value(project_migration_inventory::inspect(&backup)?)?;
    assert_eq!(
        report["unresolved"],
        json!({"entities":9,"events":1,"calls":2})
    );
    for (kind, id, reason) in [
        ("asset-task", "task-0", "PROJECT_OWNER_MISMATCH"),
        (
            "framework-check/task-0",
            "wrong-task",
            "TASK_OWNER_MISMATCH",
        ),
        (
            "validationRequest",
            "legacy-receipt",
            "VALIDATION_REQUEST_OWNER_UNKNOWN",
        ),
        ("future-record", "unknown", "UNKNOWN_ENTITY_KIND"),
        ("validationFlowRevision", "bogus", "ENTITY_ID_MISMATCH"),
        ("broken", "json", "INVALID_JSON"),
    ] {
        let entry = record(&report, "entities", Some(kind), id);
        assert_eq!(entry["scope"], "unresolved");
        assert_eq!(entry["reason"], reason);
        assert!(entry.get("projectId").is_none());
    }
    assert_eq!(data_backup::inventory(&backup)?, before);
    Ok(())
}

#[test]
fn dynamic_receipts_and_project_revision_keys_use_explicit_ownership() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    for prefix in [
        "asset-delivery",
        "asset-delivery-decisions",
        "task-callback",
        "framework-trace",
        "framework-observation",
        "framework-recovery",
        "framework-check",
        "framework-judgment",
        "framework-judgment-history",
    ] {
        fixture.store.put(
            &format!("{prefix}/task-0"),
            "item",
            &json!({"context":{"taskId":"task-0","projectId":project}}),
        )?;
    }
    fixture.store.put("task-callback-revision", "task-0", &7)?;
    fixture.store.put(
        "validationFlowRevision",
        "flow-digest:4",
        &json!({"id":"flow-digest","projectId":project,"revision":4}),
    )?;
    fixture
        .store
        .put("validationManifest", project, &json!({"hash":"digest"}))?;
    let backup = fixture.bundle()?;
    let report = serde_json::to_value(project_migration_inventory::inspect(&backup)?)?;
    assert_eq!(report["unresolved"]["entities"], 0);
    assert_eq!(report["projects"][project]["entities"], 14);
    assert_eq!(
        record(
            &report,
            "entities",
            Some("validationFlowRevision"),
            "flow-digest:4"
        )["projectId"],
        *project
    );
    fs::write(
        backup.join("projects").join(project).join("project.godot"),
        "tampered",
    )?;
    let before = data_backup::inventory(&backup)?;
    assert!(project_migration_inventory::inspect(&backup).is_err());
    assert_eq!(data_backup::inventory(&backup)?, before);
    Ok(())
}
