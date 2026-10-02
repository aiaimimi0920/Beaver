use super::*;
use serde_json::{json, Value};

#[test]
fn project_derivation_rejects_unsupported_or_inconsistent_object_history_before_copy() -> Result<()>
{
    for fault in [
        "tuple",
        "digest",
        "dangling",
        "cross-project",
        "unknown",
        "historical-parent",
        "duplicate-component",
        "duplicate-file",
        "parent-cycle",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.source, "original")?;
        let store = storage.store();
        match fault {
            "tuple" => {
                let mut value: Value = store.get("object_version", &f.first.version_id)?.unwrap();
                value[0] = json!(f.child.id);
                store.put("object_version", &f.first.version_id, &value)?;
            }
            "digest" => {
                let key = object_command_receipt::digest(&("original", "update-child"))?;
                let mut value: Value = store.get("object_command_receipt", &key)?.unwrap();
                value["inputDigest"] = json!("0".repeat(64));
                store.put("object_command_receipt", &key, &value)?;
            }
            "dangling" | "cross-project" => {
                let mut child = f.child.clone();
                if fault == "dangling" {
                    child.references[0].object_id = "missing".into();
                } else {
                    child.references[0].project_id = "other-project".into();
                }
                store.put("object", &child.id, &child)?;
            }
            "historical-parent" => {
                let mut child = f.child.clone();
                child.versions[0].manifest["parentObjectId"] = json!("missing");
                store.put(
                    "object_version",
                    &child.versions[0].version_id,
                    &(child.id.clone(), &child.versions[0]),
                )?;
                store.put("object", &child.id, &child)?;
            }
            "duplicate-component" | "duplicate-file" => {
                let mut child = f.child.clone();
                if fault == "duplicate-component" {
                    child.components = f.parent.components.clone();
                } else {
                    child.files = f.parent.files.clone();
                    child.files[0].path = child.files[0].path.to_uppercase();
                }
                store.put("object", &child.id, &child)?;
            }
            "parent-cycle" => {
                let mut parent = f.parent.clone();
                parent.parent_object_id = Some(f.child.id.clone());
                store.put("object", &parent.id, &parent)?;
            }
            "unknown" => store.put(
                "object_publication",
                "not-supported",
                &json!({"projectId":"original"}),
            )?,
            _ => unreachable!(),
        }
        drop(storage);
        let before = data_backup::inventory(&f.source)?;
        let destination = f.temp.path().join("rejected");
        assert!(
            copy::inspect_source(&f.source).is_err(),
            "inspection accepted {fault}"
        );
        let error = copy::prepare(f.request(), &destination).unwrap_err();
        let expected = match fault {
            "duplicate-component" => "DUPLICATE_OBJECT_COMPONENT",
            "duplicate-file" => "DUPLICATE_OBJECT_FILE",
            "parent-cycle" => "OBJECT_PARENT_CYCLE",
            "unknown" => "UNKNOWN_ENTITY_KIND",
            _ => "",
        };
        assert!(
            format!("{error:#}").contains(expected),
            "{fault}: {error:#}"
        );
        assert!(!destination.exists(), "created target for {fault}");
        assert_eq!(data_backup::inventory(&f.source)?, before);
    }
    Ok(())
}

#[test]
fn project_derivation_checks_missing_and_corrupt_frozen_blobs_not_working_files() -> Result<()> {
    for missing in [true, false] {
        let f = Fixture::new()?;
        let hash = f.first.manifest["files"][0]["sha256"].as_str().unwrap();
        let blob = f.source.join(".beaver/content/blobs").join(hash);
        if missing {
            fs::rename(&blob, blob.with_extension("missing"))?;
        } else {
            fs::write(&blob, "corrupt")?;
        }
        let before = data_backup::inventory(&f.source)?;
        let destination = f.temp.path().join("rejected");
        let error = copy::prepare(f.request(), &destination).unwrap_err();
        assert!(
            error.to_string().contains("DERIVATION_OBJECT_BLOB_"),
            "{error:#}"
        );
        assert!(!destination.exists());
        assert_eq!(data_backup::inventory(&f.source)?, before);
    }
    Ok(())
}

#[test]
fn project_derivation_object_preparation_never_overwrites_or_activates_damaged_copy() -> Result<()>
{
    let f = Fixture::new()?;
    let preparation = f.temp.path().join("prepared");
    copy::prepare(f.request(), &preparation)?;
    let before = data_backup::inventory(&preparation)?;
    assert!(copy::prepare(f.request(), &preparation).is_err());
    assert_eq!(data_backup::inventory(&preparation)?, before);
    let hash = f.first.manifest["files"][0]["sha256"].as_str().unwrap();
    fs::write(
        preparation.join("project/.beaver/content/blobs").join(hash),
        "corrupt",
    )?;
    assert!(copy::inspect(&preparation).is_err());
    let target = f.temp.path().join("assembled");
    assert!(assembly::create(&preparation, &target).is_err());
    assert!(!target.exists());
    assert!(preparation.join(".beaver-migration-pending").is_file());
    Ok(())
}
