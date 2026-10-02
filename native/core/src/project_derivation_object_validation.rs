//! Derivation-local checks; the legacy migration ownership contract stays unchanged.
use crate::{
    data_backup::Entry,
    object_catalog::{self, ObjectRecord, ObjectReference, ObjectVersion},
    object_command_receipt::{latest_accepted, MAX_REVISION},
    object_version_manifest::{self, VersionStatus},
    project_derivation_object_receipts as receipts,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

fn links(
    objects: &BTreeMap<String, ObjectRecord>,
    owner: &str,
    parent: Option<&str>,
    references: &[ObjectReference],
) -> Result<()> {
    if let Some(parent) = parent {
        ensure!(
            parent != owner && objects.contains_key(parent),
            "DERIVATION_OBJECT_PARENT_MISSING"
        );
    }
    for reference in references {
        ensure!(reference.object_id != owner, "SELF_OBJECT_REFERENCE");
        let target = objects
            .get(&reference.object_id)
            .context("DERIVATION_OBJECT_REFERENCE_MISSING")?;
        let version = target
            .versions
            .iter()
            .find(|v| Some(&v.version_id) == reference.version_id.as_ref())
            .context("DERIVATION_OBJECT_REFERENCE_VERSION_MISSING")?;
        ensure!(
            object_version_manifest::read_frozen_version(target, version)?.status
                == VersionStatus::Accepted,
            "DERIVATION_OBJECT_REFERENCE_NOT_ACCEPTED"
        );
    }
    Ok(())
}

fn projection(objects: &BTreeMap<String, ObjectRecord>, object: &ObjectRecord) -> Result<()> {
    object_catalog::validate(object)?;
    ensure!(object.revision <= MAX_REVISION, "INVALID_OBJECT_REVISION");
    links(
        objects,
        &object.id,
        object.parent_object_id.as_deref(),
        &object.references,
    )?;
    for version in &object.versions {
        let manifest = object_version_manifest::read_frozen_version(object, version)?;
        links(
            objects,
            &object.id,
            manifest.parent_object_id.as_deref(),
            &manifest.references,
        )?;
    }
    Ok(())
}

fn history(current: &ObjectRecord, previous: &ObjectRecord) -> Result<()> {
    ensure!(
        previous.revision <= current.revision,
        "DERIVATION_OBJECT_RECEIPT_FUTURE_REVISION"
    );
    if previous.revision == current.revision {
        ensure!(
            previous == current,
            "DERIVATION_OBJECT_RECEIPT_PROJECTION_MISMATCH"
        );
    }
    for version in &previous.versions {
        let saved = current
            .versions
            .iter()
            .find(|v| v.version_id == version.version_id)
            .context("DERIVATION_OBJECT_RECEIPT_VERSION_MISSING")?;
        let mut original = object_version_manifest::read_frozen_version(previous, version)?;
        let present = object_version_manifest::read_frozen_version(current, saved)?;
        // Acceptance is the only supported mutation of a frozen version.
        if original.status == VersionStatus::Captured && present.status == VersionStatus::Accepted {
            original.status = VersionStatus::Accepted;
        }
        ensure!(
            original == present,
            "DERIVATION_OBJECT_RECEIPT_VERSION_MISMATCH"
        );
    }
    Ok(())
}

pub(crate) fn snapshot(
    objects: &BTreeMap<String, ObjectRecord>,
    object: &ObjectRecord,
) -> Result<()> {
    projection(objects, object)?;
    history(
        objects
            .get(&object.id)
            .context("DERIVATION_OBJECT_RECEIPT_OWNER_MISSING")?,
        object,
    )
}

fn current_ownership(objects: &BTreeMap<String, ObjectRecord>) -> Result<()> {
    let mut components = BTreeSet::new();
    let mut files = BTreeSet::new();
    for object in objects.values() {
        for component in &object.components {
            ensure!(
                components.insert(&component.id),
                "DUPLICATE_OBJECT_COMPONENT: {}",
                component.id
            );
        }
        for file in &object.files {
            ensure!(
                files.insert(file.path.to_lowercase()),
                "DUPLICATE_OBJECT_FILE: {}",
                file.path
            );
        }
        let mut visited = BTreeSet::from([object.id.as_str()]);
        let mut next = object.parent_object_id.as_deref();
        while let Some(id) = next {
            ensure!(visited.insert(id), "OBJECT_PARENT_CYCLE");
            let parent = objects
                .get(id)
                .context("DERIVATION_OBJECT_PARENT_MISSING")?;
            next = parent.parent_object_id.as_deref();
        }
    }
    Ok(())
}

pub(crate) fn validate(connection: &Connection, entities: &[Entity], project: &str) -> Result<()> {
    let mut objects = BTreeMap::new();
    for entity in entities.iter().filter(|e| e.kind == "object") {
        let object: ObjectRecord =
            serde_json::from_value(entity.value.clone().context("invalid object JSON")?)?;
        ensure!(
            object.id == entity.id && object.project_id == project,
            "DERIVATION_OBJECT_IDENTITY_MISMATCH"
        );
        objects.insert(entity.id.clone(), object);
    }
    let mut versions = BTreeMap::new();
    for entity in entities.iter().filter(|e| e.kind == "object_version") {
        let saved: (String, ObjectVersion) =
            serde_json::from_value(entity.value.clone().context("invalid version JSON")?)?;
        let owner = objects
            .get(&saved.0)
            .context("DERIVATION_OBJECT_VERSION_OWNER_MISSING")?;
        ensure!(
            saved.1.version_id == entity.id && owner.versions.iter().any(|v| v == &saved.1),
            "DERIVATION_OBJECT_VERSION_MISMATCH"
        );
        versions.insert(entity.id.as_str(), saved);
    }
    for object in objects.values() {
        projection(&objects, object)?;
        for version in &object.versions {
            ensure!(
                versions.get(version.version_id.as_str())
                    == Some(&(object.id.clone(), version.clone())),
                "DERIVATION_OBJECT_VERSION_MISMATCH"
            );
        }
    }
    // Validate the stored graph, not a new write: pinned versions retain their own metadata.
    current_ownership(&objects)?;
    let mut revisions = BTreeSet::new();
    for entity in entities
        .iter()
        .filter(|e| e.kind == "object_command_receipt")
    {
        let receipt = receipts::read(
            &entity.id,
            entity.value.as_ref().context("invalid receipt JSON")?,
            project,
        )?;
        let object = &receipt.result.object;
        let current = objects
            .get(&object.id)
            .context("DERIVATION_OBJECT_RECEIPT_OWNER_MISSING")?;
        projection(&objects, object)?;
        history(current, object)?;
        ensure!(
            revisions.insert((object.id.clone(), object.revision)),
            "DERIVATION_OBJECT_RECEIPT_REVISION_CONFLICT"
        );
    }
    for object in objects.values() {
        latest_accepted(connection, object)?;
    }
    Ok(())
}

/// Inventory was hashed while holding the source's exclusive filesystem/database locks.
/// Never require live working files to match an old frozen version.
pub(crate) fn blobs(connection: &Connection, entries: &[Entry]) -> Result<()> {
    let inventory: BTreeMap<_, _> = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut statement =
        connection.prepare("SELECT value FROM entities WHERE kind='object_version'")?;
    for row in statement.query_map([], |row| row.get::<_, String>(0))? {
        let (_, version): (String, ObjectVersion) = serde_json::from_str(&row?)?;
        let manifest: object_version_manifest::VersionManifest =
            serde_json::from_value(version.manifest)?;
        for file in manifest.files {
            let path = format!(".beaver/content/blobs/{}", file.sha256);
            let entry = inventory
                .get(path.as_str())
                .context("DERIVATION_OBJECT_BLOB_MISSING")?;
            ensure!(
                entry.bytes == file.bytes && entry.sha256.as_ref() == Some(&file.sha256),
                "DERIVATION_OBJECT_BLOB_MISMATCH"
            );
        }
    }
    Ok(())
}
