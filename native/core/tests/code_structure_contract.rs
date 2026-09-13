use anyhow::Result;
use beaver_core::{
    code_structure, executor::Outcome, files::Files, media::Media, store::Store, task_finish,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    files: Files,
    project: PathBuf,
    workspace: PathBuf,
}

fn source(count: usize) -> String {
    (0..count)
        .map(|i| format!("var item_{i} = {i}\n"))
        .collect()
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let store = Store::open(&data)?;
        let files = Files::new(data);
        let project = temp.path().join("project");
        let workspace = temp.path().join("workspace");
        fs::create_dir(&project)?;
        fs::write(project.join("legacy.gd"), source(800))?;
        fs::write(project.join("note.txt"), "before")?;
        let baseline = files.capture(&project)?;
        files.restore_copy(&baseline, &workspace)?;
        store.put("project", "p", &json!({"id":"p","path":project}))?;
        store.put("task", "t", &json!({"id":"t","projectId":"p","workspace":workspace,"baseline":baseline,"status":"running","capability":"code","prompt":"Build the game"}))?;
        Ok(Self {
            _temp: temp,
            store,
            files,
            project,
            workspace,
        })
    }

    fn finish(&mut self) -> Result<Value> {
        task_finish::finish(
            &mut self.store,
            &self.files,
            "t",
            Outcome::Completed,
            &AtomicBool::new(false),
        )
    }
}

#[test]
fn code_structure_rejects_the_entire_merge_and_allows_split_recovery() -> Result<()> {
    let mut f = Fixture::new()?;
    fs::write(f.workspace.join("game.gd"), source(701))?;
    fs::write(f.workspace.join("note.txt"), "after")?;
    let rejected = f.finish()?;
    assert_eq!(rejected["status"], "failed");
    assert_eq!(rejected["codeStructure"]["ok"], false);
    assert!(rejected["error"].as_str().unwrap().contains("game.gd: 701"));
    assert!(!f.project.join("game.gd").exists());
    assert_eq!(fs::read_to_string(f.project.join("note.txt"))?, "before");
    assert_eq!(
        fs::read_to_string(f.workspace.join("game.gd"))?,
        source(701)
    );
    assert!(f.store.list::<Value>("operation")?.is_empty());

    // Stand in for the next executor turn after the ordinary continue action.
    beaver_core::task_actions::continue_task(&mut f.store, "t", "Split the game source", false)?;
    let mut resumed: Value = f.store.get("task", "t")?.unwrap();
    resumed["status"] = json!("running");
    f.store.put("task", "t", &resumed)?;
    fs::remove_file(f.workspace.join("game.gd"))?;
    for name in ["player.gd", "enemies.gd", "hud.gd", "scene.gd"] {
        fs::write(f.workspace.join(name), source(180))?;
    }
    let completed = f.finish()?;
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["codeStructure"]["ok"], true);
    assert_eq!(
        fs::read_to_string(f.project.join("player.gd"))?,
        source(180)
    );
    assert_eq!(
        fs::read_to_string(f.project.join("legacy.gd"))?,
        source(800)
    );
    assert_eq!(fs::read_to_string(f.project.join("note.txt"))?, "after");
    Ok(())
}

#[test]
fn code_structure_does_not_bless_same_length_legacy_edits() -> Result<()> {
    let mut f = Fixture::new()?;
    fs::write(
        f.workspace.join("legacy.gd"),
        source(800).replace("item_0 = 0", "item_0 = 1"),
    )?;
    let rejected = f.finish()?;
    assert_eq!(rejected["status"], "failed");
    assert!(rejected["error"]
        .as_str()
        .unwrap()
        .contains("legacy.gd: 800"));
    assert_eq!(
        fs::read_to_string(f.project.join("legacy.gd"))?,
        source(800)
    );
    Ok(())
}

#[test]
fn code_structure_merge_retry_uses_captured_output_not_a_rewritten_workspace() -> Result<()> {
    let mut f = Fixture::new()?;
    fs::write(f.workspace.join("game.gd"), source(701))?;
    let mut task: Value = f.store.get("task", "t")?.unwrap();
    let baseline = serde_json::from_value(task["baseline"].clone())?;
    task["changes"] = json!(Files::changes(&baseline, &f.files.capture(&f.workspace)?));
    task["status"] = json!("conflict");
    task["conflicts"] = json!(["game.gd"]);
    f.store.put("task", "t", &task)?;
    fs::write(f.workspace.join("game.gd"), source(10))?;
    let rejected = task_finish::retry_merge(&mut f.store, &f.files, "t", &AtomicBool::new(false))?;
    assert_eq!(rejected["status"], "failed");
    assert!(rejected["error"].as_str().unwrap().contains("game.gd: 701"));
    assert!(!f.project.join("game.gd").exists());
    Ok(())
}

#[tokio::test]
async fn code_structure_mcp_preflight_is_read_only_and_needs_no_media_credentials() -> Result<()> {
    let f = Fixture::new()?;
    fs::write(f.workspace.join("game.gd"), source(701))?;
    let before = f.files.capture(&f.workspace)?;
    let media = Media::new(f.workspace.clone(), "{}")?;
    let report: Value =
        serde_json::from_str(&media.call(code_structure::tool::NAME, json!({})).await?)?;
    assert_eq!(report["ok"], false);
    assert!(report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("game.gd: 701")));
    assert_eq!(before, f.files.capture(&f.workspace)?);
    assert!(media
        .call(code_structure::tool::NAME, json!({"root":"outside"}))
        .await
        .is_err());
    Ok(())
}
