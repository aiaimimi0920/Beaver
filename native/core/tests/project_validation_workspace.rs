use anyhow::Result;
use beaver_core::{project_storage::ProjectStore, task_plan, validation::task_completion};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[test]
fn integration_children_keep_relative_workspaces_through_preparation_and_reopen() -> Result<()> {
    for repair in [false, true] {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("game");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        fs::write(root.join("story.md"), "integrated story")?;
        let child = {
            let runtime = ProjectStore::initialize(&root, "p")?.into_runtime();
            let files = runtime.files();
            let handle = runtime.store();
            let mut store = handle.lock().unwrap();
            store.put("project", "p", &json!({"id":"p","path":root}))?;
            let mut parent = json!({
                "id":"parent","projectId":"p","title":"Story","prompt":"Write a story",
                "status":"failed","integrationValidation":true,"autoAccept":false,
                "subtaskIds":[],"validationRepair":{
                    "engineVersion":"4.4.1","snapshotId":"snapshot","code":"gut","error":"failed"
                }
            });
            store.put("task", "parent", &parent)?;
            if repair {
                task_completion::repair(&mut store, &files, &mut parent)?;
            } else {
                task_completion::steered(&mut store, &files, &mut parent, "Add another chapter")?;
            }
            assert_eq!(parent["status"], "waitingChildren");
            assert_eq!(parent["subtaskIds"].as_array().unwrap().len(), 1);
            let id = parent["subtaskIds"][0].as_str().unwrap();
            let mut child: Value = store.get("task", id)?.unwrap();
            let location = format!(".beaver/workspaces/{id}");
            assert_eq!(child["workspace"], location);
            assert_eq!(child["workspacePrepared"], false);
            assert!(!files.workspace(id)?.exists());
            assert_eq!(child["origin"], if repair { "system" } else { "user" });
            if repair {
                assert_eq!(child["validationRepair"], parent["validationRepair"]);
                assert_eq!(child["validationRepairAttempts"], 1);
            } else {
                assert_eq!(child["prompt"], "Add another chapter");
            }
            task_plan::prepare(&mut store, &files, &mut child)?;
            assert_eq!(child["workspace"], location);
            assert_eq!(child["workspacePrepared"], true);
            let workspace = files.resolve_workspace(id, Path::new(&location))?;
            assert_eq!(
                fs::read_to_string(workspace.join("story.md"))?,
                "integrated story"
            );
            let context: Value = serde_json::from_slice(&fs::read(
                workspace.join(".beaver-context/parent/task.json"),
            )?)?;
            assert_eq!(context["prompt"], parent["prompt"]);
            store.put("task", id, &child)?;
            child
        };
        let runtime = ProjectStore::open(&root, "p")?.into_runtime();
        let id = child["id"].as_str().unwrap();
        assert_eq!(
            runtime
                .store()
                .lock()
                .unwrap()
                .get::<Value>("task", id)?
                .unwrap(),
            child
        );
        let workspace = runtime
            .files()
            .resolve_workspace(id, Path::new(child["workspace"].as_str().unwrap()))?;
        assert_eq!(
            fs::read_to_string(workspace.join("story.md"))?,
            "integrated story"
        );
    }
    Ok(())
}
