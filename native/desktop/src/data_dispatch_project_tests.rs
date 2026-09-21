use super::*;

#[test]
fn project_document_reads_use_open_runtime_root() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let (root, host, router, _runtime) = open_project(&temp, "project-a")?;
    fs::write(root.join("story.md"), "project copy")?;
    let decoy = temp.path().join("decoy");
    fs::create_dir(&decoy)?;
    fs::write(decoy.join("story.md"), "host copy")?;
    let mut registered = host
        .lock()
        .unwrap()
        .get::<Value>("project", "project-a")?
        .unwrap();
    registered["path"] = json!(decoy.to_string_lossy().to_string());
    host.lock()
        .unwrap()
        .put("project", "project-a", &registered)?;

    assert_eq!(
        project_document_operation(
            &router,
            host.clone(),
            "asset.text",
            Some(json!({"id":"project-a","path":"story.md"})),
        )?,
        json!("project copy")
    );
    let document = project_document_operation(
        &router,
        host.clone(),
        "document.read",
        Some(json!({"id":"project-a","path":"story.md"})),
    )?;
    assert_eq!(document["text"], "project copy");
    let assets =
        project_document_operation(&router, host, "assets", Some(json!({"id":"project-a"})))?;
    assert!(assets
        .as_array()
        .unwrap()
        .iter()
        .any(|asset| asset["path"] == "story.md"));
    Ok(())
}

#[test]
fn project_document_reads_keep_legacy_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project_root(temp.path(), "legacy-project")?;
    let host = host_store(temp.path())?;
    register_project(&host, "legacy-project", &root)?;
    fs::write(root.join("story.md"), "legacy copy")?;
    let router = ProjectStorageRouter::new(host.clone());

    assert_eq!(
        project_document_operation(
            &router,
            host.clone(),
            "asset.text",
            Some(json!({"id":"legacy-project","path":"story.md"})),
        )?,
        json!("legacy copy")
    );
    let document = project_document_operation(
        &router,
        host.clone(),
        "document.read",
        Some(json!({"id":"legacy-project","path":"story.md"})),
    )?;
    assert_eq!(document["text"], "legacy copy");
    assert!(project_document_operation(
        &router,
        host,
        "assets",
        Some(json!({"id":"legacy-project"})),
    )?
    .as_array()
    .unwrap()
    .iter()
    .any(|asset| asset["path"] == "story.md"));
    Ok(())
}

#[test]
fn project_and_asset_reveal_use_open_runtime_root() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_id = "11111111-1111-1111-1111-111111111111";
    let (root, host, router, _runtime) = open_project(&temp, project_id)?;
    let asset = root.join("character.glb");
    fs::write(&asset, b"model")?;
    let decoy = temp.path().join("decoy");
    fs::create_dir(&decoy)?;
    fs::write(decoy.join("character.glb"), b"wrong")?;
    let mut registered = host
        .lock()
        .unwrap()
        .get::<Value>("project", project_id)?
        .unwrap();
    registered["path"] = json!(decoy.to_string_lossy().to_string());
    host.lock()
        .unwrap()
        .put("project", project_id, &registered)?;

    let project = resolve_project_asset(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "project.reveal",
        Some(json!({"id":project_id})),
    )?;
    assert!(!project.select);
    assert_eq!(project.path, fs::canonicalize(&root)?);
    let asset = resolve_project_asset(
        &router,
        host,
        &host_root(temp.path()),
        "asset.reveal",
        Some(json!({"id":project_id,"path":"character.glb"})),
    )?;
    assert!(asset.select);
    assert_eq!(asset.path, fs::canonicalize(&root)?.join("character.glb"));
    Ok(())
}

#[test]
fn project_and_asset_reveal_keep_legacy_host_fallback() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_id = "22222222-2222-2222-2222-222222222222";
    let root = project_root(temp.path(), project_id)?;
    let host = host_store(temp.path())?;
    register_project(&host, project_id, &root)?;
    fs::write(root.join("character.glb"), b"model")?;
    let router = ProjectStorageRouter::new(host.clone());

    let project = resolve_project_asset(
        &router,
        host.clone(),
        &host_root(temp.path()),
        "project.reveal",
        Some(json!({"id":project_id})),
    )?;
    assert_eq!(project.path, fs::canonicalize(&root)?);
    let asset = resolve_project_asset(
        &router,
        host,
        &host_root(temp.path()),
        "asset.reveal",
        Some(json!({"id":project_id,"path":"character.glb"})),
    )?;
    assert_eq!(asset.path, fs::canonicalize(&root)?.join("character.glb"));
    Ok(())
}

#[test]
fn project_operations_reject_host_fallback_when_local_runtime_is_closed() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_id = "33333333-3333-3333-3333-333333333333";
    let (root, host, router, runtime) = open_project(&temp, project_id)?;
    fs::write(root.join("story.md"), "project copy")?;
    fs::write(root.join("character.glb"), b"model")?;
    drop(runtime);
    router.close(project_id)?;

    let document_error = project_document_operation(
        &router,
        host.clone(),
        "document.read",
        Some(json!({"id":project_id,"path":"story.md"})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        document_error.contains("禁止回退到宿主存储"),
        "{document_error}"
    );

    let assets_error = project_document_operation(
        &router,
        host.clone(),
        "assets",
        Some(json!({"id":project_id})),
    )
    .unwrap_err()
    .to_string();
    assert!(
        assets_error.contains("禁止回退到宿主存储"),
        "{assets_error}"
    );

    for method in ["project.reveal", "asset.reveal"] {
        let error = match resolve_project_asset(
            &router,
            host.clone(),
            &host_root(temp.path()),
            method,
            Some(json!({"id":project_id,"path":"character.glb"})),
        ) {
            Ok(_) => anyhow::bail!("{method} unexpectedly used a closed runtime"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("禁止回退到宿主存储"), "{method}: {error}");
    }
    Ok(())
}
