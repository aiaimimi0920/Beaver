//! Remap the frozen closure, retaining asset paths and every pinned version.
use crate::{
    object_catalog::{ObjectFile, ObjectRecord, ObjectVersion},
    object_import_preparation::{self as preparation, Preparation, Request},
    object_import_snapshot,
    object_version_manifest::{self, VersionManifest, VersionStatus},
};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

fn record(manifest: &VersionManifest) -> ObjectRecord {
    ObjectRecord {
        id: manifest.object_id.clone(),
        project_id: manifest.project_id.clone(),
        name: manifest.name.clone(),
        category: manifest.category.clone(),
        tags: manifest.tags.clone(),
        thumbnail_path: manifest.thumbnail_path.clone(),
        parent_object_id: manifest.parent_object_id.clone(),
        revision: 1,
        components: manifest.components.clone(),
        files: manifest
            .files
            .iter()
            .map(|file| ObjectFile {
                path: file.path.clone(),
                role: file.role.clone(),
            })
            .collect(),
        references: manifest.references.clone(),
        versions: vec![],
    }
}

pub(super) fn objects(receipt: &Preparation) -> Result<Vec<ObjectRecord>> {
    ensure!(
        receipt.schema_version == 1 && !receipt.ready_to_commit,
        "IMPORT_PREPARATION_UNSUPPORTED"
    );
    let request = Request {
        request_id: receipt.request_id.clone(),
        target_project_id: receipt.target_project_id.clone(),
        source_path: receipt.source_path.clone(),
        source_project_id: receipt.source_project_id.clone(),
        object_id: receipt.source_object_id.clone(),
        baseline: receipt.baseline.clone(),
        source_digest: receipt.source_digest.clone(),
    };
    ensure!(
        receipt.target_project_id != receipt.source_project_id
            && receipt.preparation_id
                == format!(
                    "import-{}",
                    object_import_snapshot::digest(&(
                        1,
                        &receipt.target_project_id,
                        &receipt.request_id
                    ))?
                )
            && receipt.identity_map == preparation::identities(&request, &receipt.versions)?,
        "IMPORT_PREPARATION_IDENTITY_MISMATCH"
    );
    let mut source = BTreeMap::<String, ObjectRecord>::new();
    let mut hashes = BTreeSet::new();
    let mut total = 0u64;
    for manifest in &receipt.versions {
        let object = source
            .entry(manifest.object_id.clone())
            .or_insert_with(|| record(manifest));
        let version = ObjectVersion {
            version_id: manifest.version_id.clone(),
            manifest: serde_json::to_value(manifest)?,
        };
        object_version_manifest::read(object, &version)?;
        object.versions.push(version);
        for file in &manifest.files {
            if hashes.insert(&file.sha256) {
                total = total.checked_add(file.bytes).context("IMPORT_TOO_LARGE")?;
            }
        }
    }
    ensure!(
        !source.is_empty() && source.len() <= 1024 && total <= 512 * 1024 * 1024,
        "IMPORT_TOO_LARGE"
    );
    let selected = object_import_snapshot::select(
        &source.into_values().collect::<Vec<_>>(),
        &receipt.source_project_id,
        &receipt.source_object_id,
        &receipt.accepted_version_id,
    )?;
    ensure!(
        selected.versions == receipt.versions && selected.digest == receipt.source_digest,
        "IMPORT_PREPARATION_CONTENT_MISMATCH"
    );
    let mut objects = BTreeMap::<String, ObjectRecord>::new();
    for source in &receipt.versions {
        let mut manifest = source.clone();
        manifest.project_id = receipt.target_project_id.clone();
        manifest.object_id = receipt.identity_map.objects[&source.object_id].clone();
        manifest.version_id = receipt.identity_map.versions[&source.version_id].clone();
        manifest.status = VersionStatus::ImportedPendingValidation;
        // Organizational parents outside the content closure are not imported.
        manifest.parent_object_id = source
            .parent_object_id
            .as_ref()
            .and_then(|id| receipt.identity_map.objects.get(id))
            .cloned();
        for component in &mut manifest.components {
            component.id = receipt.identity_map.components[&component.id].clone();
        }
        for reference in &mut manifest.references {
            reference.project_id = receipt.target_project_id.clone();
            reference.object_id = receipt.identity_map.objects[&reference.object_id].clone();
            reference.version_id = Some(
                receipt.identity_map.versions[reference
                    .version_id
                    .as_ref()
                    .context("IMPORT_REFERENCE_NOT_PINNED")?]
                .clone(),
            );
        }
        let object = objects
            .entry(manifest.object_id.clone())
            .or_insert_with(|| record(&manifest));
        let version = ObjectVersion {
            version_id: manifest.version_id.clone(),
            manifest: serde_json::to_value(&manifest)?,
        };
        // The explicitly selected root is the materialized projection; dependencies
        // use their first frozen version. Other versions retain their own blobs.
        if source.object_id == receipt.source_object_id
            && source.version_id == receipt.accepted_version_id
        {
            let versions = std::mem::take(&mut object.versions);
            *object = record(&manifest);
            object.versions = versions;
            object.versions.insert(0, version);
        } else {
            object.versions.push(version);
        }
    }
    for object in objects.values() {
        crate::object_catalog::validate(object)?;
        for version in &object.versions {
            object_version_manifest::read_version(object, version)?;
        }
        let mut seen = BTreeSet::from([object.id.as_str()]);
        let mut parent = object.parent_object_id.as_deref();
        while let Some(id) = parent {
            ensure!(seen.insert(id), "OBJECT_PARENT_CYCLE");
            parent = objects
                .get(id)
                .context("OBJECT_PARENT_NOT_FOUND")?
                .parent_object_id
                .as_deref();
        }
    }
    Ok(objects.into_values().collect())
}
