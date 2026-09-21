//! Classify every archived entry without reading payloads or inventing a destination.
use crate::{
    data_backup::{Entry, Manifest},
    migration_bundle::Project,
    project_migration_content::Content,
    project_migration_ownership::{Entity, Owner, Ownership},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum FileOwner {
    Container,
    Project {
        #[serde(rename = "projectId")]
        project_id: String,
    },
    Shared {
        #[serde(rename = "projectIds")]
        project_ids: BTreeSet<String>,
    },
    Unresolved {
        reason: &'static str,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    /// Path relative to the bundle, never a path to the live source.
    pub archive_path: String,
    pub bytes: u64,
    pub sha256: Option<String>,
    pub category: &'static str,
    #[serde(flatten)]
    pub owner: FileOwner,
}

#[derive(Debug, Default, Serialize)]
pub struct FileInventory {
    pub unresolved: usize,
    pub records: Vec<FileRecord>,
}

fn destination(owner: Owner) -> FileOwner {
    match owner {
        Ok(Some(project_id)) => FileOwner::Project { project_id },
        Ok(None) => FileOwner::Unresolved {
            reason: "FILE_OWNER_NOT_PROJECT",
        },
        Err(reason) => FileOwner::Unresolved { reason },
    }
}

pub(crate) struct FileIndex<'a> {
    entries: BTreeMap<String, Option<&'a Entry>>,
    runs: BTreeMap<&'a str, Owner>,
    ownership: &'a Ownership,
}

impl<'a> FileIndex<'a> {
    pub fn new(entries: &'a [Entry], entities: &'a [Entity], ownership: &'a Ownership) -> Self {
        let mut indexed = BTreeMap::new();
        for entry in entries {
            indexed
                .entry(entry.path.to_ascii_lowercase())
                .and_modify(|value| *value = None)
                .or_insert(Some(entry));
        }
        let runs = entities
            .iter()
            .filter(|entity| entity.kind == "validationRun")
            .map(|entity| {
                let owner = if entity.value.as_ref().and_then(|v| v["id"].as_str())
                    == Some(entity.id.as_str())
                {
                    ownership.entity(entity)
                } else {
                    Err("ENTITY_ID_MISMATCH")
                };
                (entity.id.as_str(), owner)
            })
            .collect();
        Self {
            entries: indexed,
            runs,
            ownership,
        }
    }

    pub fn entry(&self, path: &str) -> Result<&Entry, &'static str> {
        self.entries
            .get(&path.to_ascii_lowercase())
            .ok_or("FILE_NOT_FOUND")?
            .ok_or("FILE_PATH_AMBIGUOUS")
    }

    pub fn owner(&self, path: &str) -> Owner {
        let entry = self.entry(path)?;
        let mut parts = entry.path.split('/');
        let category = parts.next().unwrap_or_default();
        let id = parts.next().ok_or("FILE_OWNER_UNKNOWN")?;
        match category {
            "workspaces" | "codex" | "asset-observer" => self.ownership.task(id),
            "validation" => self
                .runs
                .get(id)
                .cloned()
                .ok_or("VALIDATION_RUN_NOT_FOUND")?,
            _ => Err("FILE_OWNER_UNKNOWN"),
        }
    }

    fn classify(&self, entry: &Entry, content: &Content) -> (&'static str, FileOwner) {
        let parts: Vec<_> = entry.path.split('/').collect();
        let top = parts[0];
        if parts.len() == 1 {
            if entry.sha256.is_some()
                && ["beaver.sqlite", "beaver.sqlite-wal", "beaver.sqlite-shm"].contains(&top)
            {
                return ("database", FileOwner::Container);
            }
            if entry.sha256.is_none()
                && [
                    "workspaces",
                    "codex",
                    "asset-observer",
                    "validation",
                    "blobs",
                ]
                .contains(&top)
            {
                return ("directory", FileOwner::Container);
            }
        }
        if parts.len() >= 2 {
            let category = match top {
                "workspaces" => Some("workspace"),
                "codex" => Some("session"),
                "asset-observer" => Some("observation"),
                "validation" => Some("validation"),
                _ => None,
            };
            if let Some(category) = category {
                return (category, destination(self.owner(&entry.path)));
            }
            if top == "blobs" && parts.len() == 2 && entry.sha256.as_deref() == Some(parts[1]) {
                if let Some(project_ids) = content.blobs.get(parts[1]) {
                    return (
                        "blob",
                        FileOwner::Shared {
                            project_ids: project_ids.clone(),
                        },
                    );
                }
            }
        }
        (
            "unclassified",
            FileOwner::Unresolved {
                reason: "FILE_OWNER_UNKNOWN",
            },
        )
    }
}

impl FileInventory {
    fn add(&mut self, root: &str, entry: &Entry, category: &'static str, owner: FileOwner) {
        self.unresolved += usize::from(matches!(owner, FileOwner::Unresolved { .. }));
        self.records.push(FileRecord {
            archive_path: format!("{root}/{}", entry.path),
            bytes: entry.bytes,
            sha256: entry.sha256.clone(),
            category,
            owner,
        });
    }
}

pub(crate) fn inspect(
    application: &Manifest,
    projects: &[Project],
    index: &FileIndex<'_>,
    content: &Content,
) -> FileInventory {
    let mut report = FileInventory::default();
    for entry in &application.entries {
        let (category, owner) = index.classify(entry, content);
        report.add("application/data", entry, category, owner);
    }
    for project in projects {
        let root = format!("projects/{}", project.id);
        for entry in &project.entries {
            report.add(
                &root,
                entry,
                "project",
                FileOwner::Project {
                    project_id: project.id.clone(),
                },
            );
        }
    }
    report
}
