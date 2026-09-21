use super::*;
use beaver_core::store::Store;
use serde_json::json;
use std::collections::BTreeMap;

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    fn visit(root: &Path, current: &Path, entries: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(root)?.to_path_buf();
            if entry.file_type()?.is_dir() {
                entries.insert(relative, Vec::new());
                visit(root, &path, entries)?;
            } else {
                entries.insert(relative, fs::read(&path)?);
            }
        }
        Ok(())
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries)?;
    Ok(entries)
}

#[test]
fn migration_requests_require_explicit_paths_and_reject_live_host_overlap() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = temp.path().join("live");
    fs::create_dir(&host)?;
    fs::write(host.join("sentinel"), "unchanged")?;
    let store = std::sync::Arc::new(Mutex::new(Store::open(&temp.path().join("router"))?));
    let router = ProjectStorageRouter::new(store);
    let before = snapshot(&host)?;
    for input in [
        json!({}),
        json!({"backup":"relative"}),
        json!({"backup":host}),
        json!({"backup":temp.path()}),
        json!({"backup":host,"projectId":"unexpected"}),
    ] {
        assert!(call(&host, &router, "migration.inspect", &input).is_err());
    }
    let host = fs::canonicalize(host)?;
    assert!(path(
        &json!({"destination":host.join("new")}),
        "destination",
        true,
        &host
    )
    .is_err());
    assert_eq!(snapshot(&host)?, before);
    assert!(
        crate::business_catalog::validate("migration.activate", &json!({"backup":"x"})).is_err()
    );
    assert!(crate::business_catalog::validate(
        "migration.activate",
        &json!({"backup":"x","prepared":"y","force":true})
    )
    .is_err());
    assert!(crate::business_catalog::validate(
        "migration.activateAssembly",
        &json!({"preparation":"x","destination":"y"})
    )
    .is_ok());
    assert!(crate::business_catalog::validate(
        "migration.activateAssembly",
        &json!({"preparation":"x","destination":"y","backup":"z"})
    )
    .is_err());
    let tools = crate::business_catalog::tools();
    for (method, readonly) in [
        ("migration.inspect", true),
        ("migration.prepareProjects", false),
        ("migration.activate", false),
        ("migration.activateAssembly", false),
        ("migration.registerAssembly", false),
    ] {
        let tool = tools.iter().find(|tool| tool["name"] == method).unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], readonly);
    }
    Ok(())
}

#[cfg(windows)]
fn fixture(root: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let host = root.join("live");
    fs::create_dir(&host)?;
    let source = root.join("source");
    let project = root.join("game");
    fs::create_dir(&project)?;
    fs::write(project.join("project.godot"), "config_version=5")?;
    let id = "11111111-1111-4111-8111-111111111111";
    let store = Store::open(&source)?;
    store.put(
        "project",
        id,
        &json!({"id":id,"path":project,"unknown":"preserved"}),
    )?;
    drop(store);
    let backup = root.join("archive");
    migration_bundle::create(&source, &backup)?;
    Ok((host, source, backup))
}

#[cfg(windows)]
#[test]
fn migration_entry_prepares_partitions_without_enabling_or_changing_sources() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (host, source, backup) = fixture(temp.path())?;
    let original = snapshot(&source)?;
    let archived = snapshot(&backup)?;
    let host_before = snapshot(&host)?;
    let inventory = execute(&host, "migration.inspect", &json!({"backup":backup}))?;
    assert_eq!(inventory["readyToActivate"], false);
    let destination = temp.path().join("prepared");
    let request = json!({"backup":backup,"destination":destination});
    let receipt = execute(&host, "migration.prepareProjects", &request)?;
    assert_eq!(receipt["readyToActivate"], false);
    assert!(destination.join("PROJECT-MIGRATION.json").is_file());
    let id = "11111111-1111-4111-8111-111111111111";
    assert!(destination
        .join("projects")
        .join(id)
        .join(".beaver/project.sqlite")
        .is_file());
    assert!(migration_bundle::ensure_activated(&destination.join("data")).is_err());
    let prepared_before = snapshot(&destination)?;
    assert!(execute(&host, "migration.prepareProjects", &request).is_err());
    assert_eq!(snapshot(&destination)?, prepared_before);
    // A malformed explicit tool configuration fails activation without running tools.
    let tools = temp.path().join("invalid-tools.json");
    fs::write(&tools, "not json")?;
    let error = execute(
        &host,
        "migration.activate",
        &json!({"backup":backup,"prepared":destination,"toolPaths":tools}),
    )
    .unwrap_err();
    assert!(
        error.downcast_ref::<serde_json::Error>().is_some(),
        "{error:#}"
    );
    assert!(destination.join(".beaver-migration-pending").is_file());
    assert!(!destination.join("ACTIVATION.json").exists());
    assert_eq!(snapshot(&source)?, original);
    assert_eq!(snapshot(&backup)?, archived);
    assert_eq!(snapshot(&host)?, host_before);
    Ok(())
}

#[cfg(windows)]
#[test]
fn migration_entry_keeps_failed_partition_copy_and_rejects_unpartitioned_activation() -> Result<()>
{
    let temp = tempfile::tempdir()?;
    let (host, source, backup) = fixture(temp.path())?;
    let restore = temp.path().join("plain-restore");
    migration_bundle::restore(&backup, &restore)?;
    let before = snapshot(&restore)?;
    let error = execute(
        &host,
        "migration.activate",
        &json!({"backup":backup,"prepared":restore}),
    )
    .unwrap_err();
    assert!(error.to_string().contains("partition preparation"));
    assert_eq!(snapshot(&restore)?, before);
    // Existing portable storage cannot be silently overwritten by a legacy partition.
    fs::create_dir(temp.path().join("game/.beaver"))?;
    fs::write(temp.path().join("game/.beaver/sentinel"), "preserved")?;
    let incompatible = temp.path().join("incompatible-archive");
    migration_bundle::create(&source, &incompatible)?;
    let archived = snapshot(&incompatible)?;
    let destination = temp.path().join("failed-partition");
    let error = execute(
        &host,
        "migration.prepareProjects",
        &json!({"backup":incompatible,"destination":destination}),
    )
    .unwrap_err();
    assert!(error.to_string().contains("pending copy preserved"));
    assert!(destination.join("RESTORE.json").is_file());
    assert!(destination.join(".beaver-migration-pending").is_file());
    assert!(!destination.join("PROJECT-MIGRATION.json").exists());
    assert_eq!(snapshot(&incompatible)?, archived);
    Ok(())
}
