use super::*;
use crate::{
    object_catalog::ObjectFile,
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
    object_registration, object_version_capture,
    project_storage::ProjectStore,
};
use base64::{engine::general_purpose::STANDARD, Engine};

fn capture(f: &Fixture, path: &str, bytes: &[u8]) -> Result<Request> {
    fs::write(f.temp.path().join(path), bytes)?;
    let mut registration = f.request("register-preview");
    registration.files.push(ObjectFile {
        path: path.into(),
        role: "source".into(),
    });
    let object = object_registration::register(&f.runtime, &registration)?.object;
    let captured =
        object_version_capture::capture(&f.runtime, &capture_request(&object, "capture-preview"))?;
    let manifest =
        object_version_manifest::read_version(&captured.object, &captured.object.versions[0])?;
    Ok(Request {
        project_id: object.project_id,
        object_id: object.id,
        version_id: manifest.version_id,
        path: path.into(),
        sha256: manifest.files[0].sha256.clone(),
    })
}

#[test]
fn object_version_file_reopens_exact_old_content_after_new_capture_and_live_deletion() -> Result<()>
{
    let f = Fixture::new()?;
    let request = capture(&f, "scene.tscn", b"old scene")?;
    fs::write(f.temp.path().join("scene.tscn"), "new scene")?;
    object_version_capture::capture(
        &f.runtime,
        &capture_request(&f.object(&request.object_id)?, "new-capture"),
    )?;
    fs::remove_file(f.temp.path().join("scene.tscn"))?;
    let mut edit = update_request(&f.object(&request.object_id)?, "edit-metadata");
    edit.category = "New category".into();
    edit.tags = vec!["changed".into()];
    edit.thumbnail_path = Some("new.png".into());
    let edited = object_registration::update(&f.runtime, &edit)?.object;
    assert!(object_version_manifest::read_version(&edited, &edited.versions[0]).is_err());
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        read(&reopened, &request)?.content,
        Content::Text {
            text: "old scene".into()
        }
    );
    assert!(reopened
        .store()
        .lock()
        .unwrap()
        .list::<serde_json::Value>("task")?
        .is_empty());
    Ok(())
}

#[test]
fn object_version_file_rejects_identity_hash_and_corrupt_blob() -> Result<()> {
    let f = Fixture::new()?;
    let request = capture(&f, "data.txt", b"original")?;
    for changed in [
        Request {
            project_id: "foreign".into(),
            ..request.clone()
        },
        Request {
            object_id: "foreign".into(),
            ..request.clone()
        },
        Request {
            version_id: "foreign".into(),
            ..request.clone()
        },
        Request {
            path: "../data.txt".into(),
            ..request.clone()
        },
        Request {
            sha256: "0".repeat(64),
            ..request.clone()
        },
    ] {
        assert!(read(&f.runtime, &changed).is_err());
    }
    fs::write(f.runtime.files().blob(&request.sha256)?, "tampered")?;
    assert!(read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("HASH_MISMATCH"));
    fs::write(f.runtime.files().blob(&request.sha256)?, "short")?;
    assert!(read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("SIZE_MISMATCH"));
    Ok(())
}

#[test]
fn object_version_file_media_and_size_limits_are_explicit() -> Result<()> {
    for (path, bytes, expected) in [
        ("image.png", vec![137, 80, 78, 71], "image"),
        ("sound.wav", b"RIFFwave".to_vec(), "audio"),
        ("mesh.blend", vec![0, 255], "unsupported"),
        ("large.txt", vec![b'a'; INLINE_LIMIT + 1], "tooLarge"),
        ("markup.svg", b"<svg onload='bad()'/>".to_vec(), "text"),
    ] {
        let f = Fixture::new()?;
        let request = capture(&f, path, &bytes)?;
        let response = read(&f.runtime, &request)?;
        assert_eq!(response.byte_count, bytes.len() as u64);
        let value = serde_json::to_value(response)?;
        assert_eq!(value["content"]["kind"], expected);
        if matches!(expected, "image" | "audio") {
            assert_eq!(
                STANDARD.decode(value["content"]["base64"].as_str().unwrap())?,
                bytes
            );
        }
    }
    Ok(())
}

#[test]
fn object_version_file_rejects_divergent_version_projection() -> Result<()> {
    let f = Fixture::new()?;
    let request = capture(&f, "data.txt", b"original")?;
    let mut object = f.object(&request.object_id)?;
    object.versions[0].manifest["name"] = "changed".into();
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object", &object.id, &object)?;
    assert!(read(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_VERSION_MISMATCH"));
    Ok(())
}
