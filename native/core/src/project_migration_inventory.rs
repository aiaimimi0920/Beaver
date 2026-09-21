//! Read-only database and file inventory from an already complete offline archive.
use crate::{
    data_backup,
    files::safe_path,
    migration_bundle,
    project_migration_content::{self, Content},
    project_migration_file_fields,
    project_migration_files::{self, FileIndex, FileInventory},
    project_migration_ownership::{Entity, Owner, Ownership},
    project_migration_references::{self, Checks},
    project_migration_sessions::{self, Sessions},
};
use anyhow::{ensure, Result};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Default, Serialize)]
pub struct Counts {
    pub entities: usize,
    pub events: usize,
    pub calls: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum Destination {
    Host,
    Project {
        #[serde(rename = "projectId")]
        project_id: String,
    },
    Unresolved {
        reason: String,
    },
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub table: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Entity ID, event sequence, or call sequence; never narrative or secret payload.
    pub id: String,
    #[serde(flatten)]
    pub destination: Destination,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub format: &'static str,
    pub ready_to_activate: bool,
    pub host: Counts,
    pub projects: BTreeMap<String, Counts>,
    pub unresolved: Counts,
    pub records: Vec<Record>,
    pub entity_references: Checks,
    pub content_references: Content,
    pub files: FileInventory,
    pub file_references: Checks,
    pub session_indexes: Sessions,
}

impl Inventory {
    fn add(&mut self, table: &'static str, kind: Option<String>, id: String, owner: Owner) {
        let (destination, counts) = match owner {
            Ok(Some(project_id)) => {
                let counts = self.projects.entry(project_id.clone()).or_default();
                (Destination::Project { project_id }, counts)
            }
            Ok(None) => (Destination::Host, &mut self.host),
            Err(reason) => (
                Destination::Unresolved {
                    reason: reason.into(),
                },
                &mut self.unresolved,
            ),
        };
        match table {
            "entities" => counts.entities += 1,
            "events" => counts.events += 1,
            "calls" => counts.calls += 1,
            _ => unreachable!(),
        }
        self.records.push(Record {
            table,
            kind,
            id,
            destination,
        });
    }
}

pub fn inspect(backup: &Path) -> Result<Inventory> {
    let manifest = migration_bundle::verify(backup)?;
    let application = data_backup::verify(&safe_path(backup, "application")?)?;
    let temp = tempfile::tempdir()?;
    let data = safe_path(backup, "application/data")?;
    // Opening the archive itself read-only could still create or change its SHM.
    for name in ["beaver.sqlite", "beaver.sqlite-wal", "beaver.sqlite-shm"] {
        let source = safe_path(&data, name)?;
        if source.try_exists()? {
            fs::copy(source, temp.path().join(name))?;
        }
    }
    let connection = Connection::open_with_flags(
        temp.path().join("beaver.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    ensure!(
        integrity == "ok",
        "inventory database integrity check failed"
    );
    let report = inspect_database(&connection, &manifest.projects, &application, &data)?;
    migration_bundle::verify(backup)?;
    Ok(report)
}

fn inspect_database(
    connection: &Connection,
    archived_projects: &[migration_bundle::Project],
    application: &data_backup::Manifest,
    data: &Path,
) -> Result<Inventory> {
    let mut statement =
        connection.prepare("SELECT kind,id,value FROM entities ORDER BY kind,id")?;
    let entities = statement
        .query_map([], |row| {
            let raw: String = row.get(2)?;
            Ok(Entity {
                kind: row.get(0)?,
                id: row.get(1)?,
                value: serde_json::from_str(&raw).ok(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let projects = archived_projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let ownership = Ownership::new(projects, &entities);
    let index = FileIndex::new(&application.entries, &entities, &ownership);
    let content = project_migration_content::inspect(&entities, &ownership, &application.entries);
    let files = project_migration_files::inspect(application, archived_projects, &index, &content);
    let file_references =
        project_migration_file_fields::inspect(&entities, &ownership, &application.source, &index);
    let mut report = Inventory {
        format: "beaver-project-inventory-v1",
        ready_to_activate: false,
        host: Counts::default(),
        projects: archived_projects
            .iter()
            .map(|project| (project.id.clone(), Counts::default()))
            .collect(),
        unresolved: Counts::default(),
        records: Vec::new(),
        entity_references: project_migration_references::inspect(&entities, &ownership),
        content_references: content,
        files,
        file_references,
        session_indexes: project_migration_sessions::inspect(
            data,
            application,
            archived_projects,
            &index,
        ),
    };
    for entity in &entities {
        report.add(
            "entities",
            Some(entity.kind.clone()),
            entity.id.clone(),
            ownership.entity(entity),
        );
    }
    // Store::events is a UI history query capped at 500; migration must enumerate all rows.
    let mut statement = connection.prepare("SELECT seq,task FROM events ORDER BY seq")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let sequence: i64 = row.get(0)?;
        let task: String = row.get(1)?;
        report.add("events", None, sequence.to_string(), ownership.task(&task));
    }
    let mut statement =
        connection.prepare("SELECT seq,id,task,project,value FROM calls ORDER BY seq")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let sequence: i64 = row.get(0)?;
        let id: String = row.get(1)?;
        let task: Option<String> = row.get(2)?;
        let project: Option<String> = row.get(3)?;
        let raw: String = row.get(4)?;
        report.add(
            "calls",
            None,
            sequence.to_string(),
            ownership.call(&id, task.as_deref(), project.as_deref(), &raw),
        );
    }
    Ok(report)
}
