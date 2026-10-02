use super::*;

#[tokio::test]
async fn recovery_revokes_runs_and_never_replays_a_started_receipt() -> Result<()> {
    let f = Fixture::new()?;
    let request = f.request("lost", 0, "unknown.txt", "never replay");
    {
        let mut store = f.store.lock().unwrap();
        let records::Admission::New(mut receipt) = records::begin(&mut store, &request, "tool")?
        else {
            panic!("new request")
        };
        receipt["status"] = json!("started");
        records::save(&store, &receipt)?;
        store.recover_tasks()?;
    }
    assert!(f.registry.tool(request.clone()).await.is_err());
    assert_eq!(
        f.registry
            .receipt(&f.live.task_id, &f.live.run_id, "lost")?["status"],
        "unknown"
    );
    assert_eq!(
        read_receipt(
            &f.store.lock().unwrap(),
            &f.live.task_id,
            &f.live.run_id,
            "lost"
        )?["status"],
        "unknown"
    );
    assert_eq!(
        run_history(&f.store.lock().unwrap(), &f.live.task_id)?["runs"][0]["status"],
        "recoveryRequired"
    );
    let fresh = ExternalRegistry::new();
    assert!(fresh.tool(request.clone()).await.is_err());
    {
        let mut store = f.store.lock().unwrap();
        assert!(
            crate::task_actions::continue_task(&mut store, &f.live.task_id, "continue", false)
                .is_err()
        );
        let mut resumed: Value = store.get("task", &f.live.task_id)?.unwrap();
        resumed["status"] = json!("queued");
        assert!(!crate::task_plan::eligible(&resumed, &[resumed.clone()]));
        assert!(crate::task_plan::prepare(&mut store, &f.files, &mut resumed).is_err());
        resumed["status"] = json!("running");
        store.put("task", &f.live.task_id, &resumed)?;
    }
    assert!(fresh
        .register(
            &f.task,
            f.store.clone(),
            f.files.clone(),
            String::new(),
            None,
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
    assert!(fresh.tool(request).await.is_err());
    assert!(!f.live.workspace.join("unknown.txt").exists());
    Ok(())
}

#[tokio::test]
async fn cleanup_failure_retains_ownership_and_blocks_workspace_reuse() -> Result<()> {
    let f = Fixture::new()?;
    assert!(f
        .live
        .record_cleanup("revoked", Err(anyhow::anyhow!("injected cleanup timeout")))
        .is_err());
    assert!(f.registry.remove_live(&f.live).is_err());
    f.registry.cancel_all()?;
    assert!(f.registry.release_closed().is_err());
    assert!(f
        .registry
        .live(&f.live.task_id, Some(&f.live.run_id))
        .is_ok());
    {
        let mut store = f.store.lock().unwrap();
        let failed = crate::task_finish::finish(
            &mut store,
            &f.files,
            &f.live.task_id,
            Outcome::Failed("cleanup unconfirmed".into()),
            &AtomicBool::new(false),
        )?;
        assert!(failed["externalRecoveryRequired"].is_object());
        assert!(
            crate::task_actions::continue_task(&mut store, &f.live.task_id, "retry", false)
                .is_err()
        );
        assert_eq!(
            run_history(&store, &f.live.task_id)?["runs"][0]["status"],
            "recoveryRequired"
        );
        store.recover_tasks()?;
        assert!(crate::task_actions::continue_task(
            &mut store,
            &f.live.task_id,
            "retry again",
            false
        )
        .is_err());
        let mut stale: Value = store.get("task", &f.live.task_id)?.unwrap();
        stale
            .as_object_mut()
            .unwrap()
            .remove("externalRecoveryRequired");
        stale["status"] = json!("running");
        store.put("task", &f.live.task_id, &stale)?;
        assert!(crate::task_actions::continue_task(
            &mut store,
            &f.live.task_id,
            "stale metadata",
            false
        )
        .is_err());
        assert!(crate::task_plan::prepare(&mut store, &f.files, &mut stale).is_err());
    }
    assert!(ExternalRegistry::new()
        .register(
            &f.task,
            f.store.clone(),
            f.files.clone(),
            String::new(),
            None,
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
    Ok(())
}

#[tokio::test]
async fn confirmed_cleanup_allows_explicit_continuation_after_restart() -> Result<()> {
    let f = Fixture::new()?;
    f.live.close("revoked").await?;
    f.registry.remove_live(&f.live)?;
    {
        let mut store = f.store.lock().unwrap();
        store.recover_tasks()?;
        crate::task_actions::continue_task(&mut store, &f.live.task_id, "continue", false)?;
        let mut task: Value = store.get("task", &f.live.task_id)?.unwrap();
        assert!(!crate::external_run_recovery::blocked(&task));
        task["status"] = json!("running");
        store.put("task", &f.live.task_id, &task)?;
    }
    let fresh = ExternalRegistry::new();
    let (run, _finish) = fresh.register(
        &f.task,
        f.store.clone(),
        f.files.clone(),
        String::new(),
        None,
        Arc::new(AtomicBool::new(false)),
    )?;
    assert_ne!(run.run_id, f.live.run_id);
    Ok(())
}

#[tokio::test]
async fn quarantined_child_cannot_be_claimed_or_resumed_through_its_parent() -> Result<()> {
    let f = Fixture::new()?;
    assert!(f
        .live
        .record_cleanup("revoked", Err(anyhow::anyhow!("unknown process")))
        .is_err());
    {
        let store = f.store.lock().unwrap();
        let mut child: Value = store.get("task", &f.live.task_id)?.unwrap();
        child["status"] = json!("queued");
        store.put("task", &f.live.task_id, &child)?;
    }
    let limit: crate::scheduler::ParallelLimit = Arc::new(|| Ok(1));
    assert!(
        crate::scheduler_runtime_ops::claim_next(&f.store, &f.files, 0, &limit)
            .map_err(anyhow::Error::msg)?
            .is_empty()
    );
    {
        let mut store = f.store.lock().unwrap();
        let parent = json!({"id":"parent","projectId":"p","status":"waitingChildren","planPaused":true,"prompt":"parent"});
        store.put("task", "parent", &parent)?;
        let mut child: Value = store.get("task", &f.live.task_id)?.unwrap();
        child["status"] = json!("interrupted");
        child["parentTaskId"] = json!("parent");
        child["workspacePrepared"] = json!(true);
        store.put("task", &f.live.task_id, &child)?;
        assert!(crate::task_actions::continue_task(&mut store, "parent", "", false).is_err());
        assert_eq!(store.get::<Value>("task", "parent")?.unwrap(), parent);
        assert_eq!(
            store.get::<Value>("task", &f.live.task_id)?.unwrap()["status"],
            "interrupted"
        );
    }
    Ok(())
}
