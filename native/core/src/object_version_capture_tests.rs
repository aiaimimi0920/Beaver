use super::*;
use crate::{
    object_catalog::ObjectFile,
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
    object_registration::{register, update},
    object_version_manifest,
};

#[test]
fn capture_freezes_content_and_metadata_and_retries_after_live_deletion() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("register");
    request.files.push(ObjectFile {
        path: "hero.tscn".into(),
        role: "scene".into(),
    });
    request
        .references
        .push(f.accepted("dependency", "dependency-v1")?);
    fs::write(f.temp.path().join("hero.tscn"), "first scene")?;
    let object = register(&f.runtime, &request)?.object;
    let request = capture_request(&object, "capture-1");
    let first = capture(&f.runtime, &request)?;
    let manifest = object_version_manifest::read_version(&first.object, &first.object.versions[0])?;
    assert_eq!(manifest.status, VersionStatus::Captured);
    assert_eq!(manifest.references, object.references);
    assert_eq!(
        fs::read(f.runtime.files().blob(&manifest.files[0].sha256)?)?,
        b"first scene"
    );
    assert_eq!(manifest.files[0].bytes, 11);
    assert!(
        object_version_manifest::read(&first.object, &first.object.versions[0])
            .unwrap_err()
            .to_string()
            .contains("IMPORT_VERSION_NOT_ACCEPTED")
    );
    let mut edit = update_request(&first.object, "rename");
    edit.name = "New working name".into();
    edit.references.clear();
    let edited = update(&f.runtime, &edit)?.object;
    fs::write(f.temp.path().join("hero.tscn"), "second scene")?;
    let second = capture(&f.runtime, &capture_request(&edited, "capture-2"))?;
    assert_eq!(second.object.versions[0], first.object.versions[0]);
    assert_ne!(second.version_id, first.version_id);
    fs::remove_file(f.temp.path().join("hero.tscn"))?;
    assert_eq!(capture(&f.runtime, &request)?, first);
    assert_eq!(
        fs::read(f.runtime.files().blob(&manifest.files[0].sha256)?)?,
        b"first scene"
    );
    assert_eq!(f.count("task")?, 0);
    Ok(())
}

#[test]
fn empty_object_can_capture_but_inspection_cannot_offer_it_for_import() -> Result<()> {
    let f = Fixture::new()?;
    let object = register(&f.runtime, &f.request("register"))?.object;
    let result = capture(&f.runtime, &capture_request(&object, "capture"))?;
    let version_id = result.version_id.unwrap();
    let selected = crate::object_import_snapshot::select(
        &[result.object],
        "project-1",
        &object.id,
        &version_id,
    );
    assert!(selected
        .err()
        .unwrap()
        .to_string()
        .contains("IMPORT_VERSION_NOT_ACCEPTED"));
    assert_eq!(f.count("object_version")?, 1);
    Ok(())
}

#[test]
fn missing_file_or_directory_never_publishes_a_partial_version_or_receipt() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("register");
    request.files = vec![
        ObjectFile {
            path: "first.gd".into(),
            role: "script".into(),
        },
        ObjectFile {
            path: "missing.gd".into(),
            role: "script".into(),
        },
    ];
    fs::write(f.temp.path().join("first.gd"), "extends Node\n")?;
    let object = register(&f.runtime, &request)?.object;
    let request = capture_request(&object, "capture");
    assert!(capture(&f.runtime, &request).is_err());
    fs::create_dir(f.temp.path().join("missing.gd"))?;
    assert!(capture(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_CAPTURE_NOT_FILE"));
    assert_eq!(f.object(&object.id)?, object);
    assert_eq!(f.count("object_version")?, 0);
    assert_eq!(f.count("object_command_receipt")?, 1);
    Ok(())
}

#[test]
fn corrupt_content_cache_cannot_be_reused_as_a_new_version() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("register");
    request.files.push(ObjectFile {
        path: "hero.gd".into(),
        role: "script".into(),
    });
    fs::write(f.temp.path().join("hero.gd"), "extends Node\n")?;
    let object = register(&f.runtime, &request)?.object;
    let first = capture(&f.runtime, &capture_request(&object, "first"))?;
    let manifest = object_version_manifest::read_version(&first.object, &first.object.versions[0])?;
    fs::write(
        f.runtime.files().blob(&manifest.files[0].sha256)?,
        "corrupt",
    )?;
    assert!(capture(&f.runtime, &capture_request(&first.object, "second")).is_err());
    assert_eq!(f.object(&object.id)?, first.object);
    assert_eq!(f.count("object_version")?, 1);
    assert_eq!(f.count("object_command_receipt")?, 2);
    Ok(())
}
