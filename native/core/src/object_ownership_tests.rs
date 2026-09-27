use super::*;
use crate::{
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
    object_version_capture,
};

#[test]
fn ownership_edits_are_atomic_and_case_aliases_cannot_claim_the_same_file() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("owner");
    request.files.push(ObjectFile {
        path: "art/Hero.glb".into(),
        role: "source".into(),
    });
    request.components.push(ObjectComponent {
        id: "mesh".into(),
        kind: "mesh".into(),
        name: "Mesh".into(),
    });
    let owner = register(&f.runtime, &request)?;
    let other = register(&f.runtime, &f.request("other"))?;
    let mut edit = update_request(&other.object, "claim-file");
    edit.files.push(ObjectFile {
        path: "ART/hero.GLB".into(),
        role: "preview".into(),
    });
    assert!(update(&f.runtime, &edit)
        .unwrap_err()
        .to_string()
        .contains("DUPLICATE_OBJECT_FILE"));
    edit.files.clear();
    edit.components = owner.object.components.clone();
    assert!(update(&f.runtime, &edit)
        .unwrap_err()
        .to_string()
        .contains("DUPLICATE_OBJECT_COMPONENT"));
    assert_eq!(f.object(&other.object.id)?, other.object);
    let mut release = update_request(&owner.object, "release");
    release.components.clear();
    update(&f.runtime, &release)?;
    assert_eq!(
        update(&f.runtime, &edit)?.object.components,
        owner.object.components
    );
    Ok(())
}

#[test]
fn registration_rejects_unsafe_or_ambiguous_file_names() -> Result<()> {
    let f = Fixture::new()?;
    for path in [
        "../hero.gd",
        "a/./hero.gd",
        "/a.gd",
        "a\\b.gd",
        "C:/a.gd",
        "a//b.gd",
        ".beaver/project.sqlite",
        "a/.GODOT/cache",
        ".git/config",
        "CON.gd",
        "x/Lpt1.txt",
        "a /b.gd",
        "a./b.gd",
        "file.gd:stream",
        "a?.gd",
    ] {
        let mut request = f.request("bad-path");
        request.files.push(ObjectFile {
            path: path.into(),
            role: "source".into(),
        });
        assert!(register(&f.runtime, &request).is_err(), "accepted {path}");
    }
    assert_eq!(f.count("object")?, 0);
    assert_eq!(f.count("object_command_receipt")?, 0);
    Ok(())
}

#[test]
fn references_require_an_existing_accepted_same_project_version() -> Result<()> {
    let f = Fixture::new()?;
    let pinned = f.accepted("source", "accepted-v1")?;
    let source = f.object("source")?;
    let captured =
        object_version_capture::capture(&f.runtime, &capture_request(&source, "capture-source"))?;
    for reference in [
        ObjectReference {
            project_id: "other".into(),
            ..pinned.clone()
        },
        ObjectReference {
            object_id: "absent".into(),
            ..pinned.clone()
        },
        ObjectReference {
            version_id: None,
            ..pinned.clone()
        },
        ObjectReference {
            version_id: Some("absent".into()),
            ..pinned.clone()
        },
        ObjectReference {
            version_id: captured.version_id,
            ..pinned.clone()
        },
    ] {
        let mut request = f.request("bad-reference");
        request.references.push(reference);
        assert!(register(&f.runtime, &request).is_err());
    }
    let mut request = f.request("pinned");
    request.references.push(pinned.clone());
    let object = register(&f.runtime, &request)?.object;
    let mut self_reference = update_request(&object, "self-reference");
    self_reference.references[0].object_id = object.id.clone();
    assert!(update(&f.runtime, &self_reference)
        .unwrap_err()
        .to_string()
        .contains("SELF_OBJECT_REFERENCE"));
    assert_eq!(f.object(&object.id)?.references, vec![pinned]);
    Ok(())
}

#[test]
fn pinned_reference_rejects_a_missing_or_conflicting_version_entity() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("reference");
    request.references.push(f.accepted("source", "v1")?);
    f.runtime
        .store()
        .lock()
        .unwrap()
        .remove("object_version", "v1")?;
    assert!(register(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REFERENCE_VERSION_MISSING"));
    let version = f.object("source")?.versions[0].clone();
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_version", "v1", &("other", version))?;
    assert!(register(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REFERENCE_VERSION_MISMATCH"));
    assert_eq!(f.count("object_command_receipt")?, 0);
    Ok(())
}
