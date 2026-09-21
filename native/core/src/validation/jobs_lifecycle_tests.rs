use super::*;
use crate::{store::Store, validation::test_support::Fixture};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};

#[test]
fn closed_project_history_is_not_claimed_or_coordinated() -> Result<()> {
    let fixture = Fixture::project()?;
    let mut queued = fixture.run(None)?;
    queued.managed = true;
    queued.status = "queued".into();
    queued.project_id = "orphan".into();
    fixture.save(&queued)?;
    let mut failed = queued.clone();
    failed.id = "failed-orphan".into();
    failed.status = "failed".into();
    failed.engine_version = "4.4".into();
    fixture.save(&failed)?;
    for (id, project) in [("orphan-feedback", "orphan"), ("legacy-feedback", "p")] {
        fixture.store.put(
            "validationFeedback",
            id,
            &json!({
                "id":id,"projectId":project,"status":"reviewQueued","taskId":"review"
            }),
        )?;
    }
    fixture.store.put(
        "task",
        "review",
        &json!({"id":"review","status":"completed","report":"done"}),
    )?;
    let coverage = json!({"id":"orphan-coverage","projectId":"orphan","status":"pending"});
    fixture
        .store
        .put("validationCoverage", "orphan-coverage", &coverage)?;
    let storage = Storage {
        store: Arc::new(Mutex::new(fixture.store)),
        files: Arc::new(fixture.files),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    let enumerated = storage.clone();
    let state = State {
        store: storage.store.clone(),
        files: storage.files.clone(),
        storage: Arc::new(|_| anyhow::bail!("unexpected resolver")),
        enumerate: Arc::new(move || Ok(vec![enumerated.clone()])),
        resolve: Arc::new(|_| anyhow::bail!("unexpected tools")),
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    assert!(claim(&state, "code")?.is_none());
    {
        let mut store = storage.store.lock().unwrap();
        let owned = owned_projects(&storage, &store, &BTreeSet::new())?;
        assert!(super::super::coordinator::refresh_matching(
            &mut store,
            &storage.files,
            |id| owned.contains(id)
        )?);
        let orphan: Value = repository::get(&store, "validationFeedback", "orphan-feedback")?;
        let legacy: Value = repository::get(&store, "validationFeedback", "legacy-feedback")?;
        assert_eq!(orphan["status"], "reviewQueued");
        assert_eq!(legacy["status"], "reviewReady");
        assert_eq!(
            repository::get::<Value>(&store, "validationCoverage", "orphan-coverage")?,
            coverage
        );
        assert!(store
            .get::<Value>("validationRepairDecision", &failed.id)?
            .is_none());
        assert!(owned_projects(&storage, &store, &BTreeSet::from(["p".into()]))?.is_empty());
        queued.id = "legacy-run".into();
        queued.project_id = "p".into();
        store.put("validationRun", &queued.id, &queued)?;
    }
    let (run, _, _) = claim(&state, "code")?.expect("registered legacy run");
    assert_eq!(run.id, "legacy-run");
    Ok(())
}

#[test]
fn draining_blocks_project_and_host_shadow_then_claim_pins_original_storage() -> Result<()> {
    let fixture = Fixture::project()?;
    let mut queued = fixture.run(None)?;
    queued.managed = true;
    queued.status = "queued".into();
    fixture.save(&queued)?;
    let host = Arc::new(Mutex::new(Store::open(&fixture._temp.path().join("host"))?));
    host.lock()
        .unwrap()
        .put("validationRun", &queued.id, &queued)?;
    let project = Storage {
        store: Arc::new(Mutex::new(fixture.store)),
        files: Arc::new(fixture.files),
        project_id: Some("p".into()),
        draining: false,
        work_gate: Default::default(),
    };
    let fallback = Storage {
        store: host.clone(),
        files: project.files.clone(),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    let draining = Arc::new(AtomicBool::new(true));
    let enumerate = {
        let project = project.clone();
        let draining = draining.clone();
        Arc::new(move || {
            let mut project = project.clone();
            project.draining = draining.load(Ordering::SeqCst);
            Ok(vec![project, fallback.clone()])
        })
    };
    let state = State {
        store: host.clone(),
        files: project.files.clone(),
        storage: Arc::new(|_| anyhow::bail!("project no longer registered")),
        enumerate,
        resolve: Arc::new(|_| anyhow::bail!("tool resolution reached original project")),
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    assert!(claim(&state, "code")?.is_none());
    draining.store(false, Ordering::SeqCst);
    let (mut run, token, pinned) = claim(&state, "code")?.expect("registered project claim");
    assert!(Arc::ptr_eq(&pinned.store, &project.store));
    draining.store(true, Ordering::SeqCst);
    assert_eq!(
        execute(&state, &pinned, &mut run, &token)
            .unwrap_err()
            .to_string(),
        "tool resolution reached original project"
    );
    run.status = "failed".into();
    save(&state, &pinned, &run)?;
    let original: Run = repository::get(&project.store.lock().unwrap(), "validationRun", &run.id)?;
    let shadow: Run = repository::get(&host.lock().unwrap(), "validationRun", &run.id)?;
    assert_eq!(original.status, "failed");
    assert_eq!(shadow.status, "queued");
    assert!(claim(&state, "code")?.is_none());
    Ok(())
}
