use super::*;
use crate::{project_storage::ProjectStore, validation::requests};
use serde_json::json;

fn prepare(root: &Path) -> Result<PathBuf> {
    let source = root.join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("project.godot"), "config_version=5\n")?;
    let project = ProjectStore::initialize(&source, "original")?;
    project.store().put(
        "project",
        "original",
        &json!({"id":"original","path":source}),
    )?;
    project.store().put(
        "task",
        "task",
        &json!({"id":"task","projectId":"original","status":"queued",
            "workspace":".beaver/workspaces/task",
            "assetRestore":".beaver/workspaces/.codex/task/asset-checkpoints/saved.blend"}),
    )?;
    let request = requests::Request::new(
        "test",
        &json!({"projectId":"original","requestId":"request"}),
    )?;
    let (kind, key, value) = request.record(&json!({"id":"task"}));
    project.store().put(kind, &key, &value)?;
    project.store().put(
        "asset-reference",
        "reference",
        &json!({
            "id":"reference","taskId":"task","projectId":"original",
            "imagePath":".beaver/evidence/asset-observer/task/references/reference.png",
            "sha256":format!("{:x}", Sha256::digest(b"opaque source task")),"used":false,
            "frame":{"id":"frame","sessionId":"session","generation":"generation",
                "sceneRevision":1,"viewRevision":2,"capturedAt":3,"width":100,"height":100,
                "viewMatrix":vec![0.0;16],"projectionMatrix":vec![0.0;16]},"pick":null
        }),
    )?;
    drop(project);
    for path in [
        ".beaver/workspaces/task/scene.tscn",
        ".beaver/workspaces/.codex/task/asset-checkpoints/saved.blend",
        ".beaver/evidence/asset-observer/task/references/reference.png",
    ] {
        let file = source.join(path);
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(file, b"opaque source task")?;
    }
    let destination = root.join("prepared");
    copy::prepare(
        copy::Request {
            request_id: "copy".into(),
            source,
            source_project_id: "original".into(),
            target_project_id: "derived".into(),
        },
        &destination,
    )?;
    Ok(destination)
}

#[test]
fn durable_stage_recovers_after_move_without_source_and_keeps_copy_pending() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let prepared = prepare(temp.path())?;
    let before = data_backup::inventory(&prepared.join("project"))?;
    let receipt = create(&prepared, "one")?;
    assert_eq!(receipt.entries.len(), 2);
    assert_eq!(before, data_backup::inventory(&prepared.join("project"))?);
    assert!(ProjectStore::open(&prepared.join("project"), "original").is_err());
    fs::rename(
        temp.path().join("source"),
        temp.path().join("offline-source"),
    )?;
    let moved = temp.path().join("moved");
    fs::rename(&prepared, &moved)?;
    assert_eq!(inspect(&moved, "one")?.entries, receipt.entries);
    assert!(moved.join(".beaver-migration-pending").is_file());
    let generation = directory(&moved, "one")?;
    let archive: Archive = serde_json::from_slice(&fs::read(generation.join(ARCHIVE))?)?;
    assert_eq!(archive.records.len(), 1);
    assert_eq!(archive.source_project_id, "original");
    let db =
        Connection::open_with_flags(generation.join(DATABASE), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let project: String =
        db.query_row("SELECT id FROM entities WHERE kind='project'", [], |row| {
            row.get(0)
        })?;
    assert_eq!(project, "derived");
    crate::project_derivation_files::create(&moved, "one")?;
    let raw: String = db.query_row("SELECT value FROM entities WHERE kind='task'", [], |row| {
        row.get(0)
    })?;
    let task: serde_json::Value = serde_json::from_str(&raw)?;
    let root = moved.join("files-stage-one/project");
    let files = crate::files::Files::project(
        root,
        std::sync::Arc::new(fs::File::open(moved.join(".beaver-migration-pending"))?),
    );
    let id = task["id"].as_str().unwrap();
    let workspace = files.resolve_workspace(id, Path::new(task["workspace"].as_str().unwrap()))?;
    assert_eq!(
        fs::read(workspace.join("scene.tscn"))?,
        b"opaque source task"
    );
    assert_eq!(
        fs::read(files.resolve_checkpoint(task["assetRestore"].as_str().unwrap())?)?,
        b"opaque source task"
    );
    let raw: String = db.query_row(
        "SELECT value FROM entities WHERE kind='asset-reference'",
        [],
        |row| row.get(0),
    )?;
    let reference: crate::asset_task::Reference = serde_json::from_str(&raw)?;
    assert_eq!(reference.task_id, id);
    assert_eq!(reference.project_id, "derived");
    assert_eq!(
        fs::read(files.resolve_observer_reference(id, &reference.id, &reference.image_path)?)?,
        b"opaque source task"
    );
    Ok(())
}

#[test]
fn declared_paths_validate_inventory_and_preserve_unrelated_payloads() -> Result<()> {
    use crate::project_derivation_record_paths::Records;
    let temp = tempfile::tempdir()?;
    let preparation = prepare(temp.path())?;
    let mut prepared = copy::inspect(&preparation)?;
    let checkpoint = ".beaver/workspaces/.codex/task/asset-checkpoints/saved.blend";
    let mut trace = json!({"context":{"checkpoint":checkpoint},"output":{"checkpoint":checkpoint}});
    Records(&prepared).rewrite("framework-trace/task", "call", &mut trace)?;
    assert_ne!(trace["context"]["checkpoint"], checkpoint);
    assert_eq!(trace["output"]["checkpoint"], checkpoint);
    let mut bad = json!({"id":"task","workspace":".beaver/workspaces/task/scene.tscn"});
    assert!(Records(&prepared)
        .rewrite("task", "task", &mut bad)
        .is_err());
    let mut missing =
        json!({"assetRestore":".beaver/workspaces/.codex/task/asset-checkpoints/missing.blend"});
    assert!(Records(&prepared)
        .rewrite("task", "task", &mut missing)
        .is_err());
    let mut unsafe_path =
        json!({"assetRestore":".beaver/workspaces/.codex/task/asset-checkpoints/../saved.blend"});
    assert!(Records(&prepared)
        .rewrite("task", "task", &mut unsafe_path)
        .is_err());
    let mut wrong_type = json!({"assetRestore":checkpoint});
    prepared
        .entries
        .iter_mut()
        .find(|entry| entry.path == checkpoint)
        .unwrap()
        .sha256 = None;
    assert!(Records(&prepared)
        .rewrite("task", "task", &mut wrong_type)
        .is_err());
    let mut reference = json!({"id":"reference","taskId":"task","projectId":"original",
        "imagePath":".beaver/evidence/asset-observer/task/references/reference.png",
        "sha256":"incorrect"});
    assert!(Records(&prepared)
        .rewrite("asset-reference", "reference", &mut reference)
        .is_err());
    reference["sha256"] = json!(format!("{:x}", Sha256::digest(b"opaque source task")));
    reference["projectId"] = json!("foreign");
    assert!(Records(&prepared)
        .rewrite("asset-reference", "reference", &mut reference)
        .is_err());
    Ok(())
}

#[test]
fn incomplete_or_corrupt_generations_are_preserved_and_retries_use_new_names() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let prepared = prepare(temp.path())?;
    create(&prepared, "partial")?;
    let partial = directory(&prepared, "partial")?;
    fs::remove_file(partial.join(RECEIPT))?;
    let before = data_backup::inventory(&partial)?;
    assert!(inspect(&prepared, "partial").is_err());
    assert!(create(&prepared, "partial").is_err());
    assert_eq!(before, data_backup::inventory(&partial)?);
    create(&prepared, "retry")?;
    inspect(&prepared, "retry")?;
    let retry = directory(&prepared, "retry")?;
    fs::write(retry.join(ARCHIVE), "{}")?;
    assert!(inspect(&prepared, "retry").is_err());
    create(&prepared, "database")?;
    fs::write(directory(&prepared, "database")?.join(DATABASE), "corrupt")?;
    assert!(inspect(&prepared, "database").is_err());
    assert!(create(&prepared, "../escape").is_err());
    copy::inspect(&prepared)?;
    Ok(())
}

#[test]
fn receipt_cannot_be_rebound_to_a_different_preparation() -> Result<()> {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let original = prepare(first.path())?;
    let different = prepare(second.path())?;
    create(&original, "one")?;
    create(&different, "one")?;
    fs::copy(
        directory(&original, "one")?.join(RECEIPT),
        directory(&different, "one")?.join(RECEIPT),
    )?;
    assert!(inspect(&different, "one").is_err());
    inspect(&original, "one")?;
    Ok(())
}
