//! Pure partition plan over an offline inventory; flagged history is retained, never guessed.
use crate::{
    project_migration_files::FileOwner,
    project_migration_inventory::{Destination, Inventory},
    project_migration_ownership::Entity,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Retained {
    pub table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FileMove {
    /// Relative to the legacy data directory, exactly as archived.
    pub source: String,
    /// Relative to the project root, in the layout `Files` resolves for project storage.
    pub target: String,
    pub bytes: u64,
    pub sha256: Option<String>,
}

#[derive(Default)]
pub(crate) struct ProjectPlan {
    pub entities: Vec<(String, String)>,
    pub events: Vec<i64>,
    pub calls: Vec<i64>,
    pub files: Vec<FileMove>,
}

#[derive(Default)]
pub(crate) struct Plan {
    pub projects: BTreeMap<String, ProjectPlan>,
    pub retained: Vec<Retained>,
    /// Legacy-relative files that stay in the host copy because their owner is retained.
    pub retained_files: Vec<String>,
}

/// Legacy data-relative location to the project-relative location used by `Files`.
pub(crate) fn relocate(relative: &str) -> Option<String> {
    let (top, rest) = relative.split_once('/')?;
    let (id, tail) = match rest.split_once('/') {
        Some((id, tail)) => (id, Some(tail)),
        None => (rest, None),
    };
    // Project storage keeps Codex HOMEs under `workspaces/.codex`; a legacy task cannot claim it.
    if id.is_empty() || id == ".codex" {
        return None;
    }
    let prefix = match top {
        "workspaces" => format!(".beaver/workspaces/{id}"),
        "codex" => format!(".beaver/workspaces/.codex/{id}"),
        "asset-observer" => format!(".beaver/evidence/asset-observer/{id}"),
        "validation" => format!(".beaver/evidence/{id}"),
        "blobs" if tail.is_none() => format!(".beaver/content/blobs/{id}"),
        _ => return None,
    };
    Some(match tail {
        Some(tail) => format!("{prefix}/{tail}"),
        None => prefix,
    })
}

/// The task whose whole history must move together with this entity.
pub(crate) fn task_of(entity: &Entity) -> Option<String> {
    match entity.kind.as_str() {
        "task"
        | "asset-task"
        | "framework-configuration"
        | "validationCoverage"
        | "task-callback-revision" => Some(entity.id.clone()),
        "operation" | "asset-reference" | "framework-operation" | "feature" => entity
            .value
            .as_ref()?
            .get("taskId")?
            .as_str()
            .map(str::to_owned),
        kind => kind.split_once('/').map(|(_, task)| task.to_owned()),
    }
}

fn session_task(source_id: &str) -> Option<String> {
    let relative = source_id.strip_prefix("application/data/codex/")?;
    Some(relative.split('/').next()?.to_owned())
}

struct Flags {
    entities: BTreeMap<(String, String), String>,
    tasks: BTreeMap<String, String>,
    runs: BTreeSet<String>,
}

impl Flags {
    fn new(inventory: &Inventory, entities: &[Entity]) -> Self {
        let mut flags = Self {
            entities: BTreeMap::new(),
            tasks: BTreeMap::new(),
            runs: BTreeSet::new(),
        };
        let issues = inventory
            .entity_references
            .issues
            .iter()
            .chain(&inventory.content_references.checks.issues)
            .chain(&inventory.file_references.issues);
        for issue in issues {
            flags.flag(&issue.source_kind, &issue.source_id, issue.reason);
        }
        for issue in &inventory.session_indexes.checks.issues {
            if let Some(task) = session_task(&issue.source_id) {
                flags
                    .tasks
                    .insert(task, format!("SESSION_INDEX:{}", issue.reason));
            }
        }
        for record in &inventory.records {
            if let Destination::Unresolved { reason } = &record.destination {
                if record.table == "entities" {
                    let kind = record.kind.clone().unwrap_or_default();
                    flags.flag(&kind, &record.id, reason);
                }
            }
        }
        for entity in entities {
            if let Some(reason) = flags
                .entities
                .get(&(entity.kind.clone(), entity.id.clone()))
            {
                if let Some(task) = task_of(entity) {
                    flags.tasks.entry(task).or_insert_with(|| reason.clone());
                }
                if entity.kind == "validationRun" {
                    flags.runs.insert(entity.id.clone());
                }
            }
        }
        flags
    }

    fn flag(&mut self, kind: &str, id: &str, reason: &str) {
        self.entities
            .entry((kind.to_owned(), id.to_owned()))
            .or_insert_with(|| reason.to_owned());
    }

    fn entity(&self, entity: &Entity) -> Option<String> {
        if let Some(reason) = self.entities.get(&(entity.kind.clone(), entity.id.clone())) {
            return Some(reason.clone());
        }
        let task = task_of(entity)?;
        self.tasks.get(&task).map(|_| "TASK_RETAINED".to_owned())
    }

    fn file(&self, category: &str, relative: &str) -> Option<String> {
        let owner = relative.split('/').nth(1)?;
        match category {
            "workspace" | "session" | "observation" => self
                .tasks
                .contains_key(owner)
                .then(|| "TASK_RETAINED".to_owned()),
            "validation" => self.runs.contains(owner).then(|| "RUN_RETAINED".to_owned()),
            _ => None,
        }
    }
}

pub(crate) fn build(
    inventory: &Inventory,
    entities: &[Entity],
    events: &[(i64, String)],
    calls: &[(i64, Option<String>)],
) -> Result<Plan> {
    let flags = Flags::new(inventory, entities);
    let by_key: BTreeMap<_, _> = entities
        .iter()
        .map(|entity| ((entity.kind.as_str(), entity.id.as_str()), entity))
        .collect();
    let event_tasks: BTreeMap<_, _> = events.iter().map(|(seq, task)| (*seq, task)).collect();
    let call_tasks: BTreeMap<_, _> = calls.iter().map(|(seq, task)| (*seq, task)).collect();
    let mut plan = Plan::default();
    for id in inventory.projects.keys() {
        plan.projects.entry(id.clone()).or_default();
    }
    for record in &inventory.records {
        let project = match &record.destination {
            Destination::Host => continue,
            Destination::Project { project_id } => project_id,
            Destination::Unresolved { reason } => {
                plan.retain(record.table, record.kind.clone(), &record.id, reason);
                continue;
            }
        };
        let retained = match record.table {
            "entities" => {
                let kind = record.kind.as_deref().unwrap_or_default();
                if kind == "project"
                    && flags
                        .entities
                        .contains_key(&(kind.into(), record.id.clone()))
                {
                    bail!("project entity {} cannot be partitioned", record.id);
                }
                by_key
                    .get(&(kind, record.id.as_str()))
                    .and_then(|e| flags.entity(e))
            }
            "events" => event_tasks
                .get(&record.id.parse::<i64>()?)
                .and_then(|task| flags.tasks.get(*task))
                .map(|_| "TASK_RETAINED".to_owned()),
            "calls" => call_tasks
                .get(&record.id.parse::<i64>()?)
                .and_then(|task| task.as_ref())
                .and_then(|task| flags.tasks.get(task))
                .map(|_| "TASK_RETAINED".to_owned()),
            _ => None,
        };
        if let Some(reason) = retained {
            plan.retain(record.table, record.kind.clone(), &record.id, &reason);
            continue;
        }
        let target = plan.projects.entry(project.clone()).or_default();
        match record.table {
            "entities" => target
                .entities
                .push((record.kind.clone().unwrap_or_default(), record.id.clone())),
            "events" => target.events.push(record.id.parse()?),
            _ => target.calls.push(record.id.parse()?),
        }
    }
    for file in &inventory.files.records {
        let Some(relative) = file.archive_path.strip_prefix("application/data/") else {
            continue;
        };
        let owners: Vec<String> = match &file.owner {
            FileOwner::Container => continue,
            FileOwner::Project { project_id } => vec![project_id.clone()],
            FileOwner::Shared { project_ids } => project_ids.iter().cloned().collect(),
            FileOwner::Unresolved { .. } => {
                plan.retained_files.push(relative.to_owned());
                continue;
            }
        };
        if flags.file(file.category, relative).is_some() {
            plan.retained_files.push(relative.to_owned());
            continue;
        }
        let Some(target) = relocate(relative) else {
            plan.retained_files.push(relative.to_owned());
            continue;
        };
        for owner in owners {
            plan.projects
                .entry(owner)
                .or_default()
                .files
                .push(FileMove {
                    source: relative.to_owned(),
                    target: target.clone(),
                    bytes: file.bytes,
                    sha256: file.sha256.clone(),
                });
        }
    }
    Ok(plan)
}

impl Plan {
    fn retain(&mut self, table: &'static str, kind: Option<String>, id: &str, reason: &str) {
        self.retained.push(Retained {
            table: table.to_owned(),
            kind,
            id: id.to_owned(),
            reason: reason.to_owned(),
        });
    }
}
