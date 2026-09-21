use super::*;
use crate::project_storage::ProjectStore;
use serde_json::json;

fn request(source: &Path) -> Request {
    Request {
        request_id: "copy-request-1".into(),
        source: source.to_owned(),
        source_project_id: "original".into(),
        target_project_id: "independent".into(),
    }
}

fn source(root: &Path) -> Result<()> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let store = ProjectStore::initialize(root, "original")?;
    store
        .store()
        .put("project", "original", &json!({"id":"original","path":root}))?;
    store.store().put(
        "task",
        "queued",
        &json!({"id":"queued","projectId":"original","status":"queued"}),
    )?;
    fs::create_dir(root.join(".beaver/workspaces/queued"))?;
    fs::write(
        root.join(".beaver/workspaces/queued/evidence.txt"),
        "frozen content",
    )?;
    Ok(())
}

#[test]
fn preserves_committed_wal_and_recovers_preparation_without_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let original = temp.path().join("original");
    source(&original)?;
    let owner = ProjectStore::open(&original, "original")?;
    owner.store().put(
        "project",
        "original",
        &json!({"id":"original","name":"committed in WAL"}),
    )?;
    let portable = temp.path().join("portable");
    fs::create_dir(&portable)?;
    for entry in data_backup::inventory_without(&original, EXCLUDED)? {
        let target = portable.join(&entry.path);
        if entry.sha256.is_none() {
            fs::create_dir(target)?;
        } else {
            fs::copy(original.join(&entry.path), target)?;
        }
    }
    fs::write(portable.join(".beaver/.project.lock"), [])?;
    assert!(fs::metadata(portable.join(".beaver/project.sqlite-wal"))?.len() > 32);
    let before = data_backup::inventory(&portable)?;
    let destination = temp.path().join("derived");
    let prepared = prepare(request(&portable), &destination)?;
    assert_eq!(before, data_backup::inventory(&portable)?);
    assert_eq!(prepared.request.target_project_id, "independent");
    let copy = destination.join("project");
    assert!(ProjectStore::open(&copy, "original").is_err());
    assert!(ProjectStore::read_project(&copy, "original").is_err());
    let manifest = layout::manifest_in(&copy, Some("original"))?;
    let snapshot = database::snapshot(&copy.join(".beaver"), &manifest)?;
    assert_eq!(
        snapshot.project("original")?.unwrap()["name"],
        "committed in WAL"
    );
    fs::rename(&portable, temp.path().join("source-offline"))?;
    assert_eq!(inspect(&destination)?.request, prepared.request);
    assert_eq!(inspect(&destination)?.identities, prepared.identities);
    assert!(destination.join(PENDING).exists());
    Ok(())
}

#[test]
fn refuses_busy_sources_and_raw_database_writers_before_creating_target() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("source");
    source(&root)?;
    let target = temp.path().join("target");
    let owner = ProjectStore::open(&root, "original")?;
    assert!(prepare(request(&root), &target).is_err());
    assert!(!target.exists());
    drop(owner);
    let before = data_backup::inventory(&root)?;
    let writer = OpenOptions::new()
        .write(true)
        .open(root.join(".beaver/project.sqlite"))?;
    assert!(prepare(request(&root), &target).is_err());
    assert!(!target.exists());
    drop(writer);
    assert_eq!(before, data_backup::inventory(&root)?);
    Ok(())
}

#[test]
fn rejects_identity_and_destination_conflicts_without_overwrite() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("source");
    source(&root)?;
    let target = temp.path().join("target");
    let mut same = request(&root);
    same.target_project_id = same.source_project_id.clone();
    assert!(prepare(same, &target).is_err());
    assert!(!target.exists());
    assert!(prepare(request(&root), &root.join("nested")).is_err());
    assert!(!root.join("nested").exists());
    prepare(request(&root), &target)?;
    let before = data_backup::inventory(&target)?;
    assert!(prepare(request(&root), &target).is_err());
    assert_eq!(before, data_backup::inventory(&target)?);
    Ok(())
}

#[test]
fn incomplete_or_changed_preparations_remain_blocked() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("source");
    source(&root)?;
    let target = temp.path().join("target");
    prepare(request(&root), &target)?;
    let receipt = fs::read(target.join(RECEIPT))?;
    fs::remove_file(target.join(RECEIPT))?;
    assert!(inspect(&target).is_err());
    assert!(ProjectStore::open(&target.join("project"), "original").is_err());
    fs::write(target.join(RECEIPT), receipt)?;
    fs::write(
        target.join("project/.beaver/workspaces/queued/evidence.txt"),
        "changed",
    )?;
    let before = data_backup::inventory(&target)?;
    assert!(inspect(&target).is_err());
    assert_eq!(before, data_backup::inventory(&target)?);
    assert!(target.join(PENDING).exists());
    assert!(ProjectStore::open(&target.join("project"), "original").is_err());
    Ok(())
}

#[test]
fn rejects_dangling_task_references_without_touching_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("source");
    source(&root)?;
    {
        let owner = ProjectStore::open(&root, "original")?;
        owner.store().put(
            "task",
            "queued",
            &json!({
                "id":"queued", "projectId":"original", "dependsOn":["missing"]
            }),
        )?;
    }
    let before = data_backup::inventory(&root)?;
    let target = temp.path().join("target");
    let error = prepare(request(&root), &target).unwrap_err().to_string();
    assert!(error.contains("REFERENCE_NOT_FOUND"), "{error}");
    assert!(!target.exists());
    assert_eq!(before, data_backup::inventory(&root)?);
    Ok(())
}

#[test]
fn refuses_tampered_identity_maps_without_rewriting_copy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("source");
    source(&root)?;
    let target = temp.path().join("target");
    prepare(request(&root), &target)?;
    let mut receipt: serde_json::Value = serde_json::from_slice(&fs::read(target.join(RECEIPT))?)?;
    receipt["identities"]["entities"] = json!([]);
    fs::write(target.join(RECEIPT), serde_json::to_vec(&receipt)?)?;
    let before = data_backup::inventory(&target)?;
    assert!(inspect(&target)
        .unwrap_err()
        .to_string()
        .contains("identity map"));
    assert_eq!(before, data_backup::inventory(&target)?);
    assert!(target.join(PENDING).exists());
    Ok(())
}
