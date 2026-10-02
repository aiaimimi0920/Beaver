use crate::{
    data_backup, object_catalog, object_command_receipt,
    object_version_file::{self, Content},
    object_version_manifest, project_derivation_assembly as assembly,
    project_derivation_copy as copy,
    project_derivation_identity::generated,
    project_derivation_validation_records::Rewrite,
    project_storage::ProjectStore,
    project_storage_router::ProjectStorageRouter,
    store::Store,
};
use anyhow::Result;
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[path = "project_derivation_object_fixture.rs"]
mod fixture;
use fixture::{accept, capture, update, Fixture};

#[path = "project_derivation_object_rejection_tests.rs"]
mod rejection;

#[test]
fn project_derivation_objects_remain_independent_editable_and_replayable_after_reopen() -> Result<()>
{
    let f = Fixture::new()?;
    let before = data_backup::inventory(&f.source)?;
    copy::inspect_source(&f.source)?;
    let preparation = f.temp.path().join("prepared");
    let prepared = copy::prepare(f.request(), &preparation)?;
    let map = Rewrite(&prepared.identities);
    let parent_id = map.key("object", &f.parent.id)?.id;
    let child_id = map.key("object", &f.child.id)?.id;
    let first_id = map.key("object_version", &f.first.version_id)?.id;
    let second_id = map.key("object_version", &f.second.version_id)?.id;
    assert_ne!(parent_id, f.parent.id);
    assert_ne!(first_id, f.first.version_id);
    assert_ne!(second_id, f.second.version_id);
    let prepared_before = data_backup::inventory(&preparation)?;
    let destination = f.temp.path().join("assembled");
    let assembled = assembly::create(&preparation, &destination)?;
    assert!(ProjectStore::open(&assembled.binding, "derived").is_err());
    assert_eq!(assembled.tasks_interrupted, 0);
    assembly::activate(&preparation, &destination)?;
    let data = f.temp.path().join("host");
    let host = Arc::new(Mutex::new(Store::open(&data)?));
    let router = ProjectStorageRouter::new(host);
    router.register_assembly(&preparation, &destination, &data)?;
    let runtime = router.open_registered("derived")?;
    let (parent, child) = {
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        let parent = object_catalog::get(&store, &parent_id)?.unwrap();
        let child = object_catalog::get(&store, &child_id)?.unwrap();
        assert!(object_catalog::get(&store, &f.parent.id)?.is_none());
        assert_eq!(
            object_catalog::search(&store, "derived", Some("Parent"))?.len(),
            1
        );
        assert_eq!(object_catalog::list(&store, "derived")?.len(), 3);
        assert_eq!(
            object_command_receipt::latest_accepted(&store.connection, &parent)?,
            Some(first_id.clone())
        );
        assert_eq!(
            object_command_receipt::latest_accepted(&store.connection, &child)?,
            Some(child.versions[0].version_id.clone())
        );
        (parent, child)
    };
    assert_eq!(parent.project_id, "derived");
    assert_eq!(child.parent_object_id.as_deref(), Some(parent_id.as_str()));
    assert_eq!(child.references[0].project_id, "derived");
    assert_eq!(child.references[0].object_id, parent_id);
    assert_eq!(
        child.references[0].version_id.as_deref(),
        Some(first_id.as_str())
    );
    assert_ne!(parent.components[0].id, "component-parent");
    let manifest = object_version_manifest::read_frozen_version(&parent, &parent.versions[0])?;
    assert_eq!(manifest.components, parent.components);
    assert_eq!(
        manifest.files[0].sha256,
        f.first.manifest["files"][0]["sha256"]
    );
    let old_child = object_version_manifest::read_frozen_version(&child, &child.versions[0])?;
    assert_eq!(old_child.category, "old category");
    assert_eq!(child.category, "new category");
    assert_ne!(old_child.components[0].id, child.components[0].id);
    assert_ne!(old_child.components[0].id, "component-old");
    assert_eq!(old_child.parent_object_id, child.parent_object_id);
    assert_eq!(old_child.references, child.references);
    let response = object_version_file::read(
        &runtime,
        &object_version_file::Request {
            project_id: "derived".into(),
            object_id: parent_id.clone(),
            version_id: first_id.clone(),
            path: "parent.txt".into(),
            sha256: manifest.files[0].sha256.clone(),
        },
    )?;
    assert_eq!(
        response.content,
        Content::Text {
            text: "frozen one: original component-parent".into()
        }
    );
    assert_eq!(
        fs::read(runtime.project_root().join("parent.txt"))?,
        b"working original component-parent"
    );
    // Remapped historical inputs still replay the exact historical result, not today's projection.
    let old_accept = crate::object_version_acceptance::AcceptanceRequest {
        project_id: "derived".into(),
        request_id: generated(&prepared.request, "object_command_request", "accept-one")?,
        object_id: parent_id,
        version_id: first_id,
        expected_revision: 3,
    };
    assert_eq!(
        crate::object_version_acceptance::accept(&runtime, &old_accept)?.object,
        parent
    );
    let changed = update(&runtime, &child, "target-update")?;
    assert_eq!(update(&runtime, &child, "target-update")?, changed);
    let captured = capture(&runtime, &changed.object, "target-capture")?;
    let accepted = accept(
        &runtime,
        &captured.object,
        captured.version_id.as_deref().unwrap(),
        "target-accept",
    )?;
    assert_eq!(
        accept(
            &runtime,
            &captured.object,
            captured.version_id.as_deref().unwrap(),
            "target-accept"
        )?,
        accepted
    );
    assert_eq!(
        router.register_assembly(&preparation, &destination, &data)?["hostRegistrationChanged"],
        false
    );
    drop(runtime);
    router.close("derived")?;
    let reopened = router.open_registered("derived")?;
    let handle = reopened.store();
    let store = handle.lock().unwrap();
    assert_eq!(
        object_catalog::get(&store, &child_id)?,
        Some(accepted.object.clone())
    );
    assert_eq!(
        object_command_receipt::latest_accepted(&store.connection, &accepted.object)?,
        accepted.version_id
    );
    drop(store);
    drop(handle);
    drop(reopened);
    router.close("derived")?;
    assert_eq!(data_backup::inventory(&f.source)?, before);
    assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    // A used derived project can itself be derived again with its reconstructed receipts.
    copy::prepare(
        copy::Request {
            request_id: "derive-again".into(),
            source: assembled.binding,
            source_project_id: "derived".into(),
            target_project_id: "third".into(),
        },
        &f.temp.path().join("third-prepared"),
    )?;
    Ok(())
}

#[test]
fn project_derivation_keeps_pinned_history_after_referenced_parent_metadata_changes() -> Result<()>
{
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    let mut parent = f.parent.clone();
    parent.category = "updated parent category".into();
    parent.tags = vec!["updated parent tag".into()];
    update(&runtime, &parent, "update-parent")?;
    drop(runtime);
    let before = data_backup::inventory(&f.source)?;
    copy::inspect_source(&f.source)?;
    let preparation = f.temp.path().join("prepared");
    let prepared = copy::prepare(f.request(), &preparation)?;
    let map = Rewrite(&prepared.identities);
    let parent_id = map.key("object", &f.parent.id)?.id;
    let child_id = map.key("object", &f.child.id)?.id;
    let first_id = map.key("object_version", &f.first.version_id)?.id;
    let destination = f.temp.path().join("assembled");
    let assembled = assembly::create(&preparation, &destination)?;
    assembly::activate(&preparation, &destination)?;
    let runtime = ProjectStore::open(&assembled.binding, "derived")?.into_runtime();
    let (parent, child) = {
        let handle = runtime.store();
        let store = handle.lock().unwrap();
        let parent = object_catalog::get(&store, &parent_id)?.unwrap();
        assert_eq!(
            object_command_receipt::latest_accepted(&store.connection, &parent)?,
            Some(first_id.clone())
        );
        (parent, object_catalog::get(&store, &child_id)?.unwrap())
    };
    assert_eq!(parent.category, "updated parent category");
    assert_eq!(parent.tags, vec!["updated parent tag"]);
    assert_eq!(child.references[0].version_id.as_ref(), Some(&first_id));
    let frozen = object_version_manifest::read_frozen_version(&parent, &parent.versions[0])?;
    assert_eq!(frozen.category, f.parent.category);
    assert_eq!(frozen.tags, f.parent.tags);
    let response = object_version_file::read(
        &runtime,
        &object_version_file::Request {
            project_id: "derived".into(),
            object_id: parent_id,
            version_id: first_id,
            path: "parent.txt".into(),
            sha256: frozen.files[0].sha256.clone(),
        },
    )?;
    assert_eq!(
        response.content,
        Content::Text {
            text: "frozen one: original component-parent".into()
        }
    );
    let next = capture(&runtime, &parent, "capture-updated-parent")?;
    accept(
        &runtime,
        &next.object,
        next.version_id.as_deref().unwrap(),
        "accept-updated-parent",
    )?;
    assert_eq!(data_backup::inventory(&f.source)?, before);
    Ok(())
}
