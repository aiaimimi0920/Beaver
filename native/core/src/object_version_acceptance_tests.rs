use super::*;
use crate::{
    object_catalog::{ObjectFile, ObjectRecord},
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
    object_registration::{register, update},
    object_version_capture::capture,
    object_version_manifest::{self, VersionStatus},
};
use anyhow::Result;
use serde_json::json;
use std::fs;

fn acceptance_request(
    object: &ObjectRecord,
    request_id: &str,
    version_id: &str,
) -> AcceptanceRequest {
    AcceptanceRequest {
        project_id: object.project_id.clone(),
        request_id: request_id.into(),
        object_id: object.id.clone(),
        version_id: version_id.into(),
        expected_revision: object.revision,
    }
}

fn capture_with_file(fixture: &Fixture, request_id: &str) -> Result<CommandResult> {
    let mut request = fixture.request("register");
    request.files.push(ObjectFile {
        path: "hero.tscn".into(),
        role: "scene".into(),
    });
    fs::write(fixture.temp.path().join("hero.tscn"), "hero scene")?;
    let object = register(&fixture.runtime, &request)?.object;
    Ok(capture(
        &fixture.runtime,
        &capture_request(&object, request_id),
    )?)
}

#[test]
fn captured_version_can_be_accepted_without_copying_content() -> Result<()> {
    let fixture = Fixture::new()?;
    let captured = capture_with_file(&fixture, "capture")?;
    let version_id = captured.version_id.clone().unwrap();
    let captured_manifest = captured.object.versions[0].manifest.clone();
    let captured_file =
        object_version_manifest::read_version(&captured.object, &captured.object.versions[0])?
            .files[0]
            .clone();
    let blob_before = fs::read(fixture.runtime.files().blob(&captured_file.sha256)?)?;
    let accepted = accept(
        &fixture.runtime,
        &acceptance_request(&captured.object, "accept", &version_id),
    )?;

    assert_eq!(accepted.version_id.as_deref(), Some(version_id.as_str()));
    assert_eq!(accepted.object.revision, captured.object.revision + 1);
    assert_eq!(
        accepted.object.versions.len(),
        captured.object.versions.len()
    );
    assert_eq!(accepted.object.name, captured.object.name);
    let mut expected_manifest = captured_manifest;
    expected_manifest["status"] = json!("accepted");
    assert_eq!(accepted.object.versions[0].manifest, expected_manifest);
    assert_eq!(
        object_version_manifest::read(&accepted.object, &accepted.object.versions[0])?.files[0]
            .sha256,
        captured_file.sha256
    );
    assert_eq!(
        fs::read(fixture.runtime.files().blob(&captured_file.sha256)?)?,
        blob_before
    );
    let selection = crate::object_import_snapshot::select(
        std::slice::from_ref(&accepted.object),
        "project-1",
        &accepted.object.id,
        &version_id,
    )?;
    assert_eq!(selection.versions[0].status, VersionStatus::Accepted);
    assert_eq!(fixture.count("task")?, 0);
    Ok(())
}

#[test]
fn acceptance_targets_the_requested_version_and_preserves_other_versions() -> Result<()> {
    let fixture = Fixture::new()?;
    let first = capture_with_file(&fixture, "capture-first")?;
    let mut edit = update_request(&first.object, "edit");
    edit.name = "Second working name".into();
    let edited = update(&fixture.runtime, &edit)?.object;
    fs::write(fixture.temp.path().join("hero.tscn"), "second scene")?;
    let second = capture(
        &fixture.runtime,
        &capture_request(&edited, "capture-second"),
    )?;
    let first_id = first.version_id.clone().unwrap();
    let second_id = second.version_id.clone().unwrap();
    let accepted = accept(
        &fixture.runtime,
        &acceptance_request(&second.object, "accept-first", &first_id),
    )?;

    assert_eq!(accepted.object.versions.len(), 2);
    assert_eq!(accepted.object.versions[0].version_id, first_id);
    assert_eq!(accepted.object.versions[1].version_id, second_id);
    assert_eq!(accepted.object.versions[0].manifest["status"], "accepted");
    assert_eq!(accepted.object.versions[1], second.object.versions[1]);
    assert_eq!(accepted.object.name, second.object.name);
    assert_eq!(fixture.count("task")?, 0);
    Ok(())
}

#[test]
fn acceptance_replays_and_rejects_conflicts_or_duplicate_acceptance() -> Result<()> {
    let fixture = Fixture::new()?;
    let captured = capture_with_file(&fixture, "capture")?;
    let version_id = captured.version_id.clone().unwrap();
    let request = acceptance_request(&captured.object, "accept", &version_id);
    let accepted = accept(&fixture.runtime, &request)?;
    assert_eq!(accept(&fixture.runtime, &request)?, accepted);

    let mut conflict = request.clone();
    conflict.expected_revision += 1;
    assert!(accept(&fixture.runtime, &conflict)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REQUEST_CONFLICT"));

    let mut duplicate = request;
    duplicate.request_id = "accept-again".into();
    duplicate.expected_revision = accepted.object.revision;
    assert!(accept(&fixture.runtime, &duplicate)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_VERSION_ALREADY_ACCEPTED"));
    assert_eq!(fixture.count("task")?, 0);
    Ok(())
}

#[test]
fn acceptance_checks_revision_identity_and_version_identity() -> Result<()> {
    let fixture = Fixture::new()?;
    let captured = capture_with_file(&fixture, "capture")?;
    let version_id = captured.version_id.clone().unwrap();

    let mut stale = acceptance_request(&captured.object, "stale", &version_id);
    stale.expected_revision -= 1;
    assert!(accept(&fixture.runtime, &stale)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REVISION_CONFLICT"));

    let mut foreign = acceptance_request(&captured.object, "foreign", &version_id);
    foreign.project_id = "project-2".into();
    assert!(accept(&fixture.runtime, &foreign)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_PROJECT_MISMATCH"));

    let missing = acceptance_request(&captured.object, "missing", "version-missing");
    assert!(accept(&fixture.runtime, &missing)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_VERSION_NOT_FOUND"));
    Ok(())
}
