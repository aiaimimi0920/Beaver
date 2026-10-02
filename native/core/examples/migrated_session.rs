//! Isolated real-Codex migration probe. Never accepts unmarked user data.
use anyhow::{ensure, Context, Result};
use beaver_core::{
    execution_settings::ExecutionSettings,
    executor::{Execution, Outcome},
    files::Files,
    launch, preferences,
    store::Store,
    task_finish,
};
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

#[tokio::main]
async fn main() -> Result<()> {
    let upgrade = std::env::args().nth(3).as_deref() == Some("upgrade");
    let root = fs::canonicalize(PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("fixture data required")?,
    ))?;
    let fixture: Value =
        serde_json::from_slice(&fs::read(root.join("migration-session-fixture.json"))?)?;
    ensure!(
        fixture["fixture"] == true,
        "only isolated fixtures are accepted"
    );
    let id = fixture["taskId"]
        .as_str()
        .context("fixture task missing")?
        .to_owned();
    let media = fs::canonicalize(PathBuf::from(
        std::env::args_os()
            .nth(2)
            .context("Beaver executable required")?,
    ))?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join(".beaver-native.lock"))?;
    lock.try_lock_exclusive()?;
    let store = Arc::new(Mutex::new(Store::open(&root)?));
    let (mut task, settings) = {
        let db = store.lock().unwrap();
        (
            db.get::<Value>("task", &id)?.context("task missing")?,
            db.get::<Value>("settings", "main")?
                .context("settings missing")?,
        )
    };
    let endpoint = url::Url::parse(
        settings["local"]["code"]["baseUrl"]
            .as_str()
            .context("fixture URL missing")?,
    )?;
    ensure!(
        endpoint.scheme() == "http" && endpoint.host_str() == Some("127.0.0.1"),
        "only local fixture Responses API accepted"
    );
    let original_thread = task["threadId"]
        .as_str()
        .context("old real thread required")?
        .to_owned();
    task["status"] = json!("running");
    store.lock().unwrap().put("task", &id, &task)?;
    let resources = launch::Resources {
        skills: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/skills"),
        media,
        catalog: serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
        ask_user_tool: serde_json::from_str(include_str!(
            "../../../dist-native/ask-user-tool.json"
        ))?,
    };
    let mut tools = BTreeMap::new();
    for name in ["codex", "godot", "blender", "node"] {
        let path = beaver_core::tools::find(name, settings["tools"][name].as_str().unwrap_or(""))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "not installed".into());
        tools.insert(name.into(), path);
    }
    let execution_settings = ExecutionSettings::read(
        &store.lock().unwrap(),
        &preferences::SystemVault,
        settings,
        Some(
            task["capability"]
                .as_str()
                .context("task capability missing")?,
        ),
    )?;
    let files = Arc::new(Files::new(root.clone()));
    let launch = launch::prepare(
        &files,
        &store.lock().unwrap(),
        &execution_settings,
        &task,
        &resources,
        &tools,
        std::env::vars_os().collect(),
    )?;
    let execution = Execution {
        store: store.clone(),
        files: files.clone(),
        task_id: id.clone(),
        model: launch.model,
        prompt: if upgrade {
            "BEAVER_UPGRADE_TURN. Reply without editing any files."
        } else {
            "BEAVER_RELOCATED_TURN. Continue the previous session in this working directory."
        }
        .into(),
        ask_user_tool: launch.ask_user_tool,
        max_minutes: 2,
        secrets: launch.secrets,
    };
    let (_sender, receiver) = tokio::sync::mpsc::channel(8);
    let outcome = execution
        .run(launch.command.context("Missing Codex command")?, receiver)
        .await;
    ensure!(
        outcome == Outcome::Completed,
        "real relocated session failed: {outcome:?}"
    );
    let task = task_finish::finish(
        &mut store.lock().unwrap(),
        &files,
        &id,
        outcome,
        &AtomicBool::new(false),
    )?;
    if upgrade {
        ensure!(
            task["threadId"] != original_thread,
            "tool upgrade reused incompatible thread"
        );
        ensure!(
            task["priorCallbackThreadId"] == original_thread,
            "legacy thread identity lost"
        );
    } else {
        ensure!(
            task["threadId"] == original_thread,
            "resume replaced the compatible thread"
        );
    }
    ensure!(
        task["status"] == "completed",
        "resumed result not committed"
    );
    println!(
        "{}",
        json!({"passed":true,"threadId":task["threadId"],"priorCallbackThreadId":task["priorCallbackThreadId"],"status":task["status"],"workspace":task["workspace"]})
    );
    Ok(())
}
