use super::*;

#[test]
fn discovers_the_same_frozen_catalog_without_registering_or_opening_sources() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    for registered in [false, true] {
        if registered {
            f.register_source()?;
        }
        let before = inventory(&f.source)?;
        let registrations = f.host.lock().unwrap().list::<Value>("project")?;
        let discovered = f.sources.inspect_object_source(&f.source, None, None)?;
        let expected = f
            .sources
            .inspect_object_source(&f.source, Some("source-1"), None)?;
        assert_eq!(discovered.project["id"], "source-1");
        assert_eq!(discovered.objects.len(), 1);
        assert!(discovered.import_versions[0].source_digest.is_some());
        assert_eq!(
            serde_json::to_value(discovered)?,
            serde_json::to_value(expected)?
        );
        assert_eq!(inventory(&f.source)?, before);
        assert!(f.sources.runtimes()?.is_empty());
        assert_eq!(
            f.host.lock().unwrap().list::<Value>("project")?,
            registrations
        );
    }
    Ok(())
}

#[test]
fn discovery_reuses_an_open_committed_snapshot_without_mutating_running_work() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.register_source()?;
    let runtime = f.sources.open_registered("source-1")?;
    let handle = runtime.store();
    let changes = {
        let store = handle.lock().unwrap();
        store.put(
            "task",
            "running-task",
            &json!({
                "id":"running-task", "projectId":"source-1", "status":"running"
            }),
        )?;
        store.connection.total_changes()
    };
    let registrations = f.host.lock().unwrap().list::<Value>("project")?;
    let discovered = f.sources.inspect_object_source(&f.source, None, None)?;
    let expected = f
        .sources
        .inspect_object_source(&f.source, Some("source-1"), None)?;
    assert_eq!(discovered.project["id"], "source-1");
    assert_eq!(
        serde_json::to_value(discovered)?,
        serde_json::to_value(expected)?
    );
    let store = handle.lock().unwrap();
    assert_eq!(store.connection.total_changes(), changes);
    assert_eq!(
        store.get::<Value>("task", "running-task")?.unwrap()["status"],
        "running"
    );
    assert_eq!(f.sources.runtimes()?.len(), 1);
    assert_eq!(
        f.host.lock().unwrap().list::<Value>("project")?,
        registrations
    );
    Ok(())
}

#[test]
fn discovery_retains_identity_with_an_empty_or_filtered_catalog() -> Result<()> {
    for objects in [
        vec![],
        vec![record("hero", vec![accepted("hero", "hero-v1")])],
    ] {
        let f = Fixture::new(objects)?;
        let snapshot = f
            .sources
            .inspect_object_source(&f.source, None, Some("absent"))?;
        assert_eq!(snapshot.project["id"], "source-1");
        assert!(snapshot.objects.is_empty());
        assert!(snapshot.import_versions.is_empty());
        assert!(f.sources.runtimes()?.is_empty());
    }
    Ok(())
}

#[test]
fn missing_and_invalid_manifests_fail_without_repairing_the_source() -> Result<()> {
    for manifest in [None, Some("{"), Some("{}"), Some("[]")] {
        let f = Fixture::new(vec![])?;
        let path = f.source.join(".beaver/project.json");
        if let Some(contents) = manifest {
            fs::write(&path, contents)?;
        } else {
            fs::remove_file(&path)?;
        }
        let before = inventory(&f.source)?;
        assert!(f
            .sources
            .inspect_object_source(&f.source, None, None)
            .is_err());
        assert_eq!(inventory(&f.source)?, before);
        assert!(f.host.lock().unwrap().list::<Value>("project")?.is_empty());
        assert!(f.sources.runtimes()?.is_empty());
    }
    let f = Fixture::new(vec![])?;
    let plain = f.temp.path().join("ordinary-directory");
    fs::create_dir(&plain)?;
    fs::write(plain.join("asset.txt"), "ordinary file")?;
    let before = inventory(&plain)?;
    assert!(f.sources.inspect_object_source(&plain, None, None).is_err());
    assert_eq!(inventory(&plain)?, before);
    assert!(!plain.join(".beaver").exists());
    Ok(())
}
