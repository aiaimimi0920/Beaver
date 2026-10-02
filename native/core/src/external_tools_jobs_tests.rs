use super::*;

fn fixture(run: &str, status: &str, done: bool) -> Result<(String, Arc<Job>)> {
    let id = uuid::Uuid::new_v4().to_string();
    let job = Arc::new(Job {
        run_id: run.into(),
        cancelled: AtomicBool::new(false),
        done: AtomicBool::new(done),
        state: Mutex::new(
            json!({"jobId":id,"runId":run,"status":status,"largeLogTail":"x".repeat(65536)}),
        ),
        changed: Condvar::new(),
    });
    registry()
        .lock()
        .map_err(|_| lock_error())?
        .insert(id.clone(), job.clone());
    Ok((id, job))
}

#[test]
fn cleanup_reclaims_only_the_confirmed_run_and_is_idempotent() -> Result<()> {
    let run = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    let (first, _) = fixture(&run, "succeeded", true)?;
    let (second, _) = fixture(&run, "failed", true)?;
    let (unrelated, _) = fixture(&other, "succeeded", true)?;
    cleanup(&run)?;
    {
        let jobs = registry().lock().map_err(|_| lock_error())?;
        assert!(!jobs.contains_key(&first));
        assert!(!jobs.contains_key(&second));
        assert!(jobs.contains_key(&unrelated));
    }
    cleanup(&run)?;
    cleanup(&other)?;
    Ok(())
}

#[test]
fn uncertain_cleanup_preserves_every_job_in_the_run() -> Result<()> {
    let run = uuid::Uuid::new_v4().to_string();
    let (complete, _) = fixture(&run, "succeeded", true)?;
    let (unknown, _) = fixture(&run, "unknown", true)?;
    assert!(cleanup(&run).is_err());
    let mut jobs = registry().lock().map_err(|_| lock_error())?;
    assert!(jobs.contains_key(&complete));
    assert!(jobs.contains_key(&unknown));
    jobs.remove(&complete);
    jobs.remove(&unknown);
    Ok(())
}

#[test]
fn reclamation_rechecks_active_and_inconsistent_states() -> Result<()> {
    let run = uuid::Uuid::new_v4().to_string();
    let (id, job) = fixture(&run, "running", false)?;
    assert!(reclaim(&run).is_err());
    assert!(registry()
        .lock()
        .map_err(|_| lock_error())?
        .contains_key(&id));
    job.done.store(true, Ordering::SeqCst);
    assert!(reclaim(&run).is_err());
    registry().lock().map_err(|_| lock_error())?.remove(&id);
    Ok(())
}

#[test]
fn cleanup_waits_for_owned_worker_before_reclamation() -> Result<()> {
    let run = uuid::Uuid::new_v4().to_string();
    let (id, job) = fixture(&run, "running", false)?;
    let worker = std::thread::spawn(move || {
        while !job.cancelled.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut state = job.state.lock().unwrap();
        state["status"] = json!("cancelled");
        job.done.store(true, Ordering::SeqCst);
        job.changed.notify_all();
    });
    cleanup(&run)?;
    worker.join().unwrap();
    assert!(!registry()
        .lock()
        .map_err(|_| lock_error())?
        .contains_key(&id));
    Ok(())
}

#[test]
fn historical_success_cannot_release_an_uncertain_live_job() -> Result<()> {
    let run = uuid::Uuid::new_v4().to_string();
    let (id, _) = fixture(&run, "unknown", true)?;
    let directory = tempfile::tempdir()?;
    let request = uuid::Uuid::new_v4().to_string();
    let path = directory
        .path()
        .join(format!(".beaver-context/external/{run}/{request}/job.json"));
    std::fs::create_dir_all(path.parent().unwrap())?;
    let start = json!({"jobId":id,"runId":run,"requestId":request,"status":"running",
        "process":{"scriptSha256":"a".repeat(64),"executableSha256":"b".repeat(64)}});
    let mut saved = start.clone();
    saved["status"] = json!("succeeded");
    saved["result"] = json!({"ownedTreeCleanup":"completed","outputs":[]});
    std::fs::write(path, serde_json::to_vec(&saved)?)?;
    let evidence = super::super::historical_job_evidence(directory.path(), &start)?;
    assert_eq!(evidence["report"]["status"], "succeeded");
    assert!(!evidence["authorizesRecovery"].as_bool().unwrap());
    assert!(ensure_idle(&run).is_err());
    assert!(cleanup(&run).is_err());
    let mut jobs = registry().lock().map_err(|_| lock_error())?;
    assert!(jobs.contains_key(&id));
    jobs.remove(&id);
    Ok(())
}
