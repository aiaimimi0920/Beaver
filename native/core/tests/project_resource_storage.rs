use anyhow::Result;
use beaver_core::{
    assets, documents, journal::Journal, project_runtime::ProjectRuntime,
    project_storage::ProjectStore, task_create,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn runtime(root: &Path, id: &str) -> Result<ProjectRuntime> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(root, id)?.into_runtime();
    runtime
        .store()
        .lock()
        .unwrap()
        .put("project", id, &json!({"id":id,"path":root}))?;
    Ok(runtime)
}

#[test]
fn document_edits_and_recovery_use_the_project_journal_and_content() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("game");
    let runtime = runtime(&root, "p")?;
    fs::write(root.join("story.md"), "before")?;
    let original = documents::read_document(&root, "story.md")?;
    {
        let files = runtime.files();
        let handle = runtime.store();
        let mut store = handle.lock().unwrap();
        documents::save_document(
            &mut store,
            &files,
            "p",
            "story.md",
            "after",
            original["revision"].as_str(),
        )?;
        assert_eq!(fs::read_to_string(root.join("story.md"))?, "after");
        assert!(documents::save_document(
            &mut store,
            &files,
            "p",
            "story.md",
            "stale",
            original["revision"].as_str()
        )
        .is_err());
        let tasks = store.list::<Value>("task")?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["status"], "completed");
        assert_eq!(tasks[0]["baseline"]["story.md"], original["revision"]);
        let task_id = tasks[0]["id"].as_str().unwrap();
        assert_eq!(
            tasks[0]["workspace"],
            format!(".beaver/workspaces/{task_id}")
        );
        let mut operation = store.list::<Value>("operation")?.remove(0);
        assert_eq!(operation["state"], "complete");
        assert_eq!(operation["taskAfter"]["workspace"], tasks[0]["workspace"]);
        // Recreate an interrupted write; reopening must recover using retained project blobs.
        fs::write(root.join("story.md"), "before")?;
        operation["state"] = json!("applying");
        store.put("operation", operation["id"].as_str().unwrap(), &operation)?;
    }
    drop(runtime);
    let reopened = ProjectStore::open(&root, "p")?.into_runtime();
    let handle = reopened.store();
    let files = reopened.files();
    let mut store = handle.lock().unwrap();
    let mut journal = Journal::new(&mut store, &files);
    assert!(journal.blocked("p")?);
    journal.recover()?;
    assert!(!journal.blocked("p")?);
    assert_eq!(fs::read_to_string(root.join("story.md"))?, "after");
    assert_eq!(store.list::<Value>("operation")?[0]["state"], "complete");
    let recovered = store.list::<Value>("task")?.remove(0);
    let workspace = files.resolve_workspace(
        recovered["id"].as_str().unwrap(),
        Path::new(recovered["workspace"].as_str().unwrap()),
    )?;
    assert_eq!(fs::read_to_string(workspace.join("story.md"))?, "after");
    assert!(!root.join(".beaver/blobs").exists());
    Ok(())
}

#[test]
fn pending_project_journal_blocks_all_adapted_write_entrypoints() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let runtime = runtime(&temp.path().join("game"), &id)?;
    let root = runtime.project_root();
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle.lock().unwrap();
    let source = temp.path().join("reference.txt");
    fs::write(&source, "source")?;
    let png = b"\x89PNG\r\n\x1a\nfixture";
    store.put(
        "operation",
        "pending",
        &json!({
            "id":"pending","projectId":id,"taskId":"t","kind":"merge",
            "state":"applying","changes":[],"taskAfter":{}
        }),
    )?;
    assert!(assets::import_files(&mut store, &files, &id, &[source.clone()]).is_err());
    assert!(assets::save_capture(&mut store, &files, &id, png).is_err());
    assert!(documents::save_document(&mut store, &files, &id, "new.md", "draft", None).is_err());
    assert!(task_create::create(
        &mut store,
        &files,
        json!({"projectId":id,"prompt":"Blocked"}),
        &Value::Null,
        &Value::Null
    )
    .is_err());
    assert!(store.list::<Value>("task")?.is_empty());
    assert!(!root.join("references").exists());
    assert!(!root.join("new.md").exists());
    assert!(fs::read_dir(root.join(".beaver/workspaces"))?
        .next()
        .is_none());
    assert!(!root.join(".beaver/content/blobs").exists());
    store.remove("operation", "pending")?;
    let imported = assets::import_files(&mut store, &files, &id, &[source])?;
    assert_eq!(imported.len(), 1);
    assert_eq!(fs::read_to_string(root.join(&imported[0]))?, "source");
    let capture = assets::save_capture(&mut store, &files, &id, png)?;
    assert_eq!(fs::read(root.join(capture))?, png);
    assert!(!root.join(".beaver/blobs").exists());
    Ok(())
}

#[test]
fn workspace_lookup_rejects_path_components_without_creating_directories() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let runtime = runtime(&temp.path().join("game"), "p")?;
    let files = runtime.files();
    for invalid in [
        "",
        ".",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        "a:stream",
        "a\0b",
    ] {
        assert!(files.workspace(invalid).is_err(), "accepted {invalid:?}");
    }
    let workspace = files.workspace("valid-task")?;
    assert_eq!(
        workspace,
        runtime.project_root().join(".beaver/workspaces/valid-task")
    );
    assert!(!workspace.exists());
    assert!(
        fs::read_dir(runtime.project_root().join(".beaver/workspaces"))?
            .next()
            .is_none()
    );
    Ok(())
}
