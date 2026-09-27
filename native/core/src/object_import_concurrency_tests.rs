use super::*;
use rusqlite::{Connection, TransactionBehavior};
use std::{
    sync::{mpsc, Arc, Barrier},
    thread,
    time::Duration,
};

#[test]
fn managed_catalog_reads_one_committed_snapshot_during_writes() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.register_source()?;
    let runtime = f.sources.open_registered("source-1")?;
    {
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        store.put("project", "source-1", &json!({"id":"source-1","name":"0"}))?;
        let mut object = store.get::<ObjectRecord>("object", "hero")?.unwrap();
        object.name = "0".into();
        store.put("object", "hero", &object)?;
    }
    let mut writer = Connection::open(f.source.join(".beaver/project.sqlite"))?;
    writer.busy_timeout(Duration::from_secs(5))?;
    let source = f.sources.object_import_source(&f.source, "source-1")?;
    {
        let transaction = writer.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE entities SET value=json_set(value,'$.name','uncommitted') WHERE kind='project'",
            [],
        )?;
        let snapshot = source.snapshot()?;
        assert_eq!(snapshot.project["name"], "0");
        assert_eq!(snapshot.objects[0].name, "0");
        transaction.rollback()?;
    }
    let start = Arc::new(Barrier::new(2));
    let writer_start = start.clone();
    let worker = thread::spawn(move || -> Result<()> {
        writer_start.wait();
        for generation in 1..=40 {
            let transaction = writer.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let name = generation.to_string();
            transaction.execute(
                "UPDATE entities SET value=json_set(value,'$.name',?) WHERE kind='project'",
                [&name],
            )?;
            thread::yield_now();
            transaction.execute(
                "UPDATE entities SET value=json_set(value,'$.name',?) WHERE kind='object'",
                [&name],
            )?;
            transaction.commit()?;
        }
        Ok(())
    });
    start.wait();
    for _ in 0..40 {
        let snapshot = source.snapshot()?;
        assert_eq!(
            snapshot.project["name"].as_str(),
            Some(snapshot.objects[0].name.as_str())
        );
    }
    worker.join().unwrap()?;
    let snapshot = source.snapshot()?;
    assert_eq!(snapshot.project["name"], "40");
    assert_eq!(snapshot.objects[0].name, "40");
    Ok(())
}

#[test]
fn two_open_projects_can_prepare_imports_in_both_directions_concurrently() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.register_source()?;
    let target_root = f.temp.path().join("reverse");
    fs::create_dir(&target_root)?;
    fs::write(target_root.join("project.godot"), "config_version=5\n")?;
    let target = ProjectStore::initialize(&target_root, "target-1")?;
    target
        .store()
        .put("project", "target-1", &json!({"id":"target-1"}))?;
    let mut manifest = accepted("hero", "hero-v1");
    manifest["projectId"] = json!("target-1");
    let mut object = record("hero", vec![manifest]);
    object.project_id = "target-1".into();
    target.store().put("object", &object.id, &object)?;
    for version in &object.versions {
        target.store().put(
            "object_version",
            &version.version_id,
            &(object.id.clone(), version),
        )?;
    }
    drop(target);
    f.host.lock().unwrap().put(
        "project",
        "target-1",
        &json!({
            "id":"target-1", "path":target_root
        }),
    )?;
    let first = f.sources.open_registered("source-1")?;
    let second = f.sources.open_registered("target-1")?;
    let forward = f.inspected("hero-v1")?;
    let snapshot = f
        .sources
        .inspect_object_source(&target_root, Some("target-1"), None)?;
    let reverse = Request {
        target_project_id: "source-1".into(),
        source_project_id: "target-1".into(),
        source_path: target_root,
        source_digest: snapshot.import_versions[0].source_digest.clone().unwrap(),
        ..forward.clone()
    };
    let (send, receive) = mpsc::channel();
    let start = Arc::new(Barrier::new(2));
    let mut workers = vec![];
    for (mut request, target) in [(forward, second.store()), (reverse, first.store())] {
        let sources = f.sources.clone();
        let start = start.clone();
        let send = send.clone();
        workers.push(thread::spawn(move || {
            for iteration in 0..4 {
                request.request_id = format!("parallel-{iteration}");
                start.wait();
                send.send(prepare(&target, &sources, &request)).unwrap();
            }
        }));
    }
    drop(send);
    for _ in 0..8 {
        let result = receive
            .recv_timeout(Duration::from_secs(10))
            .context("bidirectional import preparation stalled")??;
        assert!(!result.ready_to_commit);
        assert_ne!(result.source_project_id, result.target_project_id);
    }
    for worker in workers {
        worker.join().unwrap();
    }
    for runtime in [first, second] {
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        assert_eq!(store.list::<Preparation>(PREPARATION_KIND)?.len(), 4);
        assert_eq!(store.list::<Value>("object")?.len(), 1);
        assert!(store.list::<Value>("task")?.is_empty());
    }
    Ok(())
}
