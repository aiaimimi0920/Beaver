use anyhow::Result;
use base64::Engine;
use beaver_core::{
    executor::Outcome, feature_tasks, files::Files, project_runtime::ProjectRuntime,
    project_storage::ProjectStore, task_actions, task_create, task_finish, task_plan,
    task_relations,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn runtime(root: &Path) -> Result<ProjectRuntime> {
    fs::create_dir(root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    fs::write(root.join("story.md"), "original")?;
    let runtime = ProjectStore::initialize(root, "p")?.into_runtime();
    runtime.store().lock().unwrap().put(
        "project",
        "p",
        &json!({"id":"p","path":root,"name":"Game"}),
    )?;
    Ok(runtime)
}

fn workspace(files: &Files, task: &Value) -> PathBuf {
    let id = task["id"].as_str().unwrap();
    assert_eq!(task["workspace"], format!(".beaver/workspaces/{id}"));
    files
        .resolve_workspace(id, Path::new(task["workspace"].as_str().unwrap()))
        .unwrap()
}

#[test]
fn creation_and_derivatives_keep_frozen_content_in_project_storage() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let runtime = runtime(&temp.path().join("game"))?;
    let root = runtime.project_root();
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle.lock().unwrap();
    let blueprints: Value =
        serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
    store.put(
        "project",
        "p",
        &json!({
            "id":"p","path":root,"name":"Game","blueprint":blueprints["default"],
            "blueprintRevision":7
        }),
    )?;
    let mut parent = task_create::create(
        &mut store,
        &files,
        json!({"projectId":"p","prompt":"Continue the story","decompose":false}),
        &Value::Null,
        &blueprints,
    )?;
    let id = parent["id"].as_str().unwrap().to_owned();
    assert_eq!(
        workspace(&files, &parent),
        root.join(".beaver/workspaces").join(&id)
    );
    assert_eq!(store.events(&id)?.len(), 1);
    let context: Value = serde_json::from_slice(&fs::read(
        workspace(&files, &parent).join(".beaver-context/project/blueprint.json"),
    )?)?;
    assert_eq!(context["revision"], 7);
    let before = serde_json::from_value(parent["baseline"].clone())?;
    fs::write(root.join("story.md"), "committed result")?;
    let after = files.capture(root)?;
    parent["changes"] = serde_json::to_value(Files::changes(&before, &after))?;
    parent["status"] = json!("completed");
    parent["report"] = json!("Story delivered");
    store.put("task", &id, &parent)?;
    fs::write(workspace(&files, &parent).join("draft.md"), "unmerged")?;
    fs::write(root.join("story.md"), "later human edit")?;
    assert_eq!(
        fs::read_to_string(workspace(&files, &parent).join("story.md"))?,
        "original"
    );

    let followup = task_relations::create(
        &mut store,
        &files,
        "task.followup",
        json!({"id":id,"text":"Add a chapter"}),
        &Value::Null,
        &blueprints,
    )?;
    assert_eq!(followup["parentTaskId"], id);
    assert_eq!(
        fs::read_to_string(workspace(&files, &followup).join("story.md"))?,
        "later human edit"
    );
    assert!(!workspace(&files, &followup).join("draft.md").exists());
    let context: Value = serde_json::from_slice(&fs::read(
        workspace(&files, &followup).join(".beaver-context/followup/task.json"),
    )?)?;
    assert_eq!(context["report"], parent["report"]);

    let rollback = task_relations::create(
        &mut store,
        &files,
        "task.dialogueRollback",
        json!({"id":id,"text":"Reconsider this change"}),
        &Value::Null,
        &blueprints,
    )?;
    let context = workspace(&files, &rollback).join(".beaver-context/rollback");
    assert_eq!(
        fs::read_to_string(context.join("before/story.md"))?,
        "original"
    );
    assert_eq!(
        fs::read_to_string(context.join("after/story.md"))?,
        "committed result"
    );
    assert_eq!(
        fs::read_to_string(root.join("story.md"))?,
        "later human edit"
    );
    assert_eq!(store.get::<Value>("task", &id)?.unwrap(), parent);
    for task in store.list::<Value>("task")? {
        assert!(workspace(&files, &task).starts_with(root.join(".beaver/workspaces")));
        let baseline: beaver_core::files::Snapshot =
            serde_json::from_value(task["baseline"].clone())?;
        assert!(baseline.keys().all(|path| !path.starts_with(".beaver/")));
        for hash in baseline.values() {
            assert!(root.join(".beaver/content/blobs").join(hash).is_file());
        }
    }
    assert!(!root.join(".beaver/blobs").exists());
    assert!(!temp.path().join("workspaces").exists());
    Ok(())
}

#[test]
fn planned_children_prepare_from_the_previous_project_commit() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let runtime = runtime(&temp.path().join("game"))?;
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle.lock().unwrap();
    let mut parent = task_create::create(
        &mut store,
        &files,
        json!({"projectId":"p","prompt":"Plan a story"}),
        &Value::Null,
        &Value::Null,
    )?;
    let id = parent["id"].as_str().unwrap().to_owned();
    parent["status"] = json!("running");
    parent["threadId"] = json!("thread");
    store.put("task", &id, &parent)?;
    task_plan::submit(
        &store,
        &id,
        &json!({"threadId":"thread","arguments":{
            "summary":"Write and review", "steps":[
                {"title":"Write","prompt":"Write story","direction":"story","acceptance":"Readable"},
                {"title":"Review","prompt":"Review story","direction":"review","acceptance":"Checked"}
            ]
        }}),
    )?;
    let parent = task_finish::finish(
        &mut store,
        &files,
        &id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(parent["status"], "waitingChildren");
    let ids: Vec<String> = serde_json::from_value(parent["subtaskIds"].clone())?;
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(store.list::<Value>("task")?.len(), 3);
    let mut first: Value = store.get("task", &ids[0])?.unwrap();
    let mut second: Value = store.get("task", &ids[1])?.unwrap();
    assert_eq!(first["workspace"], format!(".beaver/workspaces/{}", ids[0]));
    assert_eq!(
        second["workspace"],
        format!(".beaver/workspaces/{}", ids[1])
    );
    assert!(!files.workspace(&ids[1])?.exists());
    assert!(!task_plan::eligible(&second, &store.list("task")?));
    task_plan::prepare(&mut store, &files, &mut first)?;
    fs::write(
        workspace(&files, &first).join("chapter.md"),
        "approved chapter",
    )?;
    first["status"] = json!("running");
    store.put("task", &ids[0], &first)?;
    let first = task_finish::finish(
        &mut store,
        &files,
        &ids[0],
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(first["accepted"], true);
    assert!(task_plan::eligible(&second, &store.list("task")?));
    task_plan::prepare(&mut store, &files, &mut second)?;
    assert_eq!(
        workspace(&files, &second),
        runtime
            .project_root()
            .join(".beaver/workspaces")
            .join(&ids[1])
    );
    assert_eq!(
        fs::read_to_string(workspace(&files, &second).join("chapter.md"))?,
        "approved chapter"
    );
    let context: Value = serde_json::from_slice(&fs::read(
        workspace(&files, &second).join(".beaver-context/parent/task.json"),
    )?)?;
    assert_eq!(context["plan"], parent["plan"]);
    assert!(!runtime.project_root().join(".beaver/blobs").exists());
    Ok(())
}

#[test]
fn feature_upgrade_restores_previous_package_from_project_blobs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let runtime = runtime(&temp.path().join("game"))?;
    let files = runtime.files();
    let handle = runtime.store();
    let mut store = handle.lock().unwrap();
    let encode = |text: &str| base64::engine::general_purpose::STANDARD.encode(text);
    let mut sources = json!({"story":{
        "feature.json":encode(r#"{"id":"story","name":"Story","version":"1.0.0","description":"Story data"}"#),
        "story.md":encode("upstream one")
    }});
    let request = json!({"projectId":"p","featureId":"story"});
    let mut first = feature_tasks::create(
        &mut store,
        &files,
        request.clone(),
        &sources,
        &Value::Null,
        &Value::Null,
    )?;
    first["status"] = json!("completed");
    let id = first["id"].as_str().unwrap();
    store.put("task", id, &first)?;
    task_actions::accept(&mut store, id)?;
    sources["story"]["feature.json"] = json!(encode(
        r#"{"id":"story","name":"Story","version":"1.1.0","description":"Story data"}"#
    ));
    sources["story"]["story.md"] = json!(encode("upstream two"));
    let upgrade = feature_tasks::create(
        &mut store,
        &files,
        request,
        &sources,
        &Value::Null,
        &Value::Null,
    )?;
    let context = workspace(&files, &upgrade).join(".beaver-context/feature");
    assert_eq!(
        fs::read_to_string(context.join("old/story.md"))?,
        "upstream one"
    );
    assert_eq!(
        fs::read_to_string(context.join("new/story.md"))?,
        "upstream two"
    );
    assert_eq!(
        fs::read_to_string(runtime.project_root().join("story.md"))?,
        "original"
    );
    assert_eq!(
        store.get::<Value>("feature", "p:story")?.unwrap()["version"],
        "1.0.0"
    );
    assert!(!runtime.project_root().join(".beaver/blobs").exists());
    Ok(())
}
