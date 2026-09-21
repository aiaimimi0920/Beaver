use super::*;
use crate::{
    store::Store,
    validation::{
        service::{Resolver, Storage, StorageEnumerator, StorageResolver},
        test_support::Fixture,
    },
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};

#[test]
fn resolver_runs_unlocked_with_project_snapshot_and_separate_host_preferences() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut run = fixture.run(None)?;
    fixture.store.put(
        "validationSettings",
        "p",
        &json!({"revision":7,"ffmpeg":"project-ffmpeg"}),
    )?;
    let host = Arc::new(Mutex::new(Store::open(&fixture.files.root().join("host"))?));
    host.lock()
        .unwrap()
        .put("settings", "main", &json!({"tools":{"godot":"host-godot"}}))?;
    host.lock().unwrap().put(
        "project",
        "p",
        &json!({"id":"p","name":"wrong host project"}),
    )?;
    host.lock().unwrap().put(
        "validationSettings",
        "p",
        &json!({"ffmpeg":"wrong-host-ffmpeg"}),
    )?;
    let store = Arc::new(Mutex::new(fixture.store));
    let files = Arc::new(fixture.files);
    let storage_store = store.clone();
    let storage_files = files.clone();
    let storage: StorageResolver = Arc::new(move |project_id| {
        Ok(Storage {
            store: storage_store.clone(),
            files: storage_files.clone(),
            project_id: Some(project_id.to_string()),
            draining: false,
            work_gate: Default::default(),
        })
    });
    let enumerate_store = store.clone();
    let enumerate_files = files.clone();
    let enumerate: StorageEnumerator = Arc::new(move || {
        Ok(vec![Storage {
            store: enumerate_store.clone(),
            files: enumerate_files.clone(),
            project_id: Some("p".into()),
            draining: false,
            work_gate: Default::default(),
        }])
    });
    let task_store = store.clone();
    let resolver: Resolver = Arc::new(move |context| {
        let tasks = task_store
            .try_lock()
            .expect("resolver must not hold the task store lock");
        let host = host
            .try_lock()
            .expect("host store must be independently lockable");
        let preferences: Value = host.get("settings", "main")?.unwrap();
        assert_eq!(preferences["tools"]["godot"], "host-godot");
        assert_eq!(context.project["name"], "Validation fixture");
        assert_eq!(context.settings.ffmpeg, "project-ffmpeg");
        assert_eq!(context.settings.revision, 7);
        tasks.put(
            "validationSettings",
            "p",
            &json!({"revision":8,"ffmpeg":"changed"}),
        )?;
        assert_eq!(context.settings.revision, 7);
        anyhow::bail!("resolver sentinel")
    });
    let state = State {
        store,
        files,
        storage,
        enumerate,
        resolve: resolver,
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    assert_eq!(
        execute(
            &state,
            &(state.storage)(&run.project_id)?,
            &mut run,
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .to_string(),
        "resolver sentinel"
    );
    assert_eq!(
        state
            .store
            .lock()
            .unwrap()
            .get::<Value>("validationSettings", "p")?
            .unwrap()["revision"],
        8
    );
    Ok(())
}

#[test]
fn resolver_can_share_the_legacy_store_without_reentrant_locking() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut run = fixture.run(None)?;
    let store = Arc::new(Mutex::new(fixture.store));
    let files = Arc::new(fixture.files);
    let storage_store = store.clone();
    let storage_files = files.clone();
    let storage: StorageResolver = Arc::new(move |_| {
        Ok(Storage {
            store: storage_store.clone(),
            files: storage_files.clone(),
            project_id: None,
            draining: false,
            work_gate: Default::default(),
        })
    });
    let enumerate_store = store.clone();
    let enumerate_files = files.clone();
    let enumerate: StorageEnumerator = Arc::new(move || {
        Ok(vec![Storage {
            store: enumerate_store.clone(),
            files: enumerate_files.clone(),
            project_id: None,
            draining: false,
            work_gate: Default::default(),
        }])
    });
    let shared = store.clone();
    let state = State {
        store,
        files,
        storage,
        enumerate,
        resolve: Arc::new(move |_| {
            let _guard = shared.try_lock().expect("legacy store remains locked");
            anyhow::bail!("shared-store sentinel")
        }),
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    assert_eq!(
        execute(
            &state,
            &(state.storage)(&run.project_id)?,
            &mut run,
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .to_string(),
        "shared-store sentinel"
    );
    Ok(())
}

#[test]
fn missing_project_stops_before_tool_resolution() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut run = fixture.run(None)?;
    run.project_id = "missing".into();
    let state = State {
        store: Arc::new(Mutex::new(fixture.store)),
        files: Arc::new(fixture.files),
        storage: Arc::new(|_| anyhow::bail!("missing project storage")),
        enumerate: Arc::new(|| Ok(Vec::new())),
        resolve: Arc::new(|_| panic!("missing project must not resolve tools")),
        changed: Arc::new(|| {}),
        stop: AtomicBool::new(false),
        active: Mutex::new(BTreeMap::new()),
    };
    let storage = Storage {
        store: state.store.clone(),
        files: state.files.clone(),
        project_id: None,
        draining: false,
        work_gate: Default::default(),
    };
    assert!(execute(&state, &storage, &mut run, &AtomicBool::new(false)).is_err());
    Ok(())
}
