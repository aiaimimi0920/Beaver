use super::*;

#[test]
fn unregistered_closed_and_open_sources_share_frozen_preparations() -> Result<()> {
    let mut manifest = accepted("hero", "hero-v1");
    manifest["files"] = json!([file("hero.tscn", "accepted scene")]);
    let f = Fixture::new(vec![record("hero", vec![manifest])])?;
    f.blob("accepted scene")?;
    let before = inventory(&f.source)?;
    let mut request = f.inspected("hero-v1")?;
    let external = f.prepare(&request)?;
    assert_eq!(inventory(&f.source)?, before);
    assert!(f.host.lock().unwrap().list::<Value>("project")?.is_empty());
    assert!(f.sources.runtimes()?.is_empty());

    f.register_source()?;
    let registrations = f.host.lock().unwrap().list::<Value>("project")?;
    request.request_id = "registered-closed".into();
    assert_eq!(f.inspected("hero-v1")?.source_digest, request.source_digest);
    let closed = f.prepare(&request)?;
    assert_eq!(closed.versions, external.versions);
    assert_eq!(closed.source_digest, external.source_digest);
    assert_eq!(inventory(&f.source)?, before);
    assert_eq!(
        f.host.lock().unwrap().list::<Value>("project")?,
        registrations
    );
    assert!(f.sources.runtimes()?.is_empty());

    let runtime = f.sources.open_registered("source-1")?;
    let handle = runtime.store();
    let changes = {
        let store = handle.lock().unwrap();
        let mut working = store.get::<ObjectRecord>("object", "hero")?.unwrap();
        working.name = "Unaccepted name".into();
        working.references = serde_json::from_value(json!([reference("absent", "absent-v1")]))?;
        store.put("object", "hero", &working)?;
        store.put(
            "task",
            "active-task",
            &json!({
                "id":"active-task", "projectId":"source-1", "status":"running"
            }),
        )?;
        store.connection.total_changes()
    };
    fs::write(f.source.join("hero.tscn"), "unaccepted scene")?;
    request.request_id = "registered-open".into();
    assert_eq!(f.inspected("hero-v1")?.source_digest, request.source_digest);
    let opened = f.prepare(&request)?;
    assert_eq!(opened.versions, external.versions);
    assert_eq!(opened.source_digest, external.source_digest);
    assert!(!opened.ready_to_commit);
    {
        let store = handle.lock().unwrap();
        assert_eq!(store.connection.total_changes(), changes);
        assert_eq!(
            store.get::<Value>("task", "active-task")?.unwrap()["status"],
            "running"
        );
        assert!(store.list::<Value>(PREPARATION_KIND)?.is_empty());
    }
    assert_eq!(
        fs::read_to_string(f.source.join("hero.tscn"))?,
        "unaccepted scene"
    );
    assert_eq!(
        f.host.lock().unwrap().list::<Value>("project")?,
        registrations
    );
    f.assert_no_imported_entities()?;

    drop(handle);
    drop(runtime);
    f.sources.close("source-1")?;
    fs::rename(&f.source, f.temp.path().join("offline"))?;
    assert_eq!(f.prepare(&request)?, opened);
    Ok(())
}

#[test]
fn source_ownership_survives_snapshot_without_holding_the_store_lock() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.register_source()?;
    drop(f.sources.open_registered("source-1")?);
    let source = f.sources.object_import_source(&f.source, "source-1")?;
    assert!(f.sources.close("source-1").is_err());
    assert_eq!(source.snapshot()?.objects.len(), 1);
    let runtime = f.sources.runtime_for_project("source-1")?;
    assert!(runtime.store().try_lock().is_ok());
    drop(runtime);
    drop(source);
    f.sources.close("source-1")?;

    let before = inventory(&f.source)?;
    let source = f.sources.object_import_source(&f.source, "source-1")?;
    assert!(f.sources.open_registered("source-1").is_err());
    assert!(f.sources.runtimes()?.is_empty());
    assert_eq!(source.snapshot()?.objects.len(), 1);
    assert_eq!(inventory(&f.source)?, before);
    drop(source);
    drop(f.sources.open_registered("source-1")?);
    f.sources.close("source-1")?;
    Ok(())
}

#[test]
fn registered_identity_cannot_be_bypassed_with_a_manual_path() -> Result<()> {
    let f = Fixture::new(vec![])?;
    f.register_source()?;
    let other = f.temp.path().join("replacement");
    fs::create_dir(&other)?;
    fs::write(other.join("project.godot"), "config_version=5\n")?;
    drop(ProjectStore::initialize(&other, "source-1")?);
    let before = inventory(&other)?;
    for expected in [Some("source-1"), None] {
        let error = f
            .sources
            .inspect_object_source(&other, expected, None)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("IMPORT_SOURCE_REGISTERED_PATH_MISMATCH"));
    }
    assert_eq!(inventory(&other)?, before);

    f.host.lock().unwrap().put(
        "project",
        "alias",
        &json!({
            "id":"alias", "path":f.source
        }),
    )?;
    for expected in [Some("source-1"), None] {
        let error = f
            .sources
            .inspect_object_source(&f.source, expected, None)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("IMPORT_SOURCE_REGISTRATION_ID_MISMATCH"));
    }
    f.host.lock().unwrap().remove("project", "alias")?;
    f.host.lock().unwrap().put(
        "project",
        "source-1",
        &json!({
            "id":"mismatched-key", "path":f.source
        }),
    )?;
    for expected in [Some("source-1"), None] {
        let error = f
            .sources
            .inspect_object_source(&f.source, expected, None)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("IMPORT_SOURCE_REGISTRATION_ID_MISMATCH"));
    }
    assert!(f.sources.runtimes()?.is_empty());
    Ok(())
}

#[test]
fn managed_source_keeps_runtime_authority_but_rejects_a_replacement_path() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.register_source()?;
    let request = f.inspected("hero-v1")?;
    drop(f.sources.open_registered("source-1")?);
    let registration = |path: &std::path::Path| -> Result<()> {
        f.host
            .lock()
            .unwrap()
            .put("project", "source-1", &json!({"id":"source-1","path":path}))
    };
    registration(&f.temp.path().join("missing"))?;
    assert_eq!(f.inspected("hero-v1")?.source_digest, request.source_digest);
    registration(f.temp.path())?;
    let error = f.inspected("hero-v1").unwrap_err();
    assert!(error
        .to_string()
        .contains("IMPORT_SOURCE_REGISTERED_PATH_MISMATCH"));
    Ok(())
}

#[test]
fn closed_and_managed_snapshots_reject_corrupt_entity_identity() -> Result<()> {
    for opened in [false, true] {
        for case in ["project", "object-key", "object-owner"] {
            let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
            f.register_source()?;
            let source = ProjectStore::open(&f.source, "source-1")?;
            if case == "project" {
                source
                    .store()
                    .put("project", "source-1", &json!({"id":"wrong-id"}))?;
            } else {
                let mut object = source
                    .store()
                    .get::<ObjectRecord>("object", "hero")?
                    .unwrap();
                if case == "object-key" {
                    object.id = "wrong-id".into();
                } else {
                    object.project_id = "wrong-project".into();
                }
                source.store().put("object", "hero", &object)?;
            }
            drop(source);
            if opened {
                drop(f.sources.open_registered("source-1")?);
            }
            let before = (!opened).then(|| inventory(&f.source)).transpose()?;
            let error = f.inspected("hero-v1").unwrap_err();
            let code = if case == "project" {
                "IMPORT_SOURCE_PROJECT_ID_MISMATCH"
            } else {
                "IMPORT_SOURCE_OBJECT_ID_MISMATCH"
            };
            assert!(error.to_string().contains(code), "{error:#}");
            if let Some(before) = before {
                assert_eq!(inventory(&f.source)?, before);
                assert!(f.sources.runtimes()?.is_empty());
            }
        }
    }
    Ok(())
}
