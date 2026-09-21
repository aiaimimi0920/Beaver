mod project_storage_support;

use anyhow::Result;
use beaver_core::{
    project_storage::{ProjectStore, ProjectStores},
    store::Store,
};
use project_storage_support::{copy, project, tree};
use serde_json::{json, Value};
use std::{fs, sync::Arc};

#[test]
fn detached_database_and_file_workers_retain_exclusive_ownership() -> Result<()> {
    let temp = tempfile::tempdir()?;
    for last in ["database", "files"] {
        let root = project(temp.path(), last, last)?;
        let mut projects = ProjectStores::default();
        let runtime = projects.open(&root, last)?;
        let store = runtime.store();
        let files = runtime.files();
        drop(runtime);
        assert!(projects.close(last).is_err());
        // Even a registry teardown cannot release ownership while a worker is active.
        drop(projects);
        assert!(ProjectStore::open(&root, last).is_err());
        if last == "database" {
            drop(files);
            assert!(ProjectStore::open(&root, last).is_err());
            std::thread::spawn(move || {
                store
                    .lock()
                    .unwrap()
                    .put("task", "worker", &json!({"saved": true}))
            })
            .join()
            .unwrap()?;
            assert_eq!(
                ProjectStore::open(&root, last)?
                    .store()
                    .get::<Value>("task", "worker")?,
                Some(json!({"saved": true}))
            );
        } else {
            drop(store);
            assert!(ProjectStore::open(&root, last).is_err());
            let source = root.clone();
            let snapshot = std::thread::spawn(move || files.capture(&source))
                .join()
                .unwrap()?;
            let reopened = ProjectStore::open(&root, last)?.into_runtime();
            reopened
                .files()
                .restore_copy(&snapshot, &temp.path().join("restored"))?;
            assert_eq!(
                fs::read(temp.path().join("restored/project.godot"))?,
                fs::read(root.join("project.godot"))?
            );
        }
    }
    Ok(())
}

#[test]
fn close_waits_for_handles_before_reassociation_to_a_copied_identity() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "same")?;
    let destination = temp.path().join("copy");
    copy(&source, &destination)?;
    let mut projects = ProjectStores::default();
    let runtime = projects.open(&source, "same")?;
    let again = projects.open(&source.join("."), "same")?;
    assert!(Arc::ptr_eq(&runtime.store(), &again.store()));
    assert!(Arc::ptr_eq(&runtime.files(), &again.files()));
    let store = runtime.store();
    let files = runtime.files();
    assert!(projects.close("same").is_err());
    drop(runtime);
    drop(again);
    assert!(projects.close("same").is_err());
    drop(store);
    assert!(projects.close("same").is_err());
    assert!(projects.open(&destination, "same").is_err());
    assert!(ProjectStore::open(&source, "same").is_err());
    drop(files);
    projects.close("same")?;
    projects.close("absent")?;
    assert_eq!(
        projects.open(&destination, "same")?.project_root(),
        fs::canonicalize(&destination)?
    );
    drop(ProjectStore::open(&source, "same")?);
    Ok(())
}

#[test]
fn project_snapshots_are_isolated_and_travel_with_the_database() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host_root = temp.path().join("host");
    let host = Store::open(&host_root)?;
    host.put("secret", "provider", &json!({"cipher":"host-only"}))?;
    let a = project(temp.path(), "a", "a")?;
    let b = project(temp.path(), "b", "b")?;
    fs::write(a.join("model.txt"), "accepted a")?;
    fs::write(b.join("model.txt"), "accepted b")?;
    let mut projects = ProjectStores::default();
    let runtime_a = projects.open(&a, "a")?;
    let runtime_b = projects.open(&b, "b")?;
    let snapshot = runtime_a.files().capture(&a)?;
    let other = runtime_b.files().capture(&b)?;
    let hash = &snapshot["model.txt"];
    assert_eq!(
        runtime_a.files().blob(hash)?,
        fs::canonicalize(&a)?
            .join(".beaver/content/blobs")
            .join(hash)
    );
    assert!(!runtime_b.files().blob(hash)?.exists());
    assert!(!runtime_a.files().blob(&other["model.txt"])?.exists());
    assert!(snapshot.keys().all(|path| !path.starts_with(".beaver/")));
    for id in ["bad", "../escape", &"G".repeat(64)] {
        assert!(runtime_a.files().blob(id).is_err());
    }
    runtime_a
        .store()
        .lock()
        .unwrap()
        .put("task", "frozen", &json!({"snapshot":snapshot}))?;
    assert!(host.list::<Value>("task")?.is_empty());
    assert!(runtime_a
        .store()
        .lock()
        .unwrap()
        .list::<Value>("secret")?
        .is_empty());
    assert!(runtime_b
        .store()
        .lock()
        .unwrap()
        .list::<Value>("task")?
        .is_empty());
    assert!(!host_root.join("blobs").exists());
    assert!(!a.join(".beaver/blobs").exists());
    fs::write(a.join("model.txt"), "changed after capture")?;
    drop(runtime_a);
    projects.close("a")?;
    let moved = temp.path().join("moved");
    fs::rename(&a, &moved)?;
    let reopened = projects.open(&moved, "a")?;
    let record = reopened
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", "frozen")?
        .unwrap();
    let frozen = serde_json::from_value(record["snapshot"].clone())?;
    let restored = temp.path().join("restored");
    reopened.files().restore_copy(&frozen, &restored)?;
    assert_eq!(
        fs::read_to_string(restored.join("model.txt"))?,
        "accepted a"
    );
    Ok(())
}

#[test]
fn cached_open_rechecks_manifest_and_pending_instead_of_returning_a_stale_route() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "project", "project")?;
    let mut projects = ProjectStores::default();
    let runtime = projects.open(&root, "project")?;
    let manifest = root.join(".beaver/project.json");
    let original = fs::read(&manifest)?;
    let mut altered: Value = serde_json::from_slice(&original)?;
    for (key, value) in [
        ("projectId", json!("different")),
        ("storageVersion", json!(99)),
    ] {
        altered[key] = value;
        fs::write(&manifest, serde_json::to_vec(&altered)?)?;
        let before = tree(&root)?;
        assert!(projects.open(&root, "project").is_err());
        assert_eq!(tree(&root)?, before);
        altered = serde_json::from_slice(&original)?;
    }
    fs::write(&manifest, &original)?;
    let pending = root.join(".beaver/.storage-pending");
    fs::write(&pending, "unfinished")?;
    assert!(projects.open(&root, "project").is_err());
    fs::remove_file(&pending)?;
    assert!(Arc::ptr_eq(
        &runtime.store(),
        &projects.open(&root, "project")?.store()
    ));
    let missing = root.join(".beaver/cache");
    fs::remove_dir(&missing)?;
    assert!(projects.open(&root, "project").is_err());
    assert!(!missing.exists());
    assert!(!root.join(".beaver/beaver.sqlite").exists());
    Ok(())
}
