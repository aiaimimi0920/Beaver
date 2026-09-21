use super::{file, inspect, source, task, Fixture};
use anyhow::Result;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::Value;
use std::{fs, path::Path};

fn database(fixture: &Fixture, task: &str, number: usize) -> Result<Connection> {
    let home = fixture.data.join("codex").join(task);
    fs::create_dir_all(&home)?;
    let connection = Connection::open(home.join(format!("state_{number}.sqlite")))?;
    connection.execute_batch("CREATE TABLE threads(id TEXT, rollout_path, cwd, unknown TEXT);")?;
    Ok(connection)
}

fn thread(connection: &Connection, rollout: &Path, cwd: &Path) -> Result<()> {
    connection.execute(
        "INSERT INTO threads VALUES('invalid-value-must-not-escape',?,?,'file-payload-must-not-escape')",
        params![rollout.to_string_lossy(), cwd.to_string_lossy()],
    )?;
    Ok(())
}

fn issue(report: &Value, database: &str, field: &str, reason: &str) {
    assert!(
        report["sessionIndexes"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| {
                issue["sourceId"] == format!("application/data/{database}")
                    && issue["field"] == field
                    && issue["reason"] == reason
            }),
        "missing {database} {field} {reason}: {}",
        report["sessionIndexes"]
    );
}

fn target<'a>(
    report: &'a Value,
    database: &str,
    table: &str,
    column: &str,
    rowid: i64,
) -> &'a Value {
    &report["sessionIndexes"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["indexPath"] == format!("application/data/{database}")
                && record["table"] == table
                && record["column"] == column
                && record["rowid"] == rowid
        })
        .unwrap()["targetArchivePath"]
}

#[test]
fn session_paths_resolve_offline_and_leave_history_and_unknown_columns_opaque() -> Result<()> {
    let fixture = Fixture::new()?;
    for (position, project) in fixture.projects.iter().enumerate() {
        let id = format!("task-{position}");
        task(&fixture, &id, project)?;
        // Deliberately not JSON; the scanner must not parse or rewrite this history.
        file(&fixture, &format!("codex/{id}/sessions/old.jsonl"))?;
        let connection = database(&fixture, &id, 5)?;
        thread(
            &connection,
            &source(&fixture, &format!("codex/{id}/sessions/old.jsonl")),
            &source(&fixture, &format!("workspaces/{id}")),
        )?;
        if position == 0 {
            connection.execute_batch(
                "CREATE TABLE project_roots(path TEXT);
                CREATE TABLE rollout_migration_skipped_rollouts(rollout_path TEXT);",
            )?;
            let project = fixture.temp.path().join("game-0");
            fs::create_dir(project.join("scenes"))?;
            // Windows extended prefix and case differences refer to the same archive file.
            let rollout = format!(
                "\\\\?\\{}",
                source(&fixture, "codex/task-0/sessions/old.jsonl").display()
            )
            .to_uppercase();
            thread(&connection, Path::new(&rollout), &project)?;
            for path in [&project, &project.join("scenes")] {
                connection.execute(
                    "INSERT INTO project_roots VALUES(?)",
                    [path.to_string_lossy()],
                )?;
            }
            connection.execute(
                "INSERT INTO rollout_migration_skipped_rollouts VALUES(?)",
                [rollout],
            )?;
        }
    }
    file(&fixture, "codex/task-0/state_future.sqlite")?;
    let report = inspect(&fixture)?;
    assert_eq!(report["sessionIndexes"]["indexes"], 2);
    assert_eq!(report["sessionIndexes"]["checked"], 9);
    assert_eq!(
        report["sessionIndexes"]["issues"].as_array().unwrap().len(),
        0
    );
    assert_eq!(
        report["sessionIndexes"]["records"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    let db = "codex/task-0/state_5.sqlite";
    assert_eq!(
        target(&report, db, "threads", "rollout_path", 2),
        "application/data/codex/task-0/sessions/old.jsonl"
    );
    assert_eq!(
        target(&report, db, "threads", "cwd", 2),
        &format!("projects/{}", fixture.projects[0])
    );
    assert_eq!(
        target(&report, db, "project_roots", "path", 2),
        &format!("projects/{}/scenes", fixture.projects[0])
    );
    Ok(())
}

#[test]
fn session_index_queries_committed_wal_on_private_copy_without_touching_archive() -> Result<()> {
    let fixture = Fixture::new()?;
    task(&fixture, "task-0", &fixture.projects[0])?;
    file(&fixture, "codex/task-0/sessions/old.jsonl")?;
    let connection = database(&fixture, "task-0", 5)?;
    connection.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; PRAGMA wal_checkpoint(TRUNCATE);",
    )?;
    thread(
        &connection,
        &source(&fixture, "codex/task-0/sessions/old.jsonl"),
        &source(&fixture, "workspaces/task-0"),
    )?;
    let home = fixture.data.join("codex/task-0");
    assert!(fs::metadata(home.join("state_5.sqlite-wal"))?.len() > 0);
    assert!(fs::metadata(home.join("state_5.sqlite-shm"))?.len() > 0);
    let main_only = fixture.temp.path().join("main-only.sqlite");
    fs::copy(home.join("state_5.sqlite"), &main_only)?;
    let main = Connection::open_with_flags(main_only, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    assert_eq!(
        main.query_row("SELECT count(*) FROM threads", [], |row| row
            .get::<_, i64>(0))?,
        0
    );
    // Keep the writer alive so closing it cannot checkpoint away the WAL under test.
    let report = inspect(&fixture)?;
    assert_eq!(report["sessionIndexes"]["checked"], 2);
    assert_eq!(
        report["sessionIndexes"]["records"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(report["sessionIndexes"]["issues"]
        .as_array()
        .unwrap()
        .is_empty());
    drop(connection);
    Ok(())
}

#[test]
fn invalid_session_paths_report_stable_reasons_without_path_or_payload_leakage() -> Result<()> {
    let fixture = Fixture::new()?;
    for position in 0..2 {
        task(
            &fixture,
            &format!("task-{position}"),
            &fixture.projects[position],
        )?;
        file(
            &fixture,
            &format!("codex/task-{position}/sessions/old.jsonl"),
        )?;
    }
    // Even a different task in the same project cannot supply this index's rollout.
    task(&fixture, "earlier", &fixture.projects[0])?;
    file(&fixture, "codex/earlier/sessions/old.jsonl")?;
    let connection = database(&fixture, "task-0", 5)?;
    let rollout = source(&fixture, "codex/task-0/sessions/old.jsonl");
    let cwd = source(&fixture, "workspaces/task-0");
    let project = fixture.temp.path().join("game-0");
    let cases = [
        (
            source(&fixture, "codex/task-0/sessions/missing.jsonl"),
            cwd.clone(),
            "rollout_path",
            "FILE_NOT_FOUND",
        ),
        (
            source(&fixture, "codex/earlier/sessions/old.jsonl"),
            cwd.clone(),
            "rollout_path",
            "SESSION_ROLLOUT_OUTSIDE_HOME",
        ),
        (
            source(&fixture, "codex/task-1/sessions/old.jsonl"),
            cwd.clone(),
            "rollout_path",
            "SESSION_ROLLOUT_OUTSIDE_HOME",
        ),
        (
            source(&fixture, "codex/task-0/sessions"),
            cwd.clone(),
            "rollout_path",
            "FILE_TYPE_MISMATCH",
        ),
        (
            rollout.clone(),
            source(&fixture, "workspaces/task-1"),
            "cwd",
            "FILE_PROJECT_MISMATCH",
        ),
        (
            rollout.clone(),
            fixture.temp.path().join("game-1"),
            "cwd",
            "FILE_PROJECT_MISMATCH",
        ),
        (
            rollout.clone(),
            project.join("project.godot"),
            "cwd",
            "FILE_TYPE_MISMATCH",
        ),
        (
            rollout.clone(),
            project.join("missing"),
            "cwd",
            "FILE_NOT_FOUND",
        ),
        (
            rollout.clone(),
            fixture.temp.path().join("game-0-sibling"),
            "cwd",
            "SESSION_PATH_OUTSIDE_ARCHIVE",
        ),
        (
            rollout.clone(),
            Path::new("invalid-value-must-not-escape").to_path_buf(),
            "cwd",
            "INVALID_FILE_REFERENCE",
        ),
        (
            rollout.clone(),
            cwd.join(".."),
            "cwd",
            "INVALID_FILE_REFERENCE",
        ),
        (
            rollout.clone(),
            cwd.join("file:stream"),
            "cwd",
            "INVALID_FILE_REFERENCE",
        ),
        (
            rollout.clone(),
            rollout.clone(),
            "cwd",
            "FILE_TYPE_MISMATCH",
        ),
    ];
    for (rollout, cwd, _, _) in &cases {
        thread(&connection, rollout, cwd)?;
    }
    connection.execute(
        "INSERT INTO threads VALUES('hidden',NULL,?,'hidden')",
        [cwd.to_string_lossy()],
    )?;
    connection.execute(
        "INSERT INTO threads VALUES('hidden',42,?,'hidden')",
        [cwd.to_string_lossy()],
    )?;
    connection.execute_batch(
        "CREATE TABLE project_roots(path TEXT);
        CREATE TABLE rollout_migration_skipped_rollouts(rollout_path TEXT);",
    )?;
    connection.execute(
        "INSERT INTO project_roots VALUES(?)",
        [fixture.temp.path().join("game-1").to_string_lossy()],
    )?;
    connection.execute(
        "INSERT INTO rollout_migration_skipped_rollouts VALUES(?)",
        [source(&fixture, "codex/task-0/sessions/skipped.jsonl").to_string_lossy()],
    )?;
    drop(connection);
    let report = inspect(&fixture)?;
    let db = "codex/task-0/state_5.sqlite";
    for (position, (_, _, column, reason)) in cases.iter().enumerate() {
        issue(
            &report,
            db,
            &format!("/threads/{}/{column}", position + 1),
            reason,
        );
    }
    for rowid in [cases.len() + 1, cases.len() + 2] {
        issue(
            &report,
            db,
            &format!("/threads/{rowid}/rollout_path"),
            "INVALID_FILE_REFERENCE",
        );
    }
    issue(
        &report,
        db,
        "/project_roots/1/path",
        "FILE_PROJECT_MISMATCH",
    );
    issue(
        &report,
        db,
        "/rollout_migration_skipped_rollouts/1/rollout_path",
        "SESSION_SKIPPED_ROLLOUT_UNAVAILABLE",
    );
    assert_eq!(
        report["sessionIndexes"]["issues"].as_array().unwrap().len(),
        cases.len() + 4
    );
    Ok(())
}

#[test]
fn unsupported_or_corrupt_session_indexes_remain_explicitly_unresolved() -> Result<()> {
    let fixture = Fixture::new()?;
    for (number, schema) in [
        "DROP TABLE threads; CREATE TABLE threads(id TEXT);",
        "CREATE TABLE project_roots(unknown TEXT);",
        "CREATE VIEW project_roots AS SELECT cwd AS path FROM threads;",
        "DROP TABLE threads; CREATE TABLE threads(id TEXT PRIMARY KEY,rollout_path,cwd) WITHOUT ROWID;",
    ].iter().enumerate() {
        database(&fixture, "task-0", number)?.execute_batch(schema)?;
    }
    file(&fixture, "codex/task-0/state_4.sqlite")?;
    database(&fixture, "orphan", 5)?;
    fs::create_dir(fixture.data.join("codex/task-0/state_6.sqlite"))?;
    database(&fixture, "task-0", 7)?;
    fs::create_dir(fixture.data.join("codex/task-0/state_7.sqlite-wal"))?;
    let report = inspect(&fixture)?;
    assert_eq!(report["sessionIndexes"]["indexes"], 8);
    assert!(report["sessionIndexes"]["records"]
        .as_array()
        .unwrap()
        .is_empty());
    for number in 0..4 {
        issue(
            &report,
            &format!("codex/task-0/state_{number}.sqlite"),
            "/",
            "SESSION_INDEX_SCHEMA_UNSUPPORTED",
        );
    }
    issue(
        &report,
        "codex/task-0/state_4.sqlite",
        "/",
        "SESSION_INDEX_UNREADABLE",
    );
    issue(
        &report,
        "codex/orphan/state_5.sqlite",
        "/",
        "FILE_OWNER_UNRESOLVED",
    );
    for number in [6, 7] {
        issue(
            &report,
            &format!("codex/task-0/state_{number}.sqlite"),
            "/",
            "FILE_TYPE_MISMATCH",
        );
    }
    Ok(())
}
