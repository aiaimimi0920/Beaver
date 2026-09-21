use anyhow::Result;
use beaver_core::{
    executor::{Execution, Outcome},
    files::Files,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
    task_finish,
    validation::task_gate,
};
use serde_json::{json, Value};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn project(root: &Path) -> Result<(ProjectRuntime, Value)> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(root, "project")?.into_runtime();
    let workspace = runtime.files().workspace("task")?;
    fs::create_dir(&workspace)?;
    let task = json!({"id":"task","projectId":"project","status":"running",
        "capability":"code","workspace":runtime.files().workspace_location("task")?,"baseline":{},"prompt":"Change source"});
    runtime.store().lock().unwrap().put(
        "project",
        "project",
        &json!({"id":"project","path":root}),
    )?;
    runtime.store().lock().unwrap().put("task", "task", &task)?;
    Ok((runtime, task))
}

#[test]
fn resolution_requires_existing_owned_workcopy_without_creating_one() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, _) = project(&temp.path().join("game"))?;
    let files = runtime.files();
    let owned = files.workspace("task")?;
    assert_eq!(
        files.resolve_workspace("task", Path::new(".beaver/workspaces/task"))?,
        owned
    );
    let missing = files.workspace("missing")?;
    assert!(files
        .resolve_workspace("missing", Path::new(".beaver/workspaces/missing"))
        .is_err());
    assert!(!missing.exists());
    let file = files.workspace("file")?;
    fs::write(&file, "not a workspace")?;
    assert!(files
        .resolve_workspace("file", Path::new(".beaver/workspaces/file"))
        .is_err());
    for id in ["", ".", "..", ".codex", "a/b", "a\\b", "a:b"] {
        assert!(files.resolve_workspace(id, &owned).is_err(), "{id}");
        assert!(files.workspace_location(id).is_err(), "{id}");
    }
    for alias in [
        owned.to_str().unwrap(),
        "workspaces/task",
        ".beaver/workspaces/other-task",
        ".beaver/workspaces/../workspaces/task",
        ".beaver/workspaces//task",
        ".beaver/workspaces/task/",
        "./.beaver/workspaces/task",
        ".beaver/workspaces/./task",
        ".beaver\\workspaces\\task",
    ] {
        assert!(
            files.resolve_workspace("task", Path::new(alias)).is_err(),
            "{alias}"
        );
    }
    let legacy = Files::new(temp.path().to_owned());
    let historical = temp.path().join("historical-work-copy");
    assert_eq!(legacy.resolve_workspace("task", &historical)?, historical);
    assert_eq!(
        Path::new(&legacy.workspace_location("task")?),
        legacy.workspace("task")?
    );
    Ok(())
}

#[test]
fn validation_and_finish_cannot_normalize_or_capture_foreign_sources() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, mut task) = project(&temp.path().join("game"))?;
    let outside = temp.path().join("foreign");
    fs::create_dir(&outside)?;
    let bytes = b"\xef\xbb\xbfvar untouched = 1\n";
    fs::write(outside.join("foreign.gd"), bytes)?;
    task["workspace"] = json!(outside);
    let files = runtime.files();
    let error = task_gate::changes(&files, &task).unwrap_err();
    assert!(
        error.to_string().contains("不属于当前项目或任务"),
        "{error:#}"
    );
    let store = runtime.store();
    let mut db = store.lock().unwrap();
    db.put("task", "task", &task)?;
    let failed = task_finish::finish(
        &mut db,
        &files,
        "task",
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(failed["status"], "failed");
    assert!(failed["error"]
        .as_str()
        .unwrap()
        .contains("不属于当前项目或任务"));
    assert_eq!(fs::read(outside.join("foreign.gd"))?, bytes);
    assert!(!runtime
        .project_root()
        .join(".beaver/content/blobs")
        .exists());
    assert!(!runtime.project_root().join("foreign.gd").exists());
    assert!(db.list::<Value>("operation")?.is_empty());
    drop(db);
    // A valid workcopy still normalizes and captures the actual task's changed source.
    let workspace = files.workspace("task")?;
    task["workspace"] = json!(files.workspace_location("task")?);
    fs::write(workspace.join("own.gd"), bytes)?;
    let changes = task_gate::changes(&files, &task)?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "own.gd");
    assert_eq!(fs::read(workspace.join("own.gd"))?, b"var untouched = 1\n");
    Ok(())
}

#[tokio::test]
async fn executor_rejects_foreign_workspace_before_spawning_command() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, mut task) = project(&temp.path().join("game"))?;
    let outside = temp.path().join("foreign");
    fs::create_dir(&outside)?;
    task["workspace"] = json!(outside);
    runtime.store().lock().unwrap().put("task", "task", &task)?;
    let marker = outside.join("started");
    let mut command = tokio::process::Command::new(std::env::current_exe()?);
    command
        .args(["--ignored", "--exact", "spawn_probe", "--nocapture"])
        .env("BEAVER_WORKSPACE_PROBE", &marker);
    let execution = Execution {
        store: runtime.store(),
        files: runtime.files(),
        task_id: "task".into(),
        model: "fixture".into(),
        prompt: "Change source".into(),
        ask_user_tool: Value::Null,
        max_minutes: 1,
        secrets: vec![],
    };
    let (_sender, receiver) = tokio::sync::mpsc::channel(1);
    let outcome = execution.run(command, receiver).await;
    assert!(
        matches!(outcome, Outcome::Failed(ref error) if error.contains("不属于当前项目或任务")),
        "{outcome:?}"
    );
    assert!(!marker.exists());
    Ok(())
}

#[test]
#[ignore]
fn spawn_probe() -> Result<()> {
    fs::write(std::env::var("BEAVER_WORKSPACE_PROBE")?, "started")?;
    Ok(())
}
