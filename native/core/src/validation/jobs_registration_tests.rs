use super::*;
use crate::{project_work_gate::ProjectWorkGate, store::Store, validation::test_support::Fixture};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};

#[test]
fn old_snapshot_rechecks_registration_and_admitted_run_keeps_original_store() -> Result<()> {
    let fixture = Fixture::project()?;
    let mut queued = fixture.run(None)?;
    queued.managed = true;
    queued.status = "queued".into();
    fixture.save(&queued)?;
    fixture.store.put(
        "validationFeedback",
        "feedback",
        &json!({"id":"feedback","projectId":"p","status":"reviewQueued","taskId":"review"}),
    )?;
    fixture.store.put(
        "task",
        "review",
        &json!({"id":"review","status":"completed","report":"done"}),
    )?;
    let host = Arc::new(Mutex::new(Store::open(&fixture._temp.path().join("host"))?));
    let registration = json!({"id":"p"});
    host.lock().unwrap().put("project", "p", &registration)?;
    host.lock()
        .unwrap()
        .put("validationRun", &queued.id, &queued)?;
    let project = Storage {
        store: Arc::new(Mutex::new(fixture.store)),
        files: Arc::new(fixture.files),
        project_id: Some("p".into()),
        draining: false,
        work_gate: ProjectWorkGate::registered(host.clone(), "p"),
    };
    let fallback = Storage {
        store: host.clone(),
        files: project.files.clone(),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    let snapshot = project.clone();
    let tool_host = host.clone();
    let tool_store = project.store.clone();
    let changed_host = host.clone();
    let state = State {
        store: host.clone(),
        files: project.files.clone(),
        storage: Arc::new(|_| anyhow::bail!("registration removed")),
        enumerate: Arc::new(move || Ok(vec![snapshot.clone(), fallback.clone()])),
        resolve: Arc::new(move |context| {
            assert!(tool_host.try_lock().is_ok());
            assert!(tool_store.try_lock().is_ok());
            assert_eq!(context.project["id"], "p");
            anyhow::bail!("original project tools")
        }),
        changed: Arc::new(move || assert!(changed_host.try_lock().is_ok())),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    host.lock().unwrap().remove("project", "p")?;
    let local = BTreeSet::from(["p".into()]);
    assert!(!coordinate(&project, &local)?);
    assert!(claim(&state, "code")?.is_none());
    assert!(state.active.lock().unwrap().is_empty());
    let feedback = || -> Result<Value> {
        repository::get(
            &project.store.lock().unwrap(),
            "validationFeedback",
            "feedback",
        )
    };
    assert_eq!(feedback()?["status"], "reviewQueued");
    host.lock().unwrap().put("project", "p", &registration)?;
    assert!(coordinate(&project, &local)?);
    assert_eq!(feedback()?["status"], "reviewReady");
    let (mut run, token, pinned) = claim(&state, "code")?.expect("re-registered project");
    assert!(Arc::ptr_eq(&pinned.store, &project.store));
    host.lock().unwrap().remove("project", "p")?;
    assert_eq!(
        execute(&state, &pinned, &mut run, &token)
            .unwrap_err()
            .to_string(),
        "original project tools"
    );
    run.status = "failed".into();
    save(&state, &pinned, &run)?;
    let original: Run = repository::get(&project.store.lock().unwrap(), "validationRun", &run.id)?;
    let shadow: Run = repository::get(&host.lock().unwrap(), "validationRun", &run.id)?;
    assert_eq!(original.status, "failed");
    assert_eq!(shadow.status, "queued");
    Ok(())
}
