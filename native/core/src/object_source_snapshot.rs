//! One committed catalog format for managed and offline Beaver import sources.
use crate::{object_catalog::ObjectRecord, object_import_snapshot};
use anyhow::{ensure, Context, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

pub use object_import_snapshot::ImportVersion;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSourceSnapshot {
    pub project: Value,
    pub objects: Vec<ObjectRecord>,
    pub import_versions: Vec<ImportVersion>,
}

impl ObjectSourceSnapshot {
    /// The caller supplies a read transaction or an isolated offline database copy.
    pub(crate) fn read(connection: &Connection, project_id: &str) -> Result<Self> {
        let project: Option<String> = connection
            .query_row(
                "SELECT value FROM entities WHERE kind='project' AND id=?",
                [project_id],
                |row| row.get(0),
            )
            .optional()?;
        let project: Value =
            serde_json::from_str(&project.context("IMPORT_SOURCE_PROJECT_MISSING")?)?;
        ensure!(
            project["id"] == project_id,
            "IMPORT_SOURCE_PROJECT_ID_MISMATCH"
        );
        let mut statement =
            connection.prepare("SELECT id,value FROM entities WHERE kind='object' ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let objects = rows
            .map(|row| {
                let (key, json) = row?;
                let object: ObjectRecord = serde_json::from_str(&json)?;
                ensure!(
                    object.id == key && object.project_id == project_id,
                    "IMPORT_SOURCE_OBJECT_ID_MISMATCH: {key}"
                );
                Ok(object)
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            project,
            objects,
            import_versions: vec![],
        })
    }

    pub(crate) fn inspect(mut self, project_id: &str, query: Option<&str>) -> Self {
        // Resolve dependencies before filtering the displayed catalog.
        self.import_versions = object_import_snapshot::inspect(&self.objects, project_id);
        let query = query
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        if let Some(query) = query {
            self.objects.retain(|object| {
                object.id.to_ascii_lowercase().contains(&query)
                    || object.name.to_ascii_lowercase().contains(&query)
                    || object.components.iter().any(|component| {
                        component.name.to_ascii_lowercase().contains(&query)
                            || component.kind.to_ascii_lowercase().contains(&query)
                    })
                    || object
                        .files
                        .iter()
                        .any(|file| file.path.to_ascii_lowercase().contains(&query))
            });
        }
        self.import_versions.retain(|version| {
            self.objects
                .iter()
                .any(|object| object.id == version.object_id)
        });
        self
    }
}
