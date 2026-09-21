use anyhow::Result;
use beaver_core::{
    assets, project_runtime::ProjectRuntime, project_storage::ProjectStore, reveal, task_resources,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn project(root: &Path, id: &str) -> Result<(ProjectRuntime, Value)> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let runtime = ProjectStore::initialize(root, "project")?.into_runtime();
    let workspace = runtime.files().workspace(id)?;
    fs::create_dir(&workspace)?;
    fs::write(workspace.join("preview.txt"), b"owned preview")?;
    let task = json!({"id":id,"workspace":runtime.files().workspace_location(id)?,"baseline":{},"references":[],"changes":[]});
    runtime.store().lock().unwrap().put("task", id, &task)?;
    runtime
        .store()
        .lock()
        .unwrap()
        .put("project", id, &json!({"path":root}))?;
    Ok((runtime, task))
}

#[test]
fn project_readers_use_owned_workcopy_and_keep_project_asset_semantics() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let (runtime, task) = project(&temp.path().join("game"), &id)?;
    let files = runtime.files();
    let store = runtime.store();
    let db = store.lock().unwrap();
    assert_eq!(
        task_resources::resources(&files, &task)?,
        json!([{"path":"preview.txt","origin":"修改","exists":true}])
    );
    assert_eq!(
        task_resources::text(&files, &task, "preview.txt")?,
        "owned preview"
    );
    assert_eq!(
        task_resources::raw(&files, &task, "preview.txt")?["base64"],
        "b3duZWQgcHJldmlldw=="
    );
    let uri = format!("/task-{id}/preview.txt");
    let path = assets::resolve_asset(&db, &files, &uri)?;
    assert_eq!(
        assets::read_asset(&path, None, false)?.bytes,
        b"owned preview"
    );
    assert_eq!(
        reveal::resolve(&db, &files, "task.reveal", &json!({"id":id}))?.path,
        fs::canonicalize(files.workspace(&id)?)?
    );
    let project_file = assets::resolve_asset(&db, &files, &format!("/{id}/project.godot"))?;
    assert_eq!(fs::read(project_file)?, b"config_version=5\n");
    assert_eq!(
        reveal::resolve(&db, &files, "project.reveal", &json!({"id":id}))?.path,
        fs::canonicalize(runtime.project_root())?
    );
    for relative in ["../project.godot", "C:/outside", "a\\b"] {
        assert!(task_resources::text(&files, &task, relative).is_err());
        assert!(task_resources::raw(&files, &task, relative).is_err());
        assert!(assets::resolve_asset(&db, &files, &format!("/task-{id}/{relative}")).is_err());
    }
    Ok(())
}

#[test]
fn moved_project_reopens_task_readers_without_rewriting_workspace_records() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let original = temp.path().join("original");
    let moved = temp.path().join("moved");
    let task = {
        let (_, task) = project(&original, &id)?;
        task
    };
    fs::rename(&original, &moved)?;
    let runtime = ProjectStore::open(&moved, "project")?.into_runtime();
    let files = runtime.files();
    let handle = runtime.store();
    let store = handle.lock().unwrap();
    let persisted: Value = store.get("task", &id)?.unwrap();
    assert_eq!(persisted, task);
    assert_eq!(persisted["workspace"], format!(".beaver/workspaces/{id}"));
    let workspace =
        files.resolve_workspace(&id, Path::new(persisted["workspace"].as_str().unwrap()))?;
    assert!(workspace.starts_with(runtime.project_root()));
    assert_eq!(
        task_resources::resources(&files, &persisted)?,
        json!([{"path":"preview.txt","origin":"修改","exists":true}])
    );
    assert_eq!(
        task_resources::text(&files, &persisted, "preview.txt")?,
        "owned preview"
    );
    assert_eq!(
        task_resources::raw(&files, &persisted, "preview.txt")?["base64"],
        "b3duZWQgcHJldmlldw=="
    );
    let path = assets::resolve_asset(&store, &files, &format!("/task-{id}/preview.txt"))?;
    assert_eq!(path, workspace.join("preview.txt"));
    assert_eq!(
        assets::read_asset(&path, None, false)?.bytes,
        b"owned preview"
    );
    assert_eq!(
        reveal::resolve(&store, &files, "task.reveal", &json!({"id":id}))?.path,
        fs::canonicalize(&workspace)?
    );
    assert!(!original.exists());
    Ok(())
}

#[test]
fn foreign_task_and_project_workcopies_cannot_be_listed_read_previewed_or_revealed() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let id = uuid::Uuid::new_v4().to_string();
    let (runtime, mut task) = project(&temp.path().join("game"), &id)?;
    let (other, _) = project(&temp.path().join("other"), &id)?;
    let files = runtime.files();
    let sibling = files.workspace(&uuid::Uuid::new_v4().to_string())?;
    fs::create_dir(&sibling)?;
    fs::write(sibling.join("preview.txt"), b"owned preview")?;
    let store = runtime.store();
    let db = store.lock().unwrap();
    // Identical bytes in another workcopy do not establish task ownership.
    for foreign in [sibling, other.files().workspace(&id)?] {
        task["workspace"] = json!(foreign);
        db.put("task", &id, &task)?;
        let errors = [
            task_resources::resources(&files, &task).unwrap_err(),
            task_resources::text(&files, &task, "preview.txt").unwrap_err(),
            task_resources::raw(&files, &task, "preview.txt").unwrap_err(),
            assets::resolve_asset(&db, &files, &format!("/task-{id}/preview.txt")).unwrap_err(),
            reveal::resolve(&db, &files, "task.reveal", &json!({"id":id}))
                .err()
                .unwrap(),
        ];
        for error in errors {
            assert!(
                error.to_string().contains("不属于当前项目或任务"),
                "{error:#}"
            );
        }
        assert_eq!(db.get::<Value>("task", &id)?.unwrap(), task);
        assert_eq!(fs::read(foreign.join("preview.txt"))?, b"owned preview");
    }
    Ok(())
}
