mod project_storage_support;
use anyhow::Result;
use beaver_core::{
    project_storage::{ProjectStore, ProjectStores},
    project_storage_layout,
    store::Store,
};
use project_storage_support::{copy, project, tree};
use serde_json::{json, Value};
use std::fs;

#[test]
fn project_records_survive_relocation_without_copying_host_secrets() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    host.put("secret", "provider", &json!({"cipher":"host-only"}))?;
    host.put("settings", "app", &json!({"codexPath":"machine-specific"}))?;
    let root = project(temp.path(), "project", "project-a")?;
    let owner = ProjectStore::open(&root, "project-a")?;
    let record = json!({"id":"task-a","projectId":"project-a","future":{"keep":true}});
    owner.store().put("task", "task-a", &record)?;
    owner
        .store()
        .event("task-a", "2026-09-16", "note", "project evidence")?;
    assert!(owner.store().list::<Value>("secret")?.is_empty());
    assert!(owner.store().list::<Value>("settings")?.is_empty());
    assert!(host.list::<Value>("task")?.is_empty());
    for name in project_storage_layout::DIRECTORIES {
        assert!(root.join(".beaver").join(name).is_dir());
    }
    assert!(!root.join(".beaver/beaver.sqlite").exists());
    assert!(!root.join(".beaver/.storage-pending").exists());
    drop(owner);
    let moved = temp.path().join("moved");
    fs::rename(&root, &moved)?;
    let reopened = ProjectStore::open(&moved, "project-a")?;
    assert_eq!(reopened.project_id(), "project-a");
    assert_eq!(
        reopened.store().get::<Value>("task", "task-a")?,
        Some(record)
    );
    assert_eq!(
        reopened.store().events("task-a")?[0].text,
        "project evidence"
    );
    assert_eq!(
        host.get::<Value>("secret", "provider")?.unwrap()["cipher"],
        "host-only"
    );
    Ok(())
}

#[test]
fn independent_projects_keep_locks_and_records_isolated_until_close() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let a = project(temp.path(), "a", "a")?;
    let b = project(temp.path(), "b", "b")?;
    let mut projects = ProjectStores::default();
    projects
        .open(&a, "a")?
        .store()
        .lock()
        .unwrap()
        .put("task", "same", &json!({"owner":"a"}))?;
    assert!(projects
        .open(&b, "b")?
        .store()
        .lock()
        .unwrap()
        .list::<Value>("task")?
        .is_empty());
    projects
        .open(&b, "b")?
        .store()
        .lock()
        .unwrap()
        .put("task", "same", &json!({"owner":"b"}))?;
    assert_eq!(
        projects
            .open(&a.join("."), "a")?
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "same")?
            .unwrap()["owner"],
        "a"
    );
    let before = tree(&a)?;
    let error = ProjectStore::open(&a, "a").err().unwrap().to_string();
    assert!(error.contains("另一个宿主"), "{error}");
    assert_eq!(tree(&a)?, before);
    assert!(ProjectStore::open(&b, "b").is_err());
    projects.close("a")?;
    let next_owner = ProjectStore::open(&a, "a")?;
    assert_eq!(
        next_owner.store().get::<Value>("task", "same")?.unwrap()["owner"],
        "a"
    );
    assert!(ProjectStore::open(&b, "b").is_err());
    drop(projects);
    assert_eq!(
        ProjectStore::open(&b, "b")?
            .store()
            .get::<Value>("task", "same")?
            .unwrap()["owner"],
        "b"
    );
    Ok(())
}

#[test]
fn copied_identity_requires_explicit_close_before_reassociation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "same-project")?;
    let destination = temp.path().join("copy");
    copy(&source, &destination)?;
    let mut projects = ProjectStores::default();
    projects.open(&source, "same-project")?;
    let before = tree(&destination)?;
    let error = projects
        .open(&destination, "same-project")
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("同一项目 ID"), "{error}");
    assert_eq!(tree(&destination)?, before);
    projects.close("same-project")?;
    assert_eq!(
        projects.open(&destination, "same-project")?.project_root(),
        fs::canonicalize(destination)?
    );
    Ok(())
}

#[test]
fn opens_committed_wal_content_and_preserves_unknown_manifest_fields() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "project")?;
    let manifest_path = source.join(".beaver/project.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["history"] = json!({"future":"preserved"});
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    let owner = ProjectStore::open(&source, "project")?;
    owner
        .store()
        .put("task", "wal-record", &json!({"state":"committed"}))?;
    assert!(fs::metadata(source.join(".beaver/project.sqlite-wal"))?.len() > 32);
    let destination = temp.path().join("recovered");
    copy(&source, &destination)?;
    let original = fs::read(destination.join(".beaver/project.json"))?;
    let recovered = ProjectStore::open(&destination, "project")?;
    assert_eq!(
        recovered
            .store()
            .get::<Value>("task", "wal-record")?
            .unwrap()["state"],
        "committed"
    );
    assert_eq!(
        fs::read(destination.join(".beaver/project.json"))?,
        original
    );
    assert!(owner.store().get::<Value>("task", "wal-record")?.is_some());
    Ok(())
}
