use super::{host, restore_and_partition, task, Fixture};
use crate::{
    asset_task, asset_work_inputs, data_backup, files::file_hash, migration_bundle,
    project_migration_activation, project_storage_router::ProjectStorageRouter, store::Store,
    task_actions, task_plan,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

#[path = "project_migration_reopen_history.rs"]
mod history;

fn native_tools(directory: &Path) -> Result<Value> {
    fs::create_dir(directory)?;
    let executable = directory.join("probe.exe");
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(compiler)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/migration_tool_probe.rs"))
        .args(["--edition=2021", "--crate-name=migration_tool_probe", "-o"])
        .arg(&executable)
        .output()
        .context("Compile isolated native migration version probes")?;
    ensure!(
        output.status.success(),
        "Native probe compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut paths = json!({});
    for name in ["codex", "godot", "blender", "node"] {
        let path = directory.join(format!("{name}.exe"));
        fs::copy(&executable, &path)?;
        paths[name] = json!(path);
    }
    Ok(paths)
}

fn unopened(restored: &Path) -> Result<()> {
    assert!(restored.join(".beaver-migration-pending").is_file());
    assert!(!restored.join("ACTIVATION.json").exists());
    assert!(migration_bundle::ensure_activated(&restored.join("data")).is_err());
    let router = ProjectStorageRouter::new(Arc::new(Mutex::new(host(restored)?)));
    assert!(router.open_registered_local().is_err());
    assert!(router.runtimes()?.is_empty());
    Ok(())
}

fn recovered(
    history: &history::History,
    router: &ProjectStorageRouter,
    host: &Arc<Mutex<Store>>,
) -> Result<Value> {
    assert!(router.runtime_for_task("task-retained").is_err());
    let approved_runtime = router.runtime_for_task("task-0")?;
    let runtime = router.runtime_for_task("task-1")?;
    assert_ne!(approved_runtime.project_id(), runtime.project_id());
    let approved_store = approved_runtime.store();
    let approved_store = approved_store.lock().unwrap();
    let approved: Value = approved_store.get("task", "task-0")?.unwrap();
    let mut expected = history.approved.clone();
    expected["workspace"] = json!(".beaver/workspaces/task-0");
    assert_eq!(approved, expected);
    assert_eq!(approved["accepted"], true);
    assert_eq!(approved["approvalSource"], "user");
    assert_eq!(approved_store.events("task-0")?, history.approval_events);
    assert_eq!(history.approval_events.len(), 1);
    assert_eq!(history.approval_events[0].kind, "approval");

    let store = runtime.store();
    let store = store.lock().unwrap();
    let running: Value = store.get("task", "task-1")?.unwrap();
    let queued: Value = store.get("task", "task-queued")?.unwrap();
    assert_eq!(running["status"], "interrupted");
    assert_eq!(queued["status"], "interrupted");
    let asset = asset_task::get(&store, "task-1")?;
    assert_eq!(asset.revision, history.asset.revision + 1);
    assert!(asset.session_id.is_none());
    assert!(asset.recovery.is_some());
    assert_eq!(asset.work.attempts.len(), 3);
    assert_eq!(
        serde_json::to_value(&asset.work.attempts[..2])?,
        serde_json::to_value(&history.asset.work.attempts[..2])?
    );
    assert_eq!(
        serde_json::to_value(&asset.work.subtasks[..2])?,
        serde_json::to_value(&history.asset.work.subtasks[..2])?
    );
    assert_eq!(asset.work.subtasks[1].status, "cancelled");
    assert_eq!(
        asset.work.subtasks[1].note,
        "Owner cancelled this optional change"
    );
    assert_eq!(asset.work.attempts[2].status, "interrupted");
    assert!(asset.work.attempts[2].ended_at.is_some());
    assert_eq!(asset.work.subtasks[2].status, "interrupted");
    let ids = asset.work.attempts[..2]
        .iter()
        .map(|a| a.id.clone())
        .collect::<Vec<_>>();
    let inputs = asset_work_inputs::for_attempts(&asset, &ids)?;
    assert_eq!(inputs.len(), 2);
    assert_ne!(inputs[0].sha256, inputs[1].sha256);
    let files = runtime.files();
    let workspace =
        files.resolve_workspace("task-1", Path::new(running["workspace"].as_str().unwrap()))?;
    assert_eq!(
        fs::read_to_string(workspace.join("model.blend"))?,
        history::CURRENT
    );
    asset_work_inputs::verify(&files, &workspace, &inputs)?;
    for (input, bytes) in inputs.iter().zip(history::VERSIONS) {
        assert_eq!(fs::read_to_string(files.blob(&input.sha256)?)?, bytes);
        assert!(!approved_runtime.files().blob(&input.sha256)?.exists());
    }
    let evidence: Value = store.get("validationRun", "run-1")?.unwrap();
    assert_eq!(evidence, history.evidence);
    let path = runtime
        .project_root()
        .join(".beaver/evidence/run-1/run.log");
    assert_eq!(fs::read_to_string(&path)?, history::EVIDENCE);
    assert_eq!(
        file_hash(&path)?.as_deref(),
        evidence["evidence"][0]["sha256"].as_str()
    );
    for (kind, id) in [
        ("task", "task-1"),
        ("asset-task", "task-1"),
        ("validationRun", "run-1"),
    ] {
        assert!(approved_store.get::<Value>(kind, id)?.is_none());
        assert!(host.lock().unwrap().get::<Value>(kind, id)?.is_none());
    }
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", "task-0")?
        .is_none());
    let mut host = host.lock().unwrap();
    let retained: Value = host.get("task", "task-retained")?.unwrap();
    assert_eq!(retained["status"], "interrupted");
    assert_eq!(
        retained["migrationRetained"]["reason"],
        "FILE_OUTSIDE_APPLICATION"
    );
    assert_eq!(host.events("task-retained")?[0].text, "Unconverted history");
    assert!(!task_plan::eligible(&retained, &[]));
    assert!(task_actions::continue_task(&mut host, "task-retained", "", false).is_err());
    Ok(
        json!({"approved":approved,"asset":asset,"running":running,"queued":queued,"evidence":evidence,"retained":retained}),
    )
}

#[test]
fn actual_activation_retries_tools_and_reopens_preserved_history_without_source_paths() -> Result<()>
{
    let mut fixture = Fixture::new()?;
    let history = history::seed(&mut fixture)?;
    let (partition, restored) = restore_and_partition(&fixture)?;
    assert!(partition["retained"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == "task-retained"));
    let backup = fixture.temp.path().join("bundle");
    let archived = data_backup::inventory(&backup)?;
    let offline = fixture.temp.path().join("offline-host");
    let original = data_backup::inventory(&offline)?;
    let unavailable = fixture.temp.path().join("unavailable-host");
    fs::rename(&offline, &unavailable)?;
    for index in 0..2 {
        fs::rename(
            fixture.temp.path().join(format!("game-{index}")),
            fixture
                .temp
                .path()
                .join(format!("unavailable-game-{index}")),
        )?;
    }
    unopened(&restored)?;
    let paths = native_tools(&fixture.temp.path().join("tools"))?;
    let settings = fixture.temp.path().join("tools.json");
    let mut wrong = paths.clone();
    wrong["codex"] = paths["godot"].clone();
    fs::write(&settings, serde_json::to_vec(&wrong)?)?;
    let error = project_migration_activation::activate_copy(&backup, &restored, Some(&settings))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("required destination tool unavailable: codex"),
        "{error:#}"
    );
    unopened(&restored)?;
    assert!(host(&restored)?
        .get::<Value>("toolSetup", "main")?
        .is_none());
    assert_eq!(data_backup::inventory(&backup)?, archived);

    fs::write(&settings, serde_json::to_vec(&paths)?)?;
    let result = project_migration_activation::activate_copy(&backup, &restored, Some(&settings))?;
    assert_eq!(result["ready_to_activate"], true);
    assert_eq!(result["live_model_request_made"], false);
    assert_eq!(result["default_data_directory_changed"], false);
    assert_eq!(result["retained_tasks_marked"], 1);
    assert!(!restored.join(".beaver-migration-pending").exists());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(restored.join("ACTIVATION.json"))?)?,
        result
    );
    migration_bundle::ensure_activated(&restored.join("data"))?;
    assert_eq!(result["tools"].as_array().unwrap().len(), 4);
    for tool in result["tools"].as_array().unwrap() {
        assert_eq!(tool["available"], true);
        let path = PathBuf::from(tool["path"].as_str().unwrap());
        assert_eq!(
            path.canonicalize()?,
            Path::new(paths[tool["name"].as_str().unwrap()].as_str().unwrap()).canonicalize()?
        );
        assert_eq!(
            fs::read_to_string(path.with_extension("probed"))?,
            "--version\n"
        );
    }

    let mut snapshot = None;
    for _ in 0..2 {
        let host = Arc::new(Mutex::new(host(&restored)?));
        assert_eq!(
            host.lock()
                .unwrap()
                .get::<Value>("toolSetup", "main")?
                .unwrap()["status"],
            "completed"
        );
        let router = ProjectStorageRouter::new(host.clone());
        assert_eq!(router.open_registered_local()?.len(), 2);
        let current = recovered(&history, &router, &host)?;
        if let Some(previous) = &snapshot {
            assert_eq!(&current, previous);
        }
        snapshot = Some(current);
        router.close_all()?;
        // Every runtime, database and lock is dropped before constructing the next router.
    }
    assert_eq!(data_backup::inventory(&backup)?, archived);
    assert_eq!(data_backup::inventory(&unavailable)?, original);
    Ok(())
}
