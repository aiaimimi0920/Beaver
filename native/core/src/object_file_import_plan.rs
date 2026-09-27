//! Deterministic target layout; grouping changes ownership, never relative paths.
use crate::{
    object_catalog::{ObjectFile, ObjectRecord, ObjectVersion},
    object_file_import_preparation::Preparation,
    object_version_manifest::{VersionFile, VersionManifest, VersionStatus},
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub(super) fn objects(preparation: &Preparation) -> Result<Vec<ObjectRecord>> {
    let mut objects = BTreeMap::<String, (ObjectRecord, Vec<VersionFile>)>::new();
    let mut total = 0u64;
    for file in &preparation.source.files {
        total = total.checked_add(file.bytes).context("IMPORT_TOO_LARGE")?;
        ensure!(total <= 512 * 1024 * 1024, "IMPORT_TOO_LARGE");
        let group = preparation
            .groups
            .iter()
            .find(|group| group.paths.contains(&file.path));
        let (id, name) = match group {
            Some(group) => (
                &preparation.identity_map.groups[&group.id],
                group.name.clone(),
            ),
            None => (
                &preparation.identity_map.files[&file.path],
                file.relative_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("Imported file")
                    .to_owned(),
            ),
        };
        let root = preparation
            .source
            .source
            .paths
            .iter()
            .position(|path| path == &file.source_path)
            .context("IMPORT_SOURCE_ROOT_MISSING")?;
        let path = format!(
            "imports/{}/root-{root}/{}",
            preparation.preparation_id, file.relative_path
        );
        ensure!(
            crate::object_version_manifest::valid_asset_path(&path),
            "IMPORT_TARGET_PATH_INVALID"
        );
        let (object, frozen) = objects.entry(id.clone()).or_insert_with(|| {
            (
                ObjectRecord {
                    id: id.clone(),
                    project_id: preparation.target_project_id.clone(),
                    name,
                    category: crate::object_catalog::default_category(),
                    tags: vec![],
                    thumbnail_path: None,
                    parent_object_id: None,
                    revision: 0,
                    components: vec![],
                    files: vec![],
                    references: vec![],
                    versions: vec![],
                },
                vec![],
            )
        });
        object.files.push(ObjectFile {
            path: path.clone(),
            role: "source".into(),
        });
        frozen.push(VersionFile {
            path,
            role: "source".into(),
            bytes: file.bytes,
            sha256: file.sha256.clone(),
        });
    }
    ensure!(objects.len() <= 1024, "IMPORT_TOO_MANY_OBJECTS");
    objects
        .into_values()
        .map(|(mut object, files)| {
            let version_id = format!(
                "import-version-{}",
                crate::object_import_snapshot::digest(&(&preparation.preparation_id, &object.id))?
            );
            let manifest = VersionManifest {
                schema_version: 1,
                project_id: object.project_id.clone(),
                object_id: object.id.clone(),
                version_id: version_id.clone(),
                status: VersionStatus::ImportedPendingValidation,
                name: object.name.clone(),
                category: object.category.clone(),
                tags: vec![],
                thumbnail_path: None,
                parent_object_id: None,
                components: vec![],
                files,
                references: vec![],
            };
            object.versions.push(ObjectVersion {
                version_id,
                manifest: serde_json::to_value(manifest)?,
            });
            object.revision = 1;
            Ok(object)
        })
        .collect()
}
