use super::*;
use crate::{
    data_backup,
    preferences::{protect, SystemVault, Vault},
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{fs, path::PathBuf};

struct Fixture {
    temp: tempfile::TempDir,
    data: PathBuf,
    game: PathBuf,
    project: String,
    task: String,
    old_cipher: String,
}

fn fixture() -> Result<Fixture> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("old-data");
    let game = temp.path().join("old-game");
    let project = uuid::Uuid::new_v4().to_string();
    let task = uuid::Uuid::new_v4().to_string();
    fs::create_dir_all(&game)?;
    fs::write(game.join("project.godot"), "config_version=5")?;
    fs::write(game.join("a.txt"), "before")?;
    let files = Files::new(data.clone());
    let before = files.capture(&game)?;
    fs::write(game.join("a.txt"), "after")?;
    let after = files.capture(&game)?;
    fs::write(game.join("a.txt"), "before")?;
    let workspace = data.join("workspaces").join(&task);
    fs::create_dir_all(&workspace)?;
    fs::write(workspace.join("untouched.txt"), "workspace bytes")?;
    let sessions = data.join("codex").join(&task).join("sessions");
    fs::create_dir_all(&sessions)?;
    fs::write(sessions.join("old.jsonl"), "opaque old cwd and thread\n")?;
    let index = Connection::open(data.join("codex").join(&task).join("state_5.sqlite"))?;
    index.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY,rollout_path TEXT NOT NULL,cwd TEXT NOT NULL,unknown TEXT);
        CREATE TABLE project_roots(project_id TEXT,path TEXT);
        CREATE TABLE rollout_migration_skipped_rollouts(rollout_path TEXT);")?;
    index.execute(
        "INSERT INTO threads VALUES('old-thread',?,?,'preserved')",
        params![
            sessions.join("old.jsonl").to_string_lossy(),
            workspace.to_string_lossy()
        ],
    )?;
    index.execute(
        "INSERT INTO project_roots VALUES(?,?)",
        params![project, game.to_string_lossy()],
    )?;
    index.execute(
        "INSERT INTO rollout_migration_skipped_rollouts VALUES(?)",
        [sessions.join("old.jsonl").to_string_lossy()],
    )?;
    drop(index);
    fs::create_dir_all(data.join("tools"))?;
    fs::write(data.join("tools/godot.exe"), "fixture, never executed")?;
    let key = [42; 32];
    fs::write(
        data.join("Local State"),
        json!({"os_crypt":{"encrypted_key":STANDARD.encode(
        [b"DPAPI".as_slice(), protect(&key, false)?.as_slice()].concat())}})
        .to_string(),
    )?;
    let nonce = [7; 12];
    let body = Aes256Gcm::new_from_slice(&key)
        .unwrap()
        .encrypt(
            Nonce::from_slice(&nonce),
            b"dummy-import-credential".as_slice(),
        )
        .unwrap();
    let old_cipher = STANDARD.encode([b"v10".as_slice(), &nonce, &body].concat());
    let store = Store::open(&data)?;
    store.put(
        "project",
        &project,
        &json!({"id":project,"path":game,"unknown":"kept"}),
    )?;
    let current = json!({"id":task,"projectId":project,"workspace":workspace,
        "status":"running","threadId":"old-thread","turnId":"old-turn","unknown":{"keep":true}});
    store.put("task", &task, &current)?;
    let mut task_after = current;
    task_after["status"] = json!("completed");
    store.put("operation", "operation-fixture", &json!({"id":"operation-fixture","projectId":project,
        "taskId":task,"kind":"merge","state":"applying","changes":Files::changes(&before,&after),"taskAfter":task_after}))?;
    store.put("secret", "code", &old_cipher)?;
    store.put("secret", "cloud", &SystemVault.encrypt("native-fixture")?)?;
    store.put("settings", "main", &json!({"tools":{"codex":"codex","godot":data.join("tools/godot.exe"),"node":"","blender":""},"unknown":"preserved"}))?;
    store.put("toolSetup", "main", &json!({"status":"running","active":"godot","steps":[
        {"name":"godot","status":"checking","result":{"path":data.join("tools/godot.exe"),"available":true}}
    ]}))?;
    store.event(&task, "fixture-time", "report", "preserved event")?;
    drop(store);
    Ok(Fixture {
        temp,
        data,
        game,
        project,
        task,
        old_cipher,
    })
}

#[test]
fn prepared_import_rebases_before_journal_and_preserves_originals() -> Result<()> {
    let f = fixture()?;
    let source_before = data_backup::inventory(&f.data)?;
    let game_before = data_backup::inventory(&f.game)?;
    let backup = f.temp.path().join("backup");
    migration_bundle::create(&f.data, &backup)?;
    let archive_before = data_backup::inventory(&backup)?;
    let moved_data = f.temp.path().join("unavailable-data");
    let moved_game = f.temp.path().join("unavailable-game");
    fs::rename(&f.data, &moved_data)?;
    fs::rename(&f.game, &moved_game)?;
    let destination = f.temp.path().join("prepared");
    let receipt = prepare(&backup, &destination)?;
    assert_eq!(receipt.credentials_converted, 1);
    assert_eq!(receipt.codex_index_paths_rewritten, 4);
    assert_eq!(receipt.journals_recovered, 1);
    assert_eq!(receipt.tasks_interrupted, 0); // Journal commit precedes task interruption.
    assert!(!receipt.ready_to_activate);
    assert!(migration_bundle::ensure_activated(&receipt.restored_data).is_err());
    let store = Store::open(&receipt.restored_data)?;
    let index = Connection::open(
        receipt
            .restored_data
            .join("codex")
            .join(&f.task)
            .join("state_5.sqlite"),
    )?;
    let (rollout, cwd, unknown): (String, String, String) = index.query_row(
        "SELECT rollout_path,cwd,unknown FROM threads WHERE id='old-thread'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(fs::read_to_string(rollout)?, "opaque old cwd and thread\n");
    assert_eq!(
        cwd,
        safe_path(&receipt.restored_data, &format!("workspaces/{}", f.task))?.to_string_lossy()
    );
    assert_eq!(unknown, "preserved");
    assert_eq!(
        index.query_row("SELECT path FROM project_roots", [], |row| row
            .get::<_, String>(0))?,
        receipt
            .restored_data
            .parent()
            .unwrap()
            .join("projects")
            .join(&f.project)
            .to_string_lossy()
    );
    drop(index);
    let task = store.get::<Value>("task", &f.task)?.unwrap();
    assert_eq!(task["status"], "completed");
    assert_eq!(task["threadId"], "old-thread");
    assert_eq!(task["turnId"], "old-turn");
    assert_eq!(task["unknown"]["keep"], true);
    assert_eq!(
        task["workspace"],
        json!(safe_path(
            &receipt.restored_data,
            &format!("workspaces/{}", f.task)
        )?)
    );
    assert_eq!(
        store
            .get::<Value>("operation", "operation-fixture")?
            .unwrap()["taskAfter"]["workspace"],
        task["workspace"]
    );
    let project = store.get::<Value>("project", &f.project)?.unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(project["path"].as_str().unwrap()).join("a.txt"))?,
        "after"
    );
    assert_eq!(
        crate::preferences::key(&store, &SystemVault, "code")?,
        "dummy-import-credential"
    );
    assert_eq!(
        store.list::<Value>("secret_backup")?[0]["cipher"],
        f.old_cipher
    );
    let settings = store.get::<Value>("settings", "main")?.unwrap();
    assert_eq!(settings["unknown"], "preserved");
    assert_eq!(
        settings["tools"]["godot"],
        json!(safe_path(&receipt.restored_data, "tools/godot.exe")?)
    );
    let setup = store.get::<Value>("toolSetup", "main")?.unwrap();
    assert_eq!(setup["status"], "cancelled");
    assert_eq!(setup["steps"][0]["result"]["available"], false);
    assert_eq!(store.events(&f.task)?[0].text, "preserved event");
    assert_eq!(
        fs::read(
            receipt
                .restored_data
                .join("codex")
                .join(&f.task)
                .join("sessions/old.jsonl")
        )?,
        b"opaque old cwd and thread\n"
    );
    drop(store);
    assert_eq!(data_backup::inventory(&backup)?, archive_before);
    assert_eq!(data_backup::inventory(&moved_data)?, source_before);
    assert_eq!(data_backup::inventory(&moved_game)?, game_before);
    assert!(prepare(&backup, &destination).is_err());
    Ok(())
}

#[test]
fn invalid_paths_or_credentials_cannot_partially_convert_entities() -> Result<()> {
    for failure in ["workspace", "native-key", "legacy-key"] {
        let f = fixture()?;
        let store = Store::open(&f.data)?;
        if failure == "workspace" {
            let mut task = store.get::<Value>("task", &f.task)?.unwrap();
            task["workspace"] = json!(f.game);
            store.put("task", &f.task, &task)?;
        } else if failure == "native-key" {
            store.put("secret", "cloud", &"beaver-dpapi-v1:AAAA")?;
        } else {
            store.put("secret", "review", &"corrupt-v10")?;
        }
        drop(store);
        let backup = f.temp.path().join("backup");
        migration_bundle::create(&f.data, &backup)?;
        let destination = f.temp.path().join("rejected");
        assert!(prepare(&backup, &destination).is_err());
        assert!(!destination.join("IMPORT.json").exists());
        assert!(migration_bundle::ensure_activated(&destination.join("data")).is_err());
        let store = Store::open(&destination.join("data"))?;
        assert_eq!(
            store.get::<Value>("project", &f.project)?.unwrap()["path"],
            json!(f.game)
        );
        assert_eq!(
            store.get::<String>("secret", "code")?.unwrap(),
            f.old_cipher
        );
        assert!(store.list::<Value>("secret_backup")?.is_empty());
        assert_eq!(
            fs::read_to_string(destination.join("projects").join(&f.project).join("a.txt"))?,
            "before"
        );
    }
    Ok(())
}

#[test]
fn unresolved_journal_stays_pending_and_does_not_interrupt_tasks() -> Result<()> {
    let f = fixture()?;
    let store = Store::open(&f.data)?;
    let mut operation = store
        .get::<Value>("operation", "operation-fixture")?
        .unwrap();
    operation["changes"][0]["after"] = json!("0".repeat(64));
    store.put("operation", "operation-fixture", &operation)?;
    drop(store);
    let backup = f.temp.path().join("backup");
    migration_bundle::create(&f.data, &backup)?;
    let destination = f.temp.path().join("blocked");
    assert!(prepare(&backup, &destination)
        .unwrap_err()
        .to_string()
        .contains("recovery remains blocked"));
    assert!(!destination.join("IMPORT.json").exists());
    assert!(migration_bundle::ensure_activated(&destination).is_err());
    let store = Store::open(&destination.join("data"))?;
    assert_eq!(
        store
            .get::<Value>("operation", "operation-fixture")?
            .unwrap()["state"],
        "applying"
    );
    assert_eq!(
        store.get::<Value>("task", &f.task)?.unwrap()["status"],
        "conflict"
    );
    assert_eq!(fs::read_to_string(f.game.join("a.txt"))?, "before");
    Ok(())
}

#[test]
fn lexical_paths_handle_namespaces_and_reject_escape_and_prefix_collision() -> Result<()> {
    let root = Path::new(r"\\?\C:\Old\Data");
    assert_eq!(
        relative(root, r"c:/old/data/workspaces/id")?,
        Some("workspaces/id".into())
    );
    assert_eq!(relative(root, r"C:\Old\Data-other\tools\x")?, None);
    for value in [
        r"C:\Old\Data\..\other",
        r"C:\Old\Data\tools\x:stream",
        r"C:\Old\Data\tools.\x",
        r"C:relative",
    ] {
        assert!(relative(root, value).is_err());
    }
    assert_eq!(
        relative(
            Path::new(r"\\?\UNC\host\share\Data"),
            r"\\HOST\share\data\tools\x"
        )?,
        Some("tools/x".into())
    );
    Ok(())
}

#[test]
fn missing_codex_history_or_unknown_schema_keeps_import_pending() -> Result<()> {
    for malformed_schema in [false, true] {
        let f = fixture()?;
        let index = Connection::open(f.data.join("codex").join(&f.task).join("state_5.sqlite"))?;
        if malformed_schema {
            index.execute_batch(
                "ALTER TABLE threads RENAME COLUMN rollout_path TO unknown_rollout",
            )?;
        } else {
            index.execute(
                "UPDATE threads SET rollout_path=?",
                [f.data
                    .join("codex")
                    .join(&f.task)
                    .join("sessions/missing.jsonl")
                    .to_string_lossy()],
            )?;
        }
        drop(index);
        let backup = f.temp.path().join("backup");
        migration_bundle::create(&f.data, &backup)?;
        let target = f.temp.path().join("rejected");
        assert!(prepare(&backup, &target).is_err());
        assert!(!target.join("IMPORT.json").exists());
        assert!(migration_bundle::ensure_activated(&target).is_err());
        let store = Store::open(&target.join("data"))?;
        assert_eq!(
            store.get::<String>("secret", "code")?.unwrap(),
            f.old_cipher
        );
    }
    Ok(())
}
