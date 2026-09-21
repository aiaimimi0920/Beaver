use super::*;

#[test]
fn create_project_task_writes_project_storage_and_indexes_route() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &project)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;

    let task = create_project_task(
        &router,
        Some(json!({
            "projectId": "project-a",
            "prompt": "制作测试任务",
            "title": "测试任务"
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();

    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    let project_task = runtime
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .expect("task is stored in project database");
    assert_eq!(project_task["projectId"], "project-a");
    assert_eq!(task["workspace"], format!(".beaver/workspaces/{task_id}"));
    assert!(root
        .join(format!(".beaver/workspaces/{task_id}/project.godot"))
        .is_file());
    assert_eq!(router.runtime_for_task(task_id)?.project_id(), "project-a");
    Ok(())
}

#[test]
fn create_project_task_requires_open_project_runtime() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let router = ProjectStorageRouter::new(host);

    let error = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"制作测试任务"})),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("项目未打开：project-a"), "{error}");
    Ok(())
}

#[test]
fn create_related_task_writes_project_storage_and_indexes_route() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "project-a")?;
    let host = host_store(temp.path())?;
    let project = register_project(&host, "project-a", &root)?;
    initialize_project_store(&root, "project-a", &project)?;
    let router = ProjectStorageRouter::new(host.clone());
    let runtime = router.open_registered("project-a")?;
    let parent = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"父任务","title":"父任务"})),
    )?;
    let parent_id = parent["id"].as_str().unwrap();

    let child = create_related_task(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.delegate",
        Some(json!({"id":parent_id,"text":"制作独立子目标"})),
    )?;
    let child_id = child["id"].as_str().unwrap();

    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", child_id)?
        .is_none());
    let project_child = runtime
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", child_id)?
        .expect("child task is stored in project database");
    assert_eq!(project_child["parentTaskId"], parent_id);
    assert_eq!(project_child["relation"], "child");
    assert_eq!(child["workspace"], format!(".beaver/workspaces/{child_id}"));
    assert!(root
        .join(format!(".beaver/workspaces/{child_id}/project.godot"))
        .is_file());
    assert_eq!(router.runtime_for_task(child_id)?.project_id(), "project-a");
    Ok(())
}

#[test]
fn create_related_task_keeps_legacy_host_tasks_available() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "legacy-project")?;
    let host = host_store(temp.path())?;
    let project = register_project(&host, "legacy-project", &root)?;
    host.lock().unwrap().put(
        "task",
        "legacy-parent",
        &json!({
            "id":"legacy-parent",
            "projectId":"legacy-project",
            "title":"旧任务",
            "prompt":"旧任务",
            "status":"running"
        }),
    )?;
    let router = ProjectStorageRouter::new(host.clone());

    let child = create_related_task(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.delegate",
        Some(json!({"id":"legacy-parent","text":"旧库子任务"})),
    )?;
    let child_id = child["id"].as_str().unwrap();

    let host_child = host
        .lock()
        .unwrap()
        .get::<Value>("task", child_id)?
        .expect("legacy child remains in host database");
    assert_eq!(project["id"], "legacy-project");
    assert_eq!(host_child["parentTaskId"], "legacy-parent");
    assert!(host_root(temp.path())
        .join(format!("workspaces/{child_id}/project.godot"))
        .is_file());
    assert!(router.runtime_for_task(child_id).is_err());
    Ok(())
}

#[test]
fn create_related_task_requires_registered_project_or_legacy_host_task() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let router = ProjectStorageRouter::new(host.clone());

    let error = create_related_task(
        &router,
        host,
        &host_root(temp.path()),
        "task.delegate",
        Some(json!({"id":"missing-task","text":"子任务"})),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("任务未注册：missing-task"), "{error}");
    Ok(())
}

#[test]
fn task_resource_bytes_uses_project_workspace() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({
            "projectId": "project-a",
            "prompt": "读取二进制资源",
            "title": "二进制资源"
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let workspace = root.join(format!(".beaver/workspaces/{task_id}"));
    fs::write(workspace.join("binary.bin"), [0_u8, 1, 255])?;

    let result = task_resource_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.resourceBytes",
        Some(json!({"id": task_id, "path": "binary.bin"})),
    )?;

    assert_eq!(result["path"], "binary.bin");
    assert_eq!(result["bytes"], 3);
    assert_eq!(result["base64"], "AAH/");
    assert!(runtime
        .store()
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_some());
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    Ok(())
}

#[test]
fn task_rollback_uses_project_files_and_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    fs::write(root.join("scene.txt"), "before")?;
    let task = create_project_task(
        &router,
        Some(json!({
            "projectId": "project-a",
            "prompt": "修改场景",
            "title": "回滚场景"
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let workspace = root.join(format!(".beaver/workspaces/{task_id}"));
    fs::write(workspace.join("scene.txt"), "after")?;

    let files = runtime.files();
    let before: beaver_core::files::Snapshot = serde_json::from_value(task["baseline"].clone())?;
    let after = files.capture(&workspace)?;
    let changes = beaver_core::files::Files::changes(&before, &after);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "scene.txt");
    fs::write(root.join("scene.txt"), "after")?;

    let project_store = runtime.store();
    let mut completed = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    completed["status"] = json!("completed");
    completed["accepted"] = json!(true);
    completed["changes"] = serde_json::to_value(&changes)?;
    project_store
        .lock()
        .unwrap()
        .put("task", task_id, &completed)?;

    let result = complete_task_action(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.rollback",
        Some(json!({"id": task_id, "keep": []})),
    )?;
    assert_eq!(result, Value::Null);
    assert_eq!(fs::read_to_string(root.join("scene.txt"))?, "before");

    let rolled_back = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    assert_eq!(rolled_back["status"], "rolledBack");
    assert_eq!(rolled_back["accepted"], false);
    assert_eq!(rolled_back["retainedFiles"], json!([]));
    let events = project_store.lock().unwrap().events(task_id)?;
    assert!(events.iter().any(|event| event.kind == "rollback"));
    assert!(host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .is_none());
    Ok(())
}

#[test]
fn task_retry_merge_uses_project_files_and_store() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    fs::write(root.join("scene.txt"), "before")?;
    let task = create_project_task(
        &router,
        Some(json!({
            "projectId": "project-a",
            "prompt": "重试合入",
            "title": "重试合入",
            "decompose": false
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let project_store = runtime.store();
    let mut running = project_store
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .unwrap();
    running["status"] = json!("running");
    project_store
        .lock()
        .unwrap()
        .put("task", task_id, &running)?;
    let workspace = root.join(format!(".beaver/workspaces/{task_id}"));
    fs::write(workspace.join("scene.txt"), "after")?;
    fs::write(workspace.join("generated.txt"), "generated")?;
    fs::write(root.join("scene.txt"), "human")?;

    let finish_result = {
        let store_handle = runtime.store();
        let mut store = store_handle.lock().unwrap();
        beaver_core::task_finish::finish(
            &mut store,
            runtime.files().as_ref(),
            task_id,
            beaver_core::executor::Outcome::Completed,
            &std::sync::atomic::AtomicBool::new(false),
        )?
    };
    assert_eq!(finish_result["status"], "conflict");

    host.lock().unwrap().put(
        "task",
        task_id,
        &json!({
            "id": task_id,
            "projectId": "host-shadow",
            "status": "host-shadow"
        }),
    )?;

    let first_retry = retry_merge_task(
        &router,
        host.clone(),
        &host_root(temp.path()),
        &std::sync::atomic::AtomicBool::new(false),
        Some(json!({"id": task_id})),
    )?;
    assert_eq!(first_retry["status"], "conflict");
    assert_eq!(fs::read_to_string(root.join("scene.txt"))?, "human");
    assert!(!root.join("generated.txt").exists());

    fs::write(root.join("scene.txt"), "after")?;
    let second_retry = retry_merge_task(
        &router,
        host.clone(),
        &host_root(temp.path()),
        &std::sync::atomic::AtomicBool::new(false),
        Some(json!({"id": task_id})),
    )?;
    assert_eq!(second_retry["status"], "completed");
    assert_eq!(fs::read_to_string(root.join("generated.txt"))?, "generated");

    let host_shadow = host
        .lock()
        .unwrap()
        .get::<Value>("task", task_id)?
        .expect("host shadow task remains isolated");
    assert_eq!(host_shadow["projectId"], "host-shadow");
    assert_eq!(host_shadow["status"], "host-shadow");
    Ok(())
}

#[test]
fn task_operations_reject_host_fallback_when_local_runtime_is_closed() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({
            "projectId": "project-a",
            "prompt": "关闭 Runtime 后拒绝回退",
            "title": "回退保护"
        })),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let workspace = root.join(format!(".beaver/workspaces/{task_id}"));
    fs::write(workspace.join("generated.txt"), "project resource")?;
    drop(runtime);
    router.close("project-a")?;

    host.lock().unwrap().put(
        "task",
        task_id,
        &json!({
            "id": task_id,
            "projectId": "project-a",
            "status": "completed",
            "accepted": false,
            "workspace": format!("workspaces/{task_id}"),
            "baseline": {"files": []},
            "changes": []
        }),
    )?;

    let resource_error = task_resource_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.resourceText",
        Some(json!({"id":task_id,"path":"generated.txt"})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        resource_error.contains("禁止回退到宿主存储"),
        "{resource_error}"
    );

    let events_error = task_events(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        events_error.contains("禁止回退到宿主存储"),
        "{events_error}"
    );

    let callback_error = task_callback_state(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        callback_error.contains("禁止回退到宿主存储"),
        "{callback_error}"
    );

    let reveal_error = match resolve_task_reveal(
        &router,
        host.clone(),
        &host_root(temp.path()),
        Some(json!({"id":task_id})),
    ) {
        Ok(_) => anyhow::bail!("task.reveal unexpectedly used a closed runtime"),
        Err(error) => error.to_string(),
    };
    assert!(
        reveal_error.contains("禁止回退到宿主存储"),
        "{reveal_error}"
    );

    let settings_error = task_setting_operation(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.direction",
        Some(json!({"id":task_id,"direction":"visual"})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        settings_error.contains("禁止回退到宿主存储"),
        "{settings_error}"
    );

    let accept_error = complete_task_action(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "task.accept",
        Some(json!({"id":task_id})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        accept_error.contains("禁止回退到宿主存储"),
        "{accept_error}"
    );

    let retry_error = retry_merge_task(
        &router,
        host.clone(),
        &host_root(temp.path()),
        &std::sync::atomic::AtomicBool::new(false),
        Some(json!({"id":task_id})),
    )
    .unwrap_err()
    .to_string();
    assert!(retry_error.contains("禁止回退到宿主存储"), "{retry_error}");

    let child_error = create_related_task(
        &router,
        host,
        &host_root(temp.path()),
        "task.delegate",
        Some(json!({"id":task_id,"text":"不应回退"})),
    )
    .unwrap_err()
    .to_string();
    assert!(child_error.contains("禁止回退到宿主存储"), "{child_error}");
    Ok(())
}
