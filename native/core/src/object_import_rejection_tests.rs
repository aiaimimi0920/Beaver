use super::*;

#[test]
fn inspection_and_preparation_reject_invalid_frozen_manifests() -> Result<()> {
    let cases = [
        ("schemaVersion", json!(2), "IMPORT_VERSION_UNSUPPORTED"),
        ("status", json!("candidate"), "IMPORT_VERSION_NOT_ACCEPTED"),
        (
            "objectId",
            json!("wrong-object"),
            "IMPORT_VERSION_IDENTITY_MISMATCH",
        ),
        (
            "projectId",
            json!("wrong-project"),
            "IMPORT_VERSION_IDENTITY_MISMATCH",
        ),
        ("name", json!(" "), "IMPORT_VERSION_INVALID_NAME"),
        (
            "components",
            json!([{"id":"x","kind":"scene","name":"One"},{"id":"x","kind":"scene","name":"Two"}]),
            "IMPORT_VERSION_INVALID_COMPONENT",
        ),
        (
            "references",
            json!([{"projectId":"source-1","objectId":"dep","versionId":null}]),
            "IMPORT_REFERENCE_NOT_PINNED",
        ),
        (
            "references",
            json!([{"projectId":"another-project","objectId":"dep","versionId":"dep-v1"}]),
            "CROSS_PROJECT_OBJECT_REFERENCE",
        ),
        (
            "references",
            json!([reference("missing", "missing-v1")]),
            "IMPORT_REFERENCE_MISSING",
        ),
        (
            "references",
            json!([reference("dep", "unknown-version")]),
            "IMPORT_VERSION_MISSING_OR_DUPLICATE",
        ),
        (
            "files",
            json!([file("same.txt", "a"), file("SAME.txt", "b")]),
            "IMPORT_VERSION_INVALID_FILE",
        ),
    ];
    for (field, value, code) in cases {
        let mut manifest = accepted("hero", "hero-v1");
        manifest[field] = value;
        let f = Fixture::new(vec![
            record("hero", vec![manifest]),
            record("dep", vec![accepted("dep", "dep-v1")]),
        ])?;
        let before = inventory(&f.source)?;
        let snapshot = object_external_snapshot::read(&f.source, "source-1", Some("hero"))?;
        assert_eq!(inventory(&f.source)?, before);
        let option = &snapshot.import_versions[0];
        assert!(option.source_digest.is_none());
        assert!(
            option.blocker.as_deref().unwrap().contains(code),
            "{field}: {:?}",
            option.blocker
        );
        f.assert_rejected(&f.request("hero-v1"), code)?;
    }
    Ok(())
}

#[test]
fn rejects_legacy_versions_missing_versions_and_empty_baselines() -> Result<()> {
    let mut old = record("hero", vec![accepted("hero", "hero-v1")]);
    old.versions[0].manifest = json!({});
    let f = Fixture::new(vec![old])?;
    f.assert_rejected(&f.request("hero-v1"), "IMPORT_VERSION_UNSUPPORTED")?;
    f.assert_rejected(
        &f.request("nonexistent"),
        "IMPORT_VERSION_MISSING_OR_DUPLICATE",
    )?;
    let mut request = f.request("hero-v1");
    request.baseline = Baseline::LatestAccepted;
    f.assert_rejected(&request, "IMPORT_NO_ACCEPTED_VERSION")?;
    let empty = Fixture::new(vec![record("hero", vec![])])?;
    request.source_path = empty.source.clone();
    empty.assert_rejected(&request, "IMPORT_NO_ACCEPTED_VERSION")?;
    Ok(())
}

#[test]
fn rejects_paths_that_could_escape_or_overwrite_project_control_files() -> Result<()> {
    for path in [
        "../outside",
        "a/../outside",
        "/absolute",
        "C:/outside",
        "a\\b",
        "a//b",
        ".BEAVER/project.sqlite",
        ".git/config",
        "a/./b",
        "bad\0name",
    ] {
        let mut manifest = accepted("hero", "hero-v1");
        manifest["files"] = json!([file(path, "source")]);
        let f = Fixture::new(vec![record("hero", vec![manifest])])?;
        f.assert_rejected(&f.request("hero-v1"), "IMPORT_PATH_TRAVERSAL")?;
    }
    Ok(())
}

#[test]
fn frozen_closure_rejects_conflicting_component_and_file_owners() -> Result<()> {
    for kind in ["COMPONENT", "FILE"] {
        let mut hero = accepted("hero", "hero-v1");
        hero["references"] = json!([reference("dep", "dep-v1")]);
        let mut dep = accepted("dep", "dep-v1");
        if kind == "COMPONENT" {
            dep["components"] = hero["components"].clone();
        } else {
            hero["files"] = json!([file("shared.txt", "same")]);
            dep["files"] = hero["files"].clone();
        }
        let f = Fixture::new(vec![record("hero", vec![hero]), record("dep", vec![dep])])?;
        f.assert_rejected(
            &f.request("hero-v1"),
            &format!("IMPORT_{kind}_OWNER_CONFLICT"),
        )?;
    }
    Ok(())
}

#[test]
fn missing_tampered_and_wrong_sized_blobs_never_fall_back_to_working_files() -> Result<()> {
    for (content, code) in [
        (None, "IMPORT_CONTENT_MISSING"),
        (Some("tampered"), "IMPORT_CONTENT_HASH_MISMATCH"),
        (Some("short"), "IMPORT_CONTENT_SIZE_MISMATCH"),
    ] {
        let mut manifest = accepted("hero", "hero-v1");
        manifest["files"] = json!([file("hero.tscn", "accepted")]);
        let f = Fixture::new(vec![record("hero", vec![manifest])])?;
        fs::write(f.source.join("hero.tscn"), "accepted")?;
        let blob = f.blob("accepted")?;
        let request = f.inspected("hero-v1")?;
        if let Some(content) = content {
            fs::write(blob, content)?;
        } else {
            fs::remove_file(blob)?;
        }
        f.assert_rejected(&request, code)?;
    }
    Ok(())
}

#[test]
fn selection_digest_must_match_inspected_manifests() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    f.assert_rejected(&f.request("hero-v1"), "IMPORT_SOURCE_CHANGED")?;
    let mut request = f.inspected("hero-v1")?;
    request.source_digest = "not-a-sha256".into();
    f.assert_rejected(&request, "IMPORT_INVALID_SOURCE_DIGEST")?;
    Ok(())
}

#[test]
fn linked_blob_directories_are_rejected_without_following_them() -> Result<()> {
    let mut manifest = accepted("hero", "hero-v1");
    manifest["files"] = json!([file("hero.tscn", "accepted")]);
    let f = Fixture::new(vec![record("hero", vec![manifest])])?;
    let blob = f.blob("accepted")?;
    let directory = blob.parent().unwrap();
    let outside = f.temp.path().join("outside-blobs");
    let request = f.inspected("hero-v1")?;
    fs::rename(directory, &outside)?;
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(directory.to_string_lossy().replace('/', "\\"))
            .arg(outside.to_string_lossy().replace('/', "\\"))
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "test junction creation failed: {:?}",
            output
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, directory)?;
    let before = inventory(&outside)?;
    f.assert_rejected(&request, "IMPORT_UNSAFE_CONTENT_PATH")?;
    assert_eq!(inventory(&outside)?, before);
    Ok(())
}
