mod project_storage_support;
use anyhow::Result;
use beaver_core::{project_storage::ProjectStore, project_storage_layout::read_manifest};
use project_storage_support::{copy, project, tree};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::fs;

#[test]
fn invalid_databases_are_rejected_without_schema_repair_or_source_writes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "project")?;
    for (index, sql, expected) in [
        (0, "PRAGMA user_version=2", "版本"),
        (1, "UPDATE project_identity SET project_id='other'", "身份"),
        (2, "DROP TABLE events", "缺少结构"),
        (
            3,
            "CREATE TRIGGER surprise AFTER INSERT ON entities BEGIN DELETE FROM events; END",
            "未知结构",
        ),
        (4, "PRAGMA journal_mode=DELETE", "日志模式"),
        (
            5,
            "DROP TABLE entities; CREATE TABLE entities(kind TEXT,id TEXT,value TEXT)",
            "结构不匹配",
        ),
        (6, "PRAGMA application_id=0", "版本"),
    ] {
        let root = temp.path().join(format!("case-{index}"));
        copy(&source, &root)?;
        let db = root.join(".beaver/project.sqlite");
        let connection = Connection::open(&db)?;
        connection.execute_batch(sql)?;
        drop(connection);
        let before = tree(&root)?;
        let error = ProjectStore::open(&root, "project").err().unwrap();
        assert!(
            format!("{error:#}").contains(expected),
            "case {index}: {error:#}"
        );
        assert_eq!(tree(&root)?, before, "case {index}");
    }
    let corrupt = temp.path().join("corrupt");
    copy(&source, &corrupt)?;
    fs::write(corrupt.join(".beaver/project.sqlite"), b"not sqlite")?;
    let before = tree(&corrupt)?;
    assert!(ProjectStore::open(&corrupt, "project").is_err());
    assert_eq!(tree(&corrupt)?, before);
    Ok(())
}

#[test]
fn incomplete_storage_never_creates_missing_files_or_falls_back() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "project")?;
    for name in [
        "project.json",
        "project.sqlite",
        ".project.lock",
        "content",
        "evidence",
    ] {
        let root = temp.path().join(name);
        copy(&source, &root)?;
        let target = root.join(".beaver").join(name);
        if target.is_dir() {
            fs::remove_dir(target)?;
        } else {
            fs::remove_file(target)?;
        }
        let before = tree(&root)?;
        assert!(ProjectStore::open(&root, "project").is_err(), "{name}");
        assert_eq!(tree(&root)?, before, "{name}");
    }
    for name in [".storage-pending", "project.sqlite-journal"] {
        let root = temp.path().join(name);
        copy(&source, &root)?;
        fs::write(root.join(".beaver").join(name), b"interrupted")?;
        let before = tree(&root)?;
        assert!(ProjectStore::open(&root, "project").is_err(), "{name}");
        assert_eq!(tree(&root)?, before, "{name}");
    }
    let offline = temp.path().join("offline");
    assert!(ProjectStore::open(&offline, "project").is_err());
    assert!(!offline.exists());
    Ok(())
}

#[test]
fn manifest_gates_run_before_database_access() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "source", "project")?;
    let path = root.join(".beaver/project.json");
    let valid: Value = serde_json::from_slice(&fs::read(&path)?)?;
    for replacement in [
        b"not-json".to_vec(),
        vec![b' '; 65537],
        json!({"schemaVersion":2,"storageVersion":1,"projectId":"project"})
            .to_string()
            .into_bytes(),
        json!({"schemaVersion":1,"storageVersion":9,"projectId":"project"})
            .to_string()
            .into_bytes(),
        json!({"schemaVersion":1,"storageVersion":1,"projectId":"../escape"})
            .to_string()
            .into_bytes(),
    ] {
        fs::write(&path, replacement)?;
        let before = tree(&root)?;
        assert!(read_manifest(&root, None).is_err());
        assert!(ProjectStore::open(&root, "project").is_err());
        assert_eq!(tree(&root)?, before);
    }
    fs::write(&path, serde_json::to_vec(&valid)?)?;
    let before = tree(&root)?;
    assert!(ProjectStore::open(&root, "other").is_err());
    assert_eq!(tree(&root)?, before);
    Ok(())
}

#[test]
fn initialization_preserves_existing_directories_and_pending_migration_copies() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "existing", "project")?;
    let before = tree(&root)?;
    assert!(ProjectStore::initialize(&root, "replacement").is_err());
    assert_eq!(tree(&root)?, before);
    let unfinished = temp.path().join("unfinished");
    fs::create_dir(&unfinished)?;
    fs::write(unfinished.join("project.godot"), b"engine")?;
    fs::create_dir(unfinished.join(".beaver"))?;
    fs::write(unfinished.join(".beaver/diagnostic"), b"keep original")?;
    let before = tree(&unfinished)?;
    assert!(ProjectStore::initialize(&unfinished, "project").is_err());
    assert_eq!(tree(&unfinished)?, before);
    fs::write(
        temp.path().join(".beaver-migration-pending"),
        b"not activated",
    )?;
    let before = tree(temp.path())?;
    assert!(ProjectStore::open(&root, "project").is_err());
    assert!(read_manifest(&root, None).is_err());
    assert!(ProjectStore::initialize(&unfinished, "project").is_err());
    assert_eq!(tree(temp.path())?, before);
    Ok(())
}
