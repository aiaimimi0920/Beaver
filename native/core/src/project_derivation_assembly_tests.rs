use super::*;
use crate::{
    call_log,
    journal::{FileOperation, OperationKind, OperationState},
    project_derivation_validation_records::Rewrite,
    project_storage::ProjectStore,
    validation::{model::Run, repository, requests},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::PathBuf;

fn fixture(base: &Path, broken: bool) -> Result<PathBuf> {
    let source = base.join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("project.godot"), "config_version=5\n")?;
    fs::write(source.join("scene.txt"), "before")?;
    let project = ProjectStore::initialize(&source, "original")?;
    let runtime = project.into_runtime();
    let files = runtime.files();
    let handle = runtime.store();
    let store = handle.lock().unwrap();
    store.put(
        "project",
        "original",
        &json!({"id":"original","path":source,"unknown":"keep"}),
    )?;
    for (id, status) in [
        ("merge", "running"),
        ("queued", "queued"),
        ("parent", "waitingChildren"),
    ] {
        store.put(
            "task",
            id,
            &json!({"id":id,"projectId":"original","status":status,
            "workspace":format!(".beaver/workspaces/{id}")}),
        )?;
        fs::create_dir_all(source.join(format!(".beaver/workspaces/{id}")))?;
    }
    let before = files.capture(&source)?;
    fs::write(source.join("scene.txt"), "after")?;
    let after = files.capture(&source)?;
    fs::write(source.join("scene.txt"), "before")?;
    let operation = FileOperation {
        id: "merge-op".into(),
        project_id: "original".into(),
        task_id: "merge".into(),
        kind: OperationKind::Merge,
        state: OperationState::Applying,
        changes: Files::changes(&before, &after),
        task_after: json!({"id":"merge","projectId":"original","status":"completed",
            "workspace":".beaver/workspaces/merge"}),
    };
    if broken {
        let hash = operation.changes[0].after.as_ref().unwrap();
        fs::remove_file(source.join(".beaver/content/blobs").join(hash))?;
    }
    store.put("operation", "merge-op", &operation)?;
    call_log::begin(
        &store,
        "test",
        "test",
        Some("queued"),
        Some("original"),
        &json!({}),
    )?;
    store
        .connection
        .execute("UPDATE sqlite_sequence SET seq=99 WHERE name='calls'", [])?;
    let mut run = repository::build_run(
        &store,
        "original",
        Default::default(),
        None,
        Some("queued".into()),
        None,
    )?;
    run.status = "running".into();
    run.log = "partial diagnostic evidence".into();
    store.put("validationRun", &run.id, &run)?;
    let request =
        requests::Request::new("test", &json!({"projectId":"original","requestId":"saved"}))?;
    let (kind, id, value) = request.record(&json!({"runId":run.id}));
    store.put(kind, &id, &value)?;
    drop(store);
    drop(handle);
    drop(files);
    drop(runtime);
    let home = source.join(".beaver/workspaces/.codex/queued");
    fs::create_dir_all(home.join("sessions"))?;
    fs::write(
        home.join("sessions/history.jsonl"),
        b"opaque original queued\r\n",
    )?;
    let index = Connection::open(home.join("state_5.sqlite"))?;
    index.execute_batch("CREATE TABLE threads(id TEXT,rollout_path TEXT,cwd TEXT);")?;
    index.execute(
        "INSERT INTO threads VALUES(?,?,?)",
        params![
            "thread-unchanged",
            home.join("sessions/history.jsonl").to_str(),
            source.join(".beaver/workspaces/queued").to_str()
        ],
    )?;
    drop(index);
    let preparation = base.join("prepared");
    copy::prepare(
        copy::Request {
            request_id: "derive".into(),
            source,
            source_project_id: "original".into(),
            target_project_id: "derived".into(),
        },
        &preparation,
    )?;
    Ok(preparation)
}

#[test]
fn complete_assembly_recovers_only_target_and_preserves_source_and_preparation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    let source_before = data_backup::inventory(&temp.path().join("source"))?;
    let before = data_backup::inventory(&preparation)?;
    let destination = temp.path().join("assembled");
    let receipt = create(&preparation, &destination)?;
    assert_eq!(receipt.tasks_interrupted, 1);
    assert_eq!(receipt.session_paths_rewritten, 2);
    assert_eq!(
        source_before,
        data_backup::inventory(&temp.path().join("source"))?
    );
    assert_eq!(before, data_backup::inventory(&preparation)?);
    assert_eq!(
        fs::read_to_string(receipt.binding.join("scene.txt"))?,
        "after"
    );
    assert!(ProjectStore::open(&receipt.binding, "derived").is_err());
    assert_eq!(
        inspect(&preparation, &destination)?.entries,
        receipt.entries
    );
    let prepared = copy::inspect(&preparation)?;
    let queued = Rewrite(&prepared.identities).key("task", "queued")?.id;
    let home = receipt
        .binding
        .join(format!(".beaver/workspaces/.codex/{queued}"));
    assert!(!home.join("state_5.sqlite-wal").exists());
    assert!(!home.join("state_5.sqlite-shm").exists());
    assert_eq!(
        fs::read(home.join("sessions/history.jsonl"))?,
        b"opaque original queued\r\n"
    );
    let index = Connection::open(home.join("state_5.sqlite"))?;
    let (thread, rollout, cwd): (String, String, String) =
        index.query_row("SELECT id,rollout_path,cwd FROM threads", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    assert_eq!(thread, "thread-unchanged");
    assert_eq!(PathBuf::from(rollout), home.join("sessions/history.jsonl"));
    assert_eq!(
        PathBuf::from(cwd),
        receipt.binding.join(format!(".beaver/workspaces/{queued}"))
    );
    drop(index);
    let project = ProjectStore::open_partition(&receipt.binding, "derived")?;
    let store = project.store();
    assert_eq!(
        store.get::<Value>("task", &queued)?.unwrap()["status"],
        "interrupted"
    );
    let parent = Rewrite(&prepared.identities).key("task", "parent")?.id;
    assert_eq!(
        store.get::<Value>("task", &parent)?.unwrap()["planPaused"],
        true
    );
    assert_eq!(
        store.list::<FileOperation>("operation")?[0].state,
        OperationState::Complete
    );
    let run = &store.list::<Run>("validationRun")?[0];
    assert_eq!(run.status, "interrupted");
    assert_eq!(run.verdict, "needsReview");
    assert_eq!(run.log, "partial diagnostic evidence");
    assert!(store.list::<Value>("validationRequest")?.is_empty());
    let status: String = store.connection.query_row(
        "SELECT json_extract(value,'$.status') FROM calls",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(status, "interrupted");
    let sequence: i64 = store.connection.query_row(
        "SELECT seq FROM sqlite_sequence WHERE name='calls'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(sequence, 99);
    assert!(destination.join(ARCHIVE).is_file());
    Ok(())
}

#[test]
fn offline_retry_corruption_and_location_binding_are_enforced() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    fs::rename(temp.path().join("source"), temp.path().join("offline"))?;
    let moved = temp.path().join("moved-preparation");
    fs::rename(preparation, &moved)?;
    let destination = temp.path().join("assembled");
    create(&moved, &destination)?;
    inspect(&moved, &destination)?;
    let before = data_backup::inventory(&destination)?;
    assert!(create(&moved, &destination).is_err());
    assert_eq!(before, data_backup::inventory(&destination)?);
    fs::write(destination.join("project/scene.txt"), "tampered")?;
    assert!(inspect(&moved, &destination).is_err());
    let retry = temp.path().join("retry");
    create(&moved, &retry)?;
    let moved_target = temp.path().join("moved-target");
    fs::rename(retry, &moved_target)?;
    assert!(inspect(&moved, &moved_target).is_err());
    assert!(create(&moved, &moved.join("forbidden")).is_err());
    assert!(!moved.join("forbidden").exists());
    Ok(())
}

#[test]
fn unresolved_target_journal_retains_pending_and_never_publishes_receipt() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), true)?;
    let before = data_backup::inventory(&preparation)?;
    let destination = temp.path().join("failed");
    assert!(create(&preparation, &destination).is_err());
    assert!(destination.join(PENDING).is_file());
    assert!(!destination.join(RECEIPT).exists());
    assert_eq!(before, data_backup::inventory(&preparation)?);
    assert!(ProjectStore::open(&destination.join("project"), "derived").is_err());
    Ok(())
}

#[test]
fn activation_publishes_receipt_before_marker_removal_and_is_idempotent() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    let destination = temp.path().join("assembled");
    create(&preparation, &destination)?;

    let first = activate(&preparation, &destination)?;
    assert!(destination.join(ACTIVATION).is_file());
    assert!(!destination.join(PENDING).exists());
    assert_eq!(first["host_registration_changed"], false);
    assert!(ProjectStore::open(&destination.join("project"), "derived").is_ok());
    assert_eq!(inspect_activated(&preparation, &destination)?, first);

    let second = activate(&preparation, &destination)?;
    assert_eq!(second, first);
    assert_eq!(
        fs::read(destination.join(ACTIVATION))?,
        serde_json::to_vec_pretty(&first)?
    );
    Ok(())
}

#[test]
fn activation_receipt_with_pending_marker_is_recoverable_without_rewrite() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    let destination = temp.path().join("assembled");
    create(&preparation, &destination)?;
    let receipt = inspect(&preparation, &destination)?;
    let result = json!({
        "format": "beaver-project-derivation-activation-v1",
        "assembly": receipt,
        "binding": receipt.binding,
        "host_registration_changed": false,
    });
    write_new(
        &destination.join(ACTIVATION),
        &serde_json::to_vec_pretty(&result)?,
    )?;
    let before = fs::read(destination.join(ACTIVATION))?;
    assert_eq!(activate(&preparation, &destination)?, result);
    assert!(!destination.join(PENDING).exists());
    assert_eq!(fs::read(destination.join(ACTIVATION))?, before);
    assert!(inspect_activated(&preparation, &destination).is_ok());
    Ok(())
}

#[test]
fn activated_inspection_rejects_changed_receipt_fields_without_rewriting() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    let destination = temp.path().join("assembled");
    create(&preparation, &destination)?;
    let activation = activate(&preparation, &destination)?;
    for pointer in [
        "/format",
        "/assembly/projectId",
        "/assembly/preparationSha256",
        "/assembly/tasksInterrupted",
        "/host_registration_changed",
    ] {
        let mut changed = activation.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("tampered");
        let bytes = serde_json::to_vec_pretty(&changed)?;
        fs::write(destination.join(ACTIVATION), &bytes)?;
        assert!(
            inspect_activated(&preparation, &destination).is_err(),
            "accepted changed activation field: {pointer}"
        );
        assert!(activate(&preparation, &destination).is_err());
        assert_eq!(fs::read(destination.join(ACTIVATION))?, bytes);
    }
    fs::write(
        destination.join(ACTIVATION),
        serde_json::to_vec_pretty(&activation)?,
    )?;
    assert_eq!(inspect_activated(&preparation, &destination)?, activation);
    Ok(())
}

#[test]
fn activation_holds_exclusive_project_ownership_and_validates_lock_contents() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = fixture(temp.path(), false)?;
    let destination = temp.path().join("assembled");
    let receipt = create(&preparation, &destination)?;
    let control = receipt.binding.join(layout::CONTROL_DIR);
    let busy = project_storage::lock(&control, false)?;
    assert!(activate(&preparation, &destination).is_err());
    assert!(destination.join(PENDING).is_file());
    assert!(!destination.join(ACTIVATION).exists());
    drop(busy);
    fs::write(control.join(layout::LOCK), "modified")?;
    assert!(activate(&preparation, &destination).is_err());
    fs::write(control.join(layout::LOCK), "")?;
    activate(&preparation, &destination)?;
    let active = ProjectStore::open(&receipt.binding, "derived")?;
    assert!(inspect_activated(&preparation, &destination).is_err());
    drop(active);
    inspect_activated(&preparation, &destination)?;
    Ok(())
}
