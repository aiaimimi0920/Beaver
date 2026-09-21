use super::*;

fn storage_bytes(root: &Path) -> Result<std::collections::BTreeMap<String, Vec<u8>>> {
    let mut result = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root.join(layout::CONTROL_DIR))? {
        let entry = entry?;
        // Windows denies reading the lock bytes while an owner holds its range lock.
        if entry.file_type()?.is_file() && entry.file_name() != layout::LOCK {
            result.insert(
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path())?,
            );
        }
    }
    Ok(result)
}

#[test]
fn importing_portable_wal_snapshot_does_not_modify_source_storage() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Store::open(&data)?;
    let original = temp.path().join("original");
    let copy = temp.path().join("copy");
    let id = "portable";
    for root in [&original, &copy] {
        fs::create_dir(root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
    }
    let owner = ProjectStore::initialize(&original, id)?;
    owner.store().put(
        "project",
        id,
        &json!({"id":id,"name":"WAL metadata","path":original}),
    )?;
    owner.store().put(
        "task",
        "history",
        &json!({"id":"history","projectId":id,"status":"completed"}),
    )?;
    let source = original.join(layout::CONTROL_DIR);
    let target = copy.join(layout::CONTROL_DIR);
    fs::create_dir(&target)?;
    for name in layout::DIRECTORIES {
        fs::create_dir(target.join(name))?;
    }
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    assert!(fs::metadata(target.join("project.sqlite-wal"))?.len() > 32);
    let before = storage_bytes(&copy)?;
    host.connection.execute_batch("PRAGMA query_only=ON")?;
    assert!(import_project(&host, &data, &copy, &json!({})).is_err());
    assert!(host.list::<Value>("project")?.is_empty());
    assert!(
        storage_bytes(&copy)? == before,
        "failed registration modified source storage"
    );
    host.connection.execute_batch("PRAGMA query_only=OFF")?;
    let imported = import_project(&host, &data, &copy, &json!({}))?;
    assert_eq!(imported["id"], id);
    assert_eq!(imported["name"], "WAL metadata");
    assert!(
        storage_bytes(&copy)? == before,
        "registration modified source storage"
    );
    assert!(owner.store().get::<Value>("task", "history")?.is_some());
    Ok(())
}

#[test]
fn registration_snapshot_rejects_busy_and_mismatched_storage_without_writes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Store::open(&data)?;
    let root = temp.path().join("project");
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    let owner = ProjectStore::initialize(&root, "project")?;
    owner
        .store()
        .put("project", "project", &json!({"id":"wrong-id","path":root}))?;
    let before = storage_bytes(&root)?;
    let error = import_project(&host, &data, &root, &json!({})).unwrap_err();
    assert!(error.to_string().contains("另一个宿主"), "{error}");
    assert!(storage_bytes(&root)? == before);
    drop(owner);
    let before = storage_bytes(&root)?;
    let error = import_project(&host, &data, &root, &json!({})).unwrap_err();
    assert!(error.to_string().contains("实体 ID"), "{error}");
    assert!(storage_bytes(&root)? == before);
    assert!(host.list::<Value>("project")?.is_empty());
    let owner = ProjectStore::open(&root, "project")?;
    owner
        .store()
        .connection
        .execute("UPDATE project_identity SET project_id='wrong-id'", [])?;
    drop(owner);
    let before = storage_bytes(&root)?;
    let error = import_project(&host, &data, &root, &json!({})).unwrap_err();
    assert!(error.to_string().contains("数据库身份"), "{error}");
    assert!(storage_bytes(&root)? == before);
    assert!(host.list::<Value>("project")?.is_empty());
    Ok(())
}

#[test]
fn duplicate_identity_never_replaces_online_or_offline_registration() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Store::open(&data)?;
    let original = temp.path().join("original");
    let copy = temp.path().join("copy");
    let id = "shared-project-id";
    for root in [&original, &copy] {
        fs::create_dir(root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let local = ProjectStore::initialize(root, id)?;
        local.store().put(
            "project",
            id,
            &json!({"id":id,"name":"Portable","path":root}),
        )?;
        local.store().put(
            "task",
            "history",
            &json!({"id":"history","projectId":id,"status":"completed"}),
        )?;
    }
    let first = import_project(&host, &data, &original, &json!({}))?;
    assert_eq!(import_project(&host, &data, &original, &json!({}))?, first);
    let manifest_path = copy.join(layout::CONTROL_DIR).join(layout::MANIFEST);
    let manifest = fs::read(&manifest_path)?;
    for offline in [false, true] {
        if offline {
            fs::rename(&original, temp.path().join("moved-original"))?;
        }
        let error = import_project(&host, &data, &copy, &json!({})).unwrap_err();
        assert!(error.to_string().contains("同一项目 ID 已登记在其他位置"));
        assert_eq!(host.get::<Value>("project", id)?.unwrap(), first);
        assert_eq!(host.list::<Value>("project")?.len(), 1);
        assert_eq!(fs::read(&manifest_path)?, manifest);
        let local = ProjectStore::open(&copy, id)?;
        assert_eq!(
            local.store().get::<Value>("task", "history")?.unwrap()["status"],
            "completed"
        );
    }
    Ok(())
}
