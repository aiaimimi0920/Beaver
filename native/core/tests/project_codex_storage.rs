use anyhow::Result;
use beaver_core::{
    code_structure::{self, Baseline},
    codex_home::{self, HomeRequest},
    execution_settings::ExecutionSettings,
    files::Files,
    launch::{self, Resources},
    preferences::Vault,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
    scheduler::Launch,
    store::Store,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, ffi::OsStr, fs, path::Path};

struct TestVault;
impl Vault for TestVault {
    fn encrypt(&self, _: &str) -> Result<String> {
        unreachable!()
    }
    fn decrypt(&self, _: &str) -> Result<String> {
        Ok("fixture-host-key".into())
    }
}

struct Host {
    store: Store,
    resources: Resources,
    tools: BTreeMap<String, String>,
}

impl Host {
    fn new(root: &Path) -> Result<Self> {
        let store = Store::open(root)?;
        let mut settings: Value =
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?;
        settings["mode"] = json!("local");
        settings["local"]["code"] =
            json!({"baseUrl":"https://host.example/v1","model":"host-model"});
        settings["mcp"] = json!({"godot":true,"blender":false});
        store.put("settings", "main", &settings)?;
        store.put("secret", "code", &"fixture-cipher")?;
        let skills = root.join("skills");
        fs::create_dir(&skills)?;
        let executable = std::env::current_exe()?;
        Ok(Self {
            store,
            resources: Resources {
                skills,
                media: executable.clone(),
                catalog: Value::Null,
                ask_user_tool: Value::Null,
            },
            tools: BTreeMap::from([
                ("codex".into(), executable.to_string_lossy().into_owned()),
                ("godot".into(), "host-godot-v1".into()),
            ]),
        })
    }

    fn prepare(&self, project: &ProjectRuntime, task: &Value) -> Result<Launch> {
        let settings = ExecutionSettings::read(&self.store, &TestVault, Value::Null, Some("code"))?;
        launch::prepare(
            &project.files(),
            &project.store().lock().unwrap(),
            &settings,
            task,
            &self.resources,
            &self.tools,
            BTreeMap::from([("OPENAI_API_KEY".into(), "discard-inherited-key".into())]),
        )
    }
}

fn project(root: &Path, id: &str) -> Result<(ProjectRuntime, Value)> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    // A frozen oversized source must be read from project blobs, not the live workspace.
    let source = (0..501)
        .map(|n| format!("var value_{n} = {n}\n"))
        .collect::<String>();
    fs::write(root.join("legacy.gd"), source)?;
    let runtime = ProjectStore::initialize(root, id)?.into_runtime();
    let files = runtime.files();
    let baseline = files.capture(root)?;
    let workspace = files.workspace("same-task")?;
    files.restore_copy(&baseline, &workspace)?;
    let task = json!({"id":"same-task","projectId":id,"capability":"code",
        "workspace":files.workspace_location("same-task")?,"baseline":baseline,"prompt":"Continue","references":[]});
    Ok((runtime, task))
}

#[test]
fn launch_uses_project_frozen_sources_and_host_environment() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::new(&temp.path().join("host"))?;
    let (runtime, task) = project(&temp.path().join("game"), "p")?;
    let files = runtime.files();
    let workspace = files.workspace("same-task")?;
    fs::write(workspace.join("legacy.gd"), "var changed = 1\n")?;
    let expected = code_structure::measure(
        "legacy.gd",
        &fs::read(runtime.project_root().join("legacy.gd"))?,
    )?;
    let launch = host.prepare(&runtime, &task)?;
    let command = launch.command.as_ref().unwrap().as_std();
    let environment: BTreeMap<_, _> = command.get_envs().collect();
    let home = files.codex_home("same-task")?;
    assert_eq!(
        home,
        runtime
            .project_root()
            .join(".beaver/workspaces/.codex/same-task")
    );
    assert_eq!(
        environment[OsStr::new("CODEX_HOME")],
        Some(home.as_os_str())
    );
    assert_eq!(command.get_current_dir(), Some(workspace.as_path()));
    assert_eq!(
        environment[OsStr::new("BEAVER_CODEX_KEY")],
        Some(OsStr::new("fixture-host-key"))
    );
    assert!(!environment.contains_key(OsStr::new("OPENAI_API_KEY")));
    let baseline = Baseline::parse(&fs::read(home.join("code-structure-baseline.json"))?)?;
    assert_eq!(baseline.files["legacy.gd"], expected);
    assert_eq!(baseline.files.len(), 1);
    for path in beaver_core::files::list_files(&home)? {
        let bytes = fs::read(home.join(path))?;
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("fixture-host-key") && !text.contains("discard-inherited-key"));
    }
    assert!(!runtime.project_root().join(".beaver/blobs").exists());
    assert!(!runtime.project_root().join(".beaver/codex").exists());
    assert!(!temp.path().join("host/codex").exists());
    Ok(())
}

#[test]
fn moved_project_prepares_launch_with_rebased_workcopy_and_home() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::new(&temp.path().join("host"))?;
    let original = temp.path().join("original");
    let moved = temp.path().join("moved");
    let task = {
        let (runtime, task) = project(&original, "p")?;
        host.prepare(&runtime, &task)?;
        runtime
            .store()
            .lock()
            .unwrap()
            .put("task", "same-task", &task)?;
        task
    };
    fs::rename(&original, &moved)?;
    let runtime = ProjectStore::open(&moved, "p")?.into_runtime();
    let persisted: Value = runtime
        .store()
        .lock()
        .unwrap()
        .get("task", "same-task")?
        .unwrap();
    assert_eq!(persisted, task);
    assert_eq!(persisted["workspace"], ".beaver/workspaces/same-task");
    let launch = host.prepare(&runtime, &persisted)?;
    let command = launch.command.as_ref().unwrap().as_std();
    let files = runtime.files();
    let workspace = files.workspace("same-task")?;
    let home = files.codex_home("same-task")?;
    assert!(workspace.starts_with(runtime.project_root()));
    assert!(home.starts_with(runtime.project_root()));
    assert_eq!(command.get_current_dir(), Some(workspace.as_path()));
    let environment: BTreeMap<_, _> = command.get_envs().collect();
    assert_eq!(
        environment[OsStr::new("CODEX_HOME")],
        Some(home.as_os_str())
    );
    assert!(home.join("config.toml").is_file());
    assert!(!original.exists());
    Ok(())
}

#[test]
fn relaunch_rebinds_host_configuration_and_preserves_isolated_sessions() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut host = Host::new(&temp.path().join("host"))?;
    let (first, task) = project(&temp.path().join("first"), "first")?;
    let (second, other) = project(&temp.path().join("second"), "second")?;
    host.prepare(&first, &task)?;
    host.prepare(&second, &other)?;
    let home = first.files().codex_home("same-task")?;
    let other_home = second.files().codex_home("same-task")?;
    assert_ne!(home, other_home);
    fs::create_dir(home.join("sessions"))?;
    let session = b"{\"opaque\":\"old/path, keep byte-for-byte\"}\n";
    fs::write(home.join("sessions/kept.jsonl"), session)?;
    let other_config = fs::read(other_home.join("config.toml"))?;
    host.tools.insert("godot".into(), "host-godot-v2".into());
    let baseline = serde_json::from_value(task["baseline"].clone())?;
    let files = first.files();
    files.restore_copy(&baseline, &files.workspace("same-task")?)?;
    host.prepare(&first, &task)?;
    assert_eq!(fs::read(home.join("sessions/kept.jsonl"))?, session);
    assert!(!other_home.join("sessions").exists());
    assert_eq!(fs::read(other_home.join("config.toml"))?, other_config);
    let config = fs::read_to_string(home.join("config.toml"))?;
    assert!(config.contains("host-godot-v2") && !config.contains("host-godot-v1"));
    assert!(fs::read_to_string(home.join("AGENTS.md"))?.contains("host-godot-v2"));
    assert!(!files
        .capture(first.project_root())?
        .keys()
        .any(|p| p.contains(".codex")));
    assert!(!files
        .capture(&files.workspace("same-task")?)?
        .keys()
        .any(|p| p.contains(".codex")));
    Ok(())
}

#[test]
fn missing_project_source_does_not_fall_back_to_legacy_blobs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::new(&temp.path().join("host"))?;
    let (runtime, task) = project(&temp.path().join("game"), "p")?;
    let hash = task["baseline"]["legacy.gd"].as_str().unwrap();
    let blob = runtime.files().blob(hash)?;
    let old_layout = runtime.project_root().join(".beaver/blobs");
    fs::create_dir(&old_layout)?;
    fs::rename(blob, old_layout.join(hash))?;
    assert!(host.prepare(&runtime, &task).is_err());
    assert!(!runtime
        .files()
        .codex_home("same-task")?
        .join("config.toml")
        .exists());
    Ok(())
}

#[test]
fn wrong_project_or_task_workspace_is_rejected_before_home_creation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Host::new(&temp.path().join("host"))?;
    let (runtime, mut task) = project(&temp.path().join("game"), "p")?;
    let (other, _) = project(&temp.path().join("other"), "other")?;
    let files = runtime.files();
    let sibling = files.workspace("another-task")?;
    fs::create_dir(&sibling)?;
    let baseline = serde_json::from_value(task["baseline"].clone())?;
    let settings = ExecutionSettings::read(&host.store, &TestVault, Value::Null, Some("code"))?;
    for wrong in [sibling, other.files().workspace("same-task")?] {
        task["workspace"] = json!(wrong);
        let error = host.prepare(&runtime, &task).err().unwrap();
        assert!(
            error.to_string().contains("不属于当前项目或任务"),
            "{error:#}"
        );
        let error = codex_home::prepare(
            &files,
            &settings,
            HomeRequest {
                task_id: "same-task",
                asset_task: false,
                retained_blender_port: None,
                workspace: &wrong,
                baseline: &baseline,
                skills: &host.resources.skills,
                media_executable: &host.resources.media,
                media_args: &[],
                resolved_tools: &host.tools,
            },
            BTreeMap::new(),
        )
        .err()
        .unwrap();
        assert!(
            error.to_string().contains("不属于当前项目或任务"),
            "{error:#}"
        );
        assert!(!files.codex_home("same-task")?.exists());
    }
    Ok(())
}

#[test]
fn home_resolution_rejects_invalid_task_ids_without_creating_directories() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (runtime, _) = project(&temp.path().join("game"), "p")?;
    let legacy = Files::new(temp.path().to_owned());
    for files in [&*runtime.files(), &legacy] {
        for id in [
            "",
            ".",
            "..",
            ".codex",
            "a/b",
            "a\\b",
            "a:b",
            "a\0b",
            "../outside",
        ] {
            assert!(files.codex_home(id).is_err(), "{id:?}");
        }
        assert!(!files.codex_home("valid_task-1")?.exists());
    }
    assert!(runtime.files().workspace(".codex").is_err());
    assert!(!runtime
        .project_root()
        .join(".beaver/workspaces/.codex")
        .exists());
    assert!(!temp.path().join("codex").exists());
    Ok(())
}
