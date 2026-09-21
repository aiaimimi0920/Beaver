use super::*;
use crate::{project_derivation_session_paths as paths, project_storage::ProjectStore};
use rusqlite::{params, Connection};
use serde_json::json;

const INDEX: &str = ".beaver/workspaces/.codex/task/state_5.sqlite";
const ROLLOUT: &str = ".beaver/workspaces/.codex/task/sessions/history.jsonl";
const HISTORY: &[u8] = b"{\"cwd\":\"C:/old/opaque\",\"taskId\":\"task\"}\r\n";

fn prepare(
    root: &Path,
    change: impl FnOnce(&Connection, &Path) -> Result<()>,
) -> Result<(PathBuf, Connection)> {
    let source = root.join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("project.godot"), "config_version=5\n")?;
    let project = ProjectStore::initialize(&source, "original")?;
    project.store().put(
        "project",
        "original",
        &json!({"id":"original","path":source}),
    )?;
    for task in ["task", "sibling"] {
        project.store().put(
            "task",
            task,
            &json!({"id":task,"projectId":"original","status":"paused"}),
        )?;
        fs::create_dir_all(source.join(format!(".beaver/workspaces/{task}")))?;
    }
    drop(project);
    fs::create_dir_all(source.join(ROLLOUT).parent().unwrap())?;
    fs::write(source.join(ROLLOUT), HISTORY)?;
    let index = Connection::open(source.join(INDEX))?;
    index.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE threads(id TEXT PRIMARY KEY, rollout_path TEXT NOT NULL, cwd TEXT NOT NULL, opaque BLOB);
         CREATE TABLE project_roots(path TEXT, unknown TEXT);
         CREATE TABLE rollout_migration_skipped_rollouts(rollout_path TEXT);
         CREATE TABLE future_state(value BLOB);
         INSERT INTO future_state VALUES (x'00FF0100');",
    )?;
    index.execute(
        "INSERT INTO threads VALUES (?,?,?,?)",
        params![
            "thread-preserved",
            source.join(ROLLOUT).to_str(),
            source.join(".beaver/workspaces/task").to_str(),
            vec![0_u8, 255, 17],
        ],
    )?;
    index.execute(
        "INSERT INTO project_roots VALUES (?,?)",
        params![source.to_str(), "original task opaque"],
    )?;
    index.execute(
        "INSERT INTO rollout_migration_skipped_rollouts VALUES (?)",
        [source.join(ROLLOUT).to_str()],
    )?;
    change(&index, &source)?;
    assert!(source.join(format!("{INDEX}-wal")).metadata()?.len() > 0);
    let before = data_backup::inventory(&source)?;
    let preparation = root.join("prepared");
    copy::prepare(
        copy::Request {
            request_id: "copy".into(),
            source: source.clone(),
            source_project_id: "original".into(),
            target_project_id: "derived".into(),
        },
        &preparation,
    )?;
    assert_eq!(data_backup::inventory(&source)?, before);
    Ok((preparation, index))
}

#[test]
fn session_stage_preserves_wal_history_and_recovers_offline() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (preparation, source_index) = prepare(temp.path(), |_, _| Ok(()))?;
    let source_before = data_backup::inventory(&temp.path().join("source"))?;
    let copy_before = data_backup::inventory(&preparation.join("project"))?;
    let binding = temp.path().join("final-project");
    let receipt = create(&preparation, "one", &binding)?;
    assert_eq!(receipt.paths_rewritten, 4);
    assert!(!binding.exists());
    assert_eq!(
        data_backup::inventory(&preparation.join("project"))?,
        copy_before
    );
    assert_eq!(
        data_backup::inventory(&temp.path().join("source"))?,
        source_before
    );
    drop(source_index);
    fs::rename(
        temp.path().join("source"),
        temp.path().join("offline-source"),
    )?;
    let moved = temp.path().join("moved");
    fs::rename(&preparation, &moved)?;
    assert_eq!(inspect(&moved, "one", &binding)?.entries, receipt.entries);
    assert!(inspect(&moved, "one", &temp.path().join("elsewhere")).is_err());
    assert!(create(&moved, "one", &binding).is_err());
    let prepared = copy::inspect(&moved)?;
    let mapped = Paths(&prepared.identities);
    let database = directory(&moved, "one")?
        .join("indexes")
        .join(mapped.relative(INDEX)?);
    let db = Connection::open_with_flags(database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let (id, rollout, cwd, opaque): (String, String, String, Vec<u8>) = db.query_row(
        "SELECT id,rollout_path,cwd,opaque FROM threads",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    assert_eq!(id, "thread-preserved");
    assert_eq!(Path::new(&rollout), binding.join(mapped.relative(ROLLOUT)?));
    assert_eq!(
        Path::new(&cwd),
        binding.join(mapped.relative(".beaver/workspaces/task")?)
    );
    assert_eq!(opaque, [0, 255, 17]);
    assert_eq!(
        db.query_row("SELECT unknown FROM project_roots", [], |r| r
            .get::<_, String>(0))?,
        "original task opaque"
    );
    assert_eq!(
        db.query_row("SELECT value FROM future_state", [], |r| r
            .get::<_, Vec<u8>>(0))?,
        [0, 255, 1, 0]
    );
    assert_eq!(
        Path::new(&db.query_row("SELECT path FROM project_roots", [], |r| {
            r.get::<_, String>(0)
        })?),
        binding
    );
    assert_eq!(
        db.query_row(
            "SELECT rollout_path FROM rollout_migration_skipped_rollouts",
            [],
            |r| r.get::<_, String>(0)
        )?,
        rollout
    );
    assert_eq!(fs::read(moved.join("project").join(ROLLOUT))?, HISTORY);
    crate::project_derivation_files::create(&moved, "one")?;
    assert_eq!(
        fs::read(
            moved
                .join("files-stage-one/project")
                .join(mapped.relative(ROLLOUT)?)
        )?,
        HISTORY
    );
    let second = create(&moved, "two", &temp.path().join("elsewhere"))?;
    assert_eq!(second.paths_rewritten, 4);
    Ok(())
}

#[test]
fn session_paths_reject_cross_home_missing_type_and_traversal() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (preparation, _source_index) = prepare(temp.path(), |_, _| Ok(()))?;
    let prepared = copy::inspect(&preparation)?;
    let binding = temp.path().join("final");
    for (relative, rollout) in [
        (".beaver/workspaces/sibling", false),
        (
            ".beaver/workspaces/.codex/sibling/sessions/history.jsonl",
            true,
        ),
        (".beaver/workspaces/task/missing", false),
        (".beaver/workspaces/task", true),
        (ROLLOUT, false),
        (
            ".beaver/workspaces/.codex/task/../task/sessions/history.jsonl",
            true,
        ),
    ] {
        assert!(
            paths::target(
                &prepared,
                INDEX,
                &format!("{}/{relative}", prepared.request.source.display()),
                rollout,
                &binding
            )
            .is_err(),
            "{relative}"
        );
    }
    assert!(paths::target(
        &prepared,
        INDEX,
        temp.path().join("outside").to_str().unwrap(),
        false,
        &binding
    )
    .is_err());
    Ok(())
}

#[test]
fn session_failure_preserves_generation_and_preparation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (preparation, _source_index) = prepare(temp.path(), |db, source| {
        db.execute(
            "UPDATE threads SET cwd=?",
            [source.join("missing").to_str()],
        )?;
        Ok(())
    })?;
    let before = data_backup::inventory(&preparation.join("project"))?;
    let binding = temp.path().join("final");
    assert!(create(&preparation, "bad", &binding).is_err());
    assert!(directory(&preparation, "bad")?
        .join(".beaver-migration-pending")
        .is_file());
    assert!(!directory(&preparation, "bad")?.join(RECEIPT).exists());
    assert!(inspect(&preparation, "bad", &binding).is_err());
    assert_eq!(
        data_backup::inventory(&preparation.join("project"))?,
        before
    );
    Ok(())
}

#[test]
fn session_schema_and_trigger_rejection_is_fail_closed() -> Result<()> {
    for sql in [
        "DROP TABLE threads; CREATE VIEW threads AS SELECT 'id' AS id, 'p' AS rollout_path, 'c' AS cwd",
        "DROP TABLE threads; CREATE TABLE threads(id TEXT, cwd TEXT)",
        "CREATE TRIGGER mutate AFTER UPDATE ON threads BEGIN DELETE FROM future_state; END",
        "DROP TABLE project_roots; CREATE TABLE project_roots(unknown TEXT)",
    ] {
        let temp = tempfile::tempdir()?;
        let (preparation, _source_index) = prepare(temp.path(), |db, _| Ok(db.execute_batch(sql)?))?;
        assert!(create(&preparation, "bad", &temp.path().join("final")).is_err(), "{sql}");
    }
    Ok(())
}

#[test]
fn session_receipt_rejects_corrupt_payload() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (preparation, _source_index) = prepare(temp.path(), |_, _| Ok(()))?;
    let binding = temp.path().join("final");
    let receipt = create(&preparation, "one", &binding)?;
    let file = receipt
        .entries
        .iter()
        .find(|entry| entry.sha256.is_some())
        .unwrap();
    fs::write(
        directory(&preparation, "one")?
            .join("indexes")
            .join(&file.path),
        b"corrupt",
    )?;
    assert!(inspect(&preparation, "one", &binding).is_err());
    Ok(())
}
