use super::*;

#[test]
fn callback_state_preserves_public_validation_before_storage_lookup() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let host = host_store(temp.path())?;
    let router = ProjectStorageRouter::new(host.clone());
    for (input, expected) in [
        (None, "Missing field: id"),
        (Some(Value::Null), "Input must be an object"),
        (Some(json!({"id":42})), "Invalid or unknown field: id"),
        (
            Some(json!({"id":"missing","requestId":42})),
            "Invalid or unknown field: requestId",
        ),
        (
            Some(json!({"id":"missing","unknown":true})),
            "Invalid or unknown field: unknown",
        ),
    ] {
        let error =
            task_callback_state(&router, host.clone(), &host_root(temp.path()), input).unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
    assert!(router.runtimes()?.is_empty());
    let effects = crate::business_effects::data_effects("task.callbackState");
    assert!(!effects.notify && !effects.wake_scheduler);
    Ok(())
}

#[test]
fn callback_state_keeps_legacy_receipts_but_never_reads_closed_project_shadows(
) -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (_root, host, router, runtime) = open_project(&temp, "project-a")?;
    let task = create_project_task(
        &router,
        Some(json!({"projectId":"project-a","prompt":"Read the saved callback receipt"})),
    )?;
    let task_id = task["id"].as_str().unwrap();
    let request_id = "saved-receipt";
    let receipt = json!({"requestId":request_id,"response":{"revision":7}});
    let receipt_kind = format!("task-callback/{task_id}");
    {
        let local = runtime.store();
        let store = local.lock().unwrap();
        store.put("task-callback-revision", task_id, &7)?;
        store.put(&receipt_kind, request_id, &receipt)?;
    }
    let input = Some(json!({"id":task_id,"requestId":request_id}));
    let local = task_callback_state(
        &router,
        host.clone(),
        &host_root(temp.path()),
        input.clone(),
    )?;
    assert_eq!(local, json!({"revision":7,"receipt":receipt}));
    host.lock().unwrap().put("task", task_id, &task)?;
    drop(runtime);
    router.close("project-a")?;
    let error =
        task_callback_state(&router, host.clone(), &host_root(temp.path()), input).unwrap_err();
    assert!(error.to_string().contains("禁止回退到宿主存储"), "{error}");

    let legacy_root = project_root(temp.path(), "legacy-project")?;
    register_project(&host, "legacy-project", &legacy_root)?;
    {
        let store = host.lock().unwrap();
        store.put(
            "task",
            "legacy",
            &json!({"id":"legacy","projectId":"legacy-project","status":"interrupted"}),
        )?;
        store.put("task-callback-revision", "legacy", &7)?;
        store.put("task-callback/legacy", request_id, &receipt)?;
    }
    assert_eq!(
        task_callback_state(
            &router,
            host,
            &host_root(temp.path()),
            Some(json!({"id":"legacy","requestId":request_id})),
        )?,
        local
    );
    assert!(router.runtimes()?.is_empty());
    Ok(())
}
