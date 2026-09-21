//! Project-owned object registration and immutable version manifests.
//!
//! This is the storage boundary for F2.1.  It deliberately stores only
//! registration metadata; execution, preview and publication remain owned by
//! their existing domains.
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
    pub components: Vec<ObjectComponent>,
    pub files: Vec<ObjectFile>,
    pub references: Vec<ObjectReference>,
    pub versions: Vec<ObjectVersion>,
}

fn id(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b)),
        "INVALID_OBJECT_CATALOG: invalid {label}"
    );
    Ok(())
}

fn validate(record: &ObjectRecord) -> Result<()> {
    id(&record.id, "object id")?;
    id(&record.project_id, "project id")?;
    ensure!(
        !record.name.trim().is_empty(),
        "INVALID_OBJECT_CATALOG: empty name"
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
            !component.kind.trim().is_empty() && !component.name.trim().is_empty(),
            "INVALID_OBJECT_CATALOG: component fields"
        );
    }
    let mut file_paths = std::collections::HashSet::new();
    for file in &record.files {
        ensure!(
            !file.path.is_empty() && !file.path.starts_with('/') && !file.path.contains(".."),
            "INVALID_OBJECT_CATALOG: invalid file path"
        );
        ensure!(
            file_paths.insert(&file.path),
            "DUPLICATE_OBJECT_FILE: {}",
            file.path
        );
        ensure!(
            !file.role.trim().is_empty(),
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
    for reference in &record.references {
        ensure!(
            reference.project_id == record.project_id,
            "CROSS_PROJECT_OBJECT_REFERENCE"
        );
        id(&reference.project_id, "reference project id")?;
        id(&reference.object_id, "reference object id")?;
        if let Some(version_id) = &reference.version_id {
            id(version_id, "reference version id")?;
        }
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
                !existing.files.iter().any(|item| item.path == file.path),
                "DUPLICATE_OBJECT_FILE: {}",
                file.path
            );
        }
    }
    Ok(())
}

/// Register an object in its project Store. Empty components, files and
/// versions are valid, allowing an object to exist before manufacturing.
pub fn register(store: &mut crate::store::Store, record: &ObjectRecord) -> Result<()> {
    validate(record)?;
    store.transaction(|connection| {
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
        let json = serde_json::to_string(record)?;
        connection.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
            params![OBJECT_KIND, record.id, json],
        )?;
        for version in &record.versions {
            let version_json = serde_json::to_string(&(record.id.clone(), version))?;
            connection.execute(
                "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
                params![VERSION_KIND, version.version_id, version_json],
            )?;
        }
        Ok(())
    })
}

pub fn get(store: &crate::store::Store, object_id: &str) -> Result<Option<ObjectRecord>> {
    id(object_id, "object id")?;
    store.get(OBJECT_KIND, object_id)
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
        })
        .collect())
}

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
    fn version_manifest_is_not_overwritten_by_registration() -> Result<()> {
        let (_temp, mut store, mut record) = fixture();
        record.versions.push(ObjectVersion {
            version_id: "v1".into(),
            manifest: json!({"hash":"a"}),
        });
        register(&mut store, &record)?;
        assert_eq!(
            store.list::<(String, ObjectVersion)>(VERSION_KIND)?[0]
                .1
                .manifest,
            json!({"hash":"a"})
        );
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
