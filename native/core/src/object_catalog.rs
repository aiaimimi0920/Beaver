//! Project-owned object registration and immutable version manifests.
//!
//! This is the storage boundary for F2.1.  It deliberately stores only
//! registration metadata; execution, preview and publication remain owned by
//! their existing domains.
use crate::object_version_manifest::{self, valid_asset_path, valid_id};
use anyhow::{ensure, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const OBJECT_KIND: &str = "object";
const VERSION_KIND: &str = "object_version";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectComponent {
    pub id: String,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectFile {
    pub path: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectReference {
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "objectId")]
    pub object_id: String,
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectVersion {
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub manifest: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectRecord {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "thumbnailPath", default)]
    pub thumbnail_path: Option<String>,
    #[serde(rename = "parentObjectId", default)]
    pub parent_object_id: Option<String>,
    #[serde(default)]
    pub revision: u64,
    pub components: Vec<ObjectComponent>,
    pub files: Vec<ObjectFile>,
    pub references: Vec<ObjectReference>,
    pub versions: Vec<ObjectVersion>,
}

pub(crate) fn default_category() -> String {
    "其他".to_owned()
}

pub(crate) fn validate_metadata(
    owner_id: &str,
    category: &str,
    tags: &[String],
    thumbnail_path: Option<&str>,
    parent_object_id: Option<&str>,
) -> Result<()> {
    ensure!(
        !category.trim().is_empty() && category.len() <= 128 && category.trim() == category,
        "INVALID_OBJECT_CATALOG: invalid category"
    );
    ensure!(tags.len() <= 64, "INVALID_OBJECT_CATALOG: too many tags");
    let mut unique_tags = std::collections::HashSet::new();
    for tag in tags {
        ensure!(
            !tag.trim().is_empty() && tag.len() <= 128 && tag.trim() == tag,
            "INVALID_OBJECT_CATALOG: invalid tag"
        );
        ensure!(
            unique_tags.insert(tag.to_ascii_lowercase()),
            "DUPLICATE_OBJECT_TAG: {tag}"
        );
    }
    if let Some(path) = thumbnail_path {
        ensure!(
            valid_asset_path(path),
            "INVALID_OBJECT_CATALOG: invalid thumbnail path"
        );
    }
    if let Some(parent) = parent_object_id {
        id(parent, "parent object id")?;
        ensure!(parent != owner_id, "SELF_OBJECT_PARENT");
    }
    Ok(())
}

fn id(value: &str, label: &str) -> Result<()> {
    ensure!(valid_id(value), "INVALID_OBJECT_CATALOG: invalid {label}");
    Ok(())
}

pub(crate) fn validate(record: &ObjectRecord) -> Result<()> {
    id(&record.id, "object id")?;
    id(&record.project_id, "project id")?;
    ensure!(
        !record.name.trim().is_empty() && record.name.len() <= 800,
        "INVALID_OBJECT_CATALOG: invalid name"
    );
    validate_metadata(
        &record.id,
        &record.category,
        &record.tags,
        record.thumbnail_path.as_deref(),
        record.parent_object_id.as_deref(),
    )?;
    ensure!(
        record.components.len() <= 256
            && record.files.len() <= 1024
            && record.references.len() <= 256,
        "INVALID_OBJECT_CATALOG: too many entries"
    );
    let mut component_ids = std::collections::HashSet::new();
    for component in &record.components {
        id(&component.id, "component id")?;
        ensure!(
            component_ids.insert(&component.id),
            "DUPLICATE_OBJECT_COMPONENT: {}",
            component.id
        );
        ensure!(
            !component.kind.trim().is_empty()
                && component.kind.len() <= 128
                && !component.name.trim().is_empty()
                && component.name.len() <= 800,
            "INVALID_OBJECT_CATALOG: component fields"
        );
    }
    let mut file_paths = std::collections::HashSet::new();
    for file in &record.files {
        ensure!(
            valid_asset_path(&file.path),
            "INVALID_OBJECT_CATALOG: invalid file path"
        );
        ensure!(
            file_paths.insert(file.path.to_lowercase()),
            "DUPLICATE_OBJECT_FILE: {}",
            file.path
        );
        ensure!(
            !file.role.trim().is_empty() && file.role.len() <= 128,
            "INVALID_OBJECT_CATALOG: file role"
        );
    }
    let mut version_ids = std::collections::HashSet::new();
    for version in &record.versions {
        id(&version.version_id, "version id")?;
        ensure!(
            version_ids.insert(&version.version_id),
            "DUPLICATE_OBJECT_VERSION: {}",
            version.version_id
        );
    }
    let mut references = std::collections::HashSet::new();
    for reference in &record.references {
        ensure!(
            reference.project_id == record.project_id,
            "CROSS_PROJECT_OBJECT_REFERENCE"
        );
        id(&reference.project_id, "reference project id")?;
        id(&reference.object_id, "reference object id")?;
        ensure!(reference.object_id != record.id, "SELF_OBJECT_REFERENCE");
        ensure!(
            reference.version_id.as_deref().is_some_and(valid_id),
            "OBJECT_REFERENCE_NOT_PINNED"
        );
        ensure!(
            references.insert((&reference.object_id, &reference.version_id)),
            "DUPLICATE_OBJECT_REFERENCE"
        );
    }
    Ok(())
}

fn duplicate_ownership(connection: &Connection, record: &ObjectRecord) -> Result<()> {
    let mut statement = connection.prepare("SELECT id,value FROM entities WHERE kind=?")?;
    let rows = statement.query_map([OBJECT_KIND], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, json) = row?;
        if id == record.id {
            continue;
        }
        let existing: ObjectRecord =
            serde_json::from_str(&json).context("invalid stored object")?;
        if existing.project_id != record.project_id {
            continue;
        }
        for component in &record.components {
            ensure!(
                !existing
                    .components
                    .iter()
                    .any(|item| item.id == component.id),
                "DUPLICATE_OBJECT_COMPONENT: {}",
                component.id
            );
        }
        for file in &record.files {
            ensure!(
                !existing
                    .files
                    .iter()
                    .any(|item| item.path.to_lowercase() == file.path.to_lowercase()),
                "DUPLICATE_OBJECT_FILE: {}",
                file.path
            );
        }
    }
    Ok(())
}

fn validate_parent(connection: &Connection, record: &ObjectRecord) -> Result<()> {
    let mut next = record.parent_object_id.clone();
    let mut visited = std::collections::HashSet::new();
    while let Some(parent_id) = next {
        ensure!(visited.insert(parent_id.clone()), "OBJECT_PARENT_CYCLE");
        ensure!(parent_id != record.id, "SELF_OBJECT_PARENT");
        let parent = read(connection, &parent_id)?.context("OBJECT_PARENT_NOT_FOUND")?;
        ensure!(
            parent.project_id == record.project_id,
            "CROSS_PROJECT_OBJECT_PARENT"
        );
        next = parent.parent_object_id;
    }
    Ok(())
}

pub(crate) fn validate_write(connection: &Connection, record: &ObjectRecord) -> Result<()> {
    validate(record)?;
    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM entities WHERE kind='project' AND id=?",
            [record.project_id.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        project_exists.is_some(),
        "UNKNOWN_OBJECT_PROJECT: {}",
        record.project_id
    );
    duplicate_ownership(connection, record)?;
    validate_parent(connection, record)?;
    for reference in &record.references {
        let target = read(connection, &reference.object_id)?.context("OBJECT_REFERENCE_MISSING")?;
        ensure!(
            target.project_id == record.project_id,
            "CROSS_PROJECT_OBJECT_REFERENCE"
        );
        let mut versions = target
            .versions
            .iter()
            .filter(|version| Some(&version.version_id) == reference.version_id.as_ref());
        let version = versions
            .next()
            .context("OBJECT_REFERENCE_VERSION_MISSING")?;
        ensure!(versions.next().is_none(), "DUPLICATE_OBJECT_VERSION");
        object_version_manifest::read(&target, version)?;
        let saved: Option<String> = connection
            .query_row(
                "SELECT value FROM entities WHERE kind=? AND id=?",
                params![VERSION_KIND, version.version_id],
                |row| row.get(0),
            )
            .optional()?;
        let saved: (String, ObjectVersion) =
            serde_json::from_str(&saved.context("OBJECT_REFERENCE_VERSION_MISSING")?)?;
        ensure!(
            saved == (target.id.clone(), version.clone()),
            "OBJECT_REFERENCE_VERSION_MISMATCH"
        );
    }
    Ok(())
}

pub(crate) fn insert(connection: &Connection, record: &ObjectRecord) -> Result<()> {
    ensure!(
        record.versions.is_empty() && record.revision == 0,
        "OBJECT_REGISTRATION_CANNOT_SUPPLY_VERSIONS"
    );
    validate_write(connection, record)?;
    connection.execute(
        "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
        params![OBJECT_KIND, record.id, serde_json::to_string(record)?],
    )?;
    Ok(())
}

/// Register metadata only. Version capture and future acceptance own version creation.
pub fn register(store: &mut crate::store::Store, record: &ObjectRecord) -> Result<()> {
    store.transaction(|connection| insert(connection, record))
}

pub(crate) fn update_projection(connection: &Connection, record: &ObjectRecord) -> Result<()> {
    let changed = connection.execute(
        "UPDATE entities SET value=? WHERE kind='object' AND id=?",
        params![serde_json::to_string(record)?, record.id],
    )?;
    ensure!(changed == 1, "OBJECT_NOT_FOUND");
    Ok(())
}

pub(crate) fn read(connection: &Connection, object_id: &str) -> Result<Option<ObjectRecord>> {
    id(object_id, "object id")?;
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![OBJECT_KIND, object_id],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| {
            let record: ObjectRecord = serde_json::from_str(&value)?;
            ensure!(record.id == object_id, "OBJECT_IDENTITY_MISMATCH");
            Ok(record)
        })
        .transpose()
}

pub fn get(store: &crate::store::Store, object_id: &str) -> Result<Option<ObjectRecord>> {
    read(&store.connection, object_id)
}

pub fn list(store: &crate::store::Store, project_id: &str) -> Result<Vec<ObjectRecord>> {
    id(project_id, "project id")?;
    Ok(store
        .list::<ObjectRecord>(OBJECT_KIND)?
        .into_iter()
        .filter(|item| item.project_id == project_id)
        .collect())
}

/// Query the project-owned catalog without exposing objects from another
/// project. An empty query is equivalent to `list`.
pub fn search(
    store: &crate::store::Store,
    project_id: &str,
    query: Option<&str>,
) -> Result<Vec<ObjectRecord>> {
    let objects = list(store, project_id)?;
    let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(objects);
    };
    let query = query.to_ascii_lowercase();
    Ok(objects
        .into_iter()
        .filter(|object| {
            object.name.to_ascii_lowercase().contains(&query)
                || object.id.to_ascii_lowercase().contains(&query)
                || object.components.iter().any(|component| {
                    component.name.to_ascii_lowercase().contains(&query)
                        || component.kind.to_ascii_lowercase().contains(&query)
                })
                || object
                    .files
                    .iter()
                    .any(|file| file.path.to_ascii_lowercase().contains(&query))
                || object.category.to_ascii_lowercase().contains(&query)
                || object
                    .tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains(&query))
                || object
                    .thumbnail_path
                    .as_deref()
                    .is_some_and(|path| path.to_ascii_lowercase().contains(&query))
                || object
                    .parent_object_id
                    .as_deref()
                    .is_some_and(|parent| parent.to_ascii_lowercase().contains(&query))
        })
        .collect())
}

#[cfg(test)]
#[path = "object_metadata_tests.rs"]
mod metadata_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, crate::store::Store, ObjectRecord) {
        let temp = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(temp.path()).unwrap();
        store
            .put("project", "project-1", &json!({"id":"project-1"}))
            .unwrap();
        let record = ObjectRecord {
            id: "object-1".into(),
            project_id: "project-1".into(),
            name: "Empty object".into(),
            category: default_category(),
            tags: vec![],
            thumbnail_path: None,
            parent_object_id: None,
            revision: 0,
            components: vec![],
            files: vec![],
            references: vec![],
            versions: vec![],
        };
        (temp, store, record)
    }

    #[test]
    fn registers_object_without_content_or_task() -> Result<()> {
        let (_temp, mut store, record) = fixture();
        register(&mut store, &record)?;
        assert_eq!(get(&store, "object-1")?, Some(record));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_component_and_file_ownership() -> Result<()> {
        let (_temp, mut store, mut first) = fixture();
        first.components.push(ObjectComponent {
            id: "mesh".into(),
            kind: "mesh".into(),
            name: "Mesh".into(),
        });
        first.files.push(ObjectFile {
            path: "art/a.glb".into(),
            role: "source".into(),
        });
        register(&mut store, &first)?;
        let second = ObjectRecord {
            id: "object-2".into(),
            components: first.components.clone(),
            files: first.files.clone(),
            ..fixture().2
        };
        assert!(register(&mut store, &second).is_err());
        Ok(())
    }

    #[test]
    fn rejects_cross_project_reference_and_rolls_back() -> Result<()> {
        let (_temp, mut store, mut record) = fixture();
        record.references.push(ObjectReference {
            project_id: "other".into(),
            object_id: "foreign".into(),
            version_id: None,
        });
        assert!(register(&mut store, &record).is_err());
        assert!(get(&store, "object-1")?.is_none());
        Ok(())
    }

    #[test]
    fn registration_cannot_inject_version_manifests() -> Result<()> {
        let (_temp, mut store, mut record) = fixture();
        record.versions.push(ObjectVersion {
            version_id: "v1".into(),
            manifest: json!({"hash":"a"}),
        });
        assert!(register(&mut store, &record).is_err());
        assert!(get(&store, &record.id)?.is_none());
        assert!(store.list::<Value>(VERSION_KIND)?.is_empty());
        Ok(())
    }

    #[test]
    fn search_is_project_scoped_and_matches_catalog_metadata() -> Result<()> {
        let (_temp, mut store, mut record) = fixture();
        record.name = "Hero prop".into();
        record.files.push(ObjectFile {
            path: "art/hero.glb".into(),
            role: "source".into(),
        });
        register(&mut store, &record)?;
        assert_eq!(search(&store, "project-1", Some("hero"))?.len(), 1);
        assert!(search(&store, "other", Some("hero"))?.is_empty());
        Ok(())
    }
}
