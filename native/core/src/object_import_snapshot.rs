//! Pure selection of accepted version closures shared by inspection and preparation.
use crate::{
    object_catalog::ObjectRecord,
    object_version_manifest::{self, VersionManifest},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportVersion {
    pub object_id: String,
    pub version_id: String,
    pub source_digest: Option<String>,
    pub blocker: Option<String>,
}

pub(crate) struct Selection {
    pub versions: Vec<VersionManifest>,
    pub digest: String,
}

pub(crate) fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub(crate) fn select(
    objects: &[ObjectRecord],
    project_id: &str,
    object_id: &str,
    version_id: &str,
) -> Result<Selection> {
    select_with(objects, project_id, object_id, version_id, false)
}

pub(crate) fn select_frozen(
    objects: &[ObjectRecord],
    project_id: &str,
    object_id: &str,
    version_id: &str,
) -> Result<Selection> {
    select_with(objects, project_id, object_id, version_id, true)
}

fn select_with(
    objects: &[ObjectRecord],
    project_id: &str,
    object_id: &str,
    version_id: &str,
    historical: bool,
) -> Result<Selection> {
    let mut catalog = BTreeMap::new();
    for object in objects {
        ensure!(
            object.project_id == project_id && catalog.insert(&object.id, object).is_none(),
            "IMPORT_INVALID_OBJECT_CATALOG: {}",
            object.id
        );
    }
    let mut pending = vec![(object_id.to_owned(), version_id.to_owned())];
    let mut versions = BTreeMap::new();
    let mut component_owners = BTreeMap::new();
    let mut file_owners = BTreeMap::new();
    let mut version_owners = BTreeMap::new();
    while let Some((object_id, version_id)) = pending.pop() {
        let key = (object_id.clone(), version_id.clone());
        if versions.contains_key(&key) {
            continue;
        }
        let object = catalog
            .get(&object_id)
            .with_context(|| format!("IMPORT_REFERENCE_MISSING: {object_id}/{version_id}"))?;
        let matching: Vec<_> = object
            .versions
            .iter()
            .filter(|version| version.version_id == version_id)
            .collect();
        ensure!(
            matching.len() == 1,
            "IMPORT_VERSION_MISSING_OR_DUPLICATE: {object_id}/{version_id}"
        );
        let manifest = if historical {
            object_version_manifest::read_frozen_version(object, matching[0])?
        } else {
            object_version_manifest::read(object, matching[0])?
        };
        owner(&mut version_owners, &version_id, &object_id, "VERSION")?;
        for component in &manifest.components {
            owner(
                &mut component_owners,
                &component.id,
                &object_id,
                "COMPONENT",
            )?;
        }
        for file in &manifest.files {
            owner(
                &mut file_owners,
                &file.path.to_lowercase(),
                &object_id,
                "FILE",
            )?;
        }
        for reference in &manifest.references {
            pending.push((
                reference.object_id.clone(),
                reference
                    .version_id
                    .clone()
                    .context("IMPORT_REFERENCE_NOT_PINNED")?,
            ));
        }
        versions.insert(key, manifest);
    }
    let versions: Vec<_> = versions.into_values().collect();
    Ok(Selection {
        digest: digest(&versions)?,
        versions,
    })
}

fn owner(
    owners: &mut BTreeMap<String, String>,
    id: &str,
    object_id: &str,
    kind: &str,
) -> Result<()> {
    if let Some(existing) = owners.insert(id.to_owned(), object_id.to_owned()) {
        ensure!(
            existing == object_id,
            "IMPORT_{kind}_OWNER_CONFLICT: {id}: {existing}/{object_id}"
        );
    }
    Ok(())
}

pub(crate) fn inspect(objects: &[ObjectRecord], project_id: &str) -> Vec<ImportVersion> {
    objects
        .iter()
        .flat_map(|object| {
            object.versions.iter().map(|version| {
                let result = select(objects, project_id, &object.id, &version.version_id);
                let (source_digest, blocker) = match result {
                    Ok(selection) => (Some(selection.digest), None),
                    Err(error) => (None, Some(error.to_string())),
                };
                ImportVersion {
                    object_id: object.id.clone(),
                    version_id: version.version_id.clone(),
                    source_digest,
                    blocker,
                }
            })
        })
        .collect()
}
