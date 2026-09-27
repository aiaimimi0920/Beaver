use super::*;
use crate::{
    object_catalog_test_fixture::{update_request, Fixture},
    project_storage::ProjectStore,
};
use serde_json::{json, Value};
use std::sync::{Arc, Barrier};

#[test]
fn empty_registration_is_durable_and_replays_its_original_receipt() -> Result<()> {
    let f = Fixture::new()?;
    let request = f.request("register-1");
    let first = register(&f.runtime, &request)?;
    assert!(first.object.versions.is_empty() && first.object.files.is_empty());
    assert_eq!(f.count("task")?, 0);
    assert_eq!(f.count("object_version")?, 0);
    let mut edit = update_request(&first.object, "edit-1");
    edit.name = "Renamed".into();
    let changed = update(&f.runtime, &edit)?;
    assert_eq!(changed.object.revision, 1);
    assert_eq!(register(&f.runtime, &request)?, first);
    assert_eq!(update(&f.runtime, &edit)?, changed);
    let mut conflicting = request.clone();
    conflicting.name = "Other".into();
    assert!(register(&f.runtime, &conflicting)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REQUEST_CONFLICT"));
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(register(&reopened, &request)?, first);
    assert_eq!(
        object_catalog::get(&reopened.store().lock().unwrap(), &first.object.id)?,
        Some(changed.object)
    );
    Ok(())
}

#[test]
fn concurrent_registration_with_the_same_request_creates_one_object() -> Result<()> {
    let f = Fixture::new()?;
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let runtime = f.runtime.clone();
        let request = f.request("same-request");
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            register(&runtime, &request)
        }));
    }
    barrier.wait();
    let first = workers.remove(0).join().unwrap()?;
    assert_eq!(workers.remove(0).join().unwrap()?, first);
    assert_eq!(f.count("object")?, 1);
    assert_eq!(f.count("object_command_receipt")?, 1);
    Ok(())
}

#[test]
fn concurrent_metadata_writes_compare_revision_and_keep_accepted_history() -> Result<()> {
    let f = Fixture::new()?;
    f.accepted("hero", "v1")?;
    let original = f.object("hero")?;
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for name in ["first", "second"] {
        let runtime = f.runtime.clone();
        let barrier = barrier.clone();
        let mut request = update_request(&original, name);
        request.name = name.into();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            update(&runtime, &request)
        }));
    }
    barrier.wait();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(results
        .iter()
        .find_map(|result| result.as_ref().err())
        .unwrap()
        .to_string()
        .contains("OBJECT_REVISION_CONFLICT"));
    assert_eq!(f.object("hero")?.versions, original.versions);
    assert_eq!(f.count("object_command_receipt")?, 1);
    Ok(())
}

#[test]
fn typed_requests_reject_acceptance_and_nested_unrecognized_fields() -> Result<()> {
    let f = Fixture::new()?;
    for key in ["id", "versions", "status", "accepted", "revision"] {
        let mut request = serde_json::to_value(f.request("bad"))?;
        request[key] = json!("injected");
        assert!(
            serde_json::from_value::<RegisterRequest>(request).is_err(),
            "{key}"
        );
    }
    let mut request = serde_json::to_value(f.request("bad"))?;
    request["files"] = json!([{"path":"hero.tscn","role":"scene","accepted":true}]);
    assert!(serde_json::from_value::<RegisterRequest>(request).is_err());
    let mut foreign = f.request("foreign");
    foreign.project_id = "project-2".into();
    assert!(register(&f.runtime, &foreign)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_PROJECT_MISMATCH"));
    assert_eq!(f.count("object")?, 0);
    Ok(())
}

#[test]
fn request_identity_is_shared_between_commands_and_invalid_revisions_roll_back() -> Result<()> {
    let f = Fixture::new()?;
    let original = register(&f.runtime, &f.request("shared"))?;
    let request = update_request(&original.object, "shared");
    assert!(update(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REQUEST_CONFLICT"));
    let mut invalid = update_request(&original.object, "invalid");
    invalid.expected_revision = MAX_REVISION;
    assert!(update(&f.runtime, &invalid)
        .unwrap_err()
        .to_string()
        .contains("INVALID_OBJECT_REVISION"));
    assert_eq!(f.count("object_command_receipt")?, 1);
    let store = f.runtime.store();
    let store = store.lock().unwrap();
    assert_eq!(store.list::<Value>("task")?.len(), 0);
    Ok(())
}
