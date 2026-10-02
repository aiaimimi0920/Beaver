//! Offline identity staging with inventory-checked paths for verified preparations.
//! Runtime recovery, project metadata and activation remain separate stages.
use crate::{
    object_run_recovery::resume::derivation::rework::Prompts,
    project_derivation_asset_records as assets,
    project_derivation_copy::Request,
    project_derivation_framework_records as framework, project_derivation_history,
    project_derivation_identity::{self, IdentityMap, Key, Target},
    project_derivation_task_records as tasks, project_derivation_validation,
    project_derivation_validation_records::{self as validation, Rewrite},
    store_schema,
};
use anyhow::{bail, ensure, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivedRecord {
    pub source: Key,
    pub raw_json: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Archive {
    pub format: String,
    pub request_id: String,
    pub source_project_id: String,
    pub target_project_id: String,
    pub records: Vec<ArchivedRecord>,
}

/// Keep provenance and converted data together until a later durable staging protocol saves both.
pub struct Staged {
    pub connection: Connection,
    pub archive: Archive,
}

fn rewrite(
    db: &Connection,
    plans: &crate::project_derivation_plan_validation::Plans,
    prompts: &Prompts,
    map: &IdentityMap,
    request: &Request,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    if crate::project_derivation_execution_validation::supports(kind) {
        return crate::project_derivation_execution_records::rewrite(
            db, map, request, prompts, kind, id, value,
        );
    }
    if crate::project_derivation_queue_records::supports(kind) {
        return crate::project_derivation_queue_records::rewrite(map, request, kind, id, value);
    }
    if crate::project_derivation_declaration_records::supports(kind) {
        return crate::project_derivation_declaration_records::rewrite(
            plans, map, request, kind, id, value,
        );
    }
    if crate::project_derivation_planning_records::supports(kind) {
        return crate::project_derivation_planning_records::rewrite(map, request, kind, id, value);
    }
    if crate::project_derivation_plan_records::supports(kind) {
        return crate::project_derivation_plan_records::rewrite(
            map, request, prompts, kind, id, value,
        );
    }
    match kind {
        "object" | "object_version" | "object_command_receipt" => {
            crate::project_derivation_object_records::rewrite(map, request, kind, id, value)
        }
        "project" => {
            ensure!(
                value.is_object() && value["id"] == id,
                "project key mismatch"
            );
            let key = Rewrite(map).key(kind, id)?;
            let mut value = value.clone();
            value["id"] = key.id.clone().into();
            Ok((key, value))
        }
        "task" | "operation" | "feature" | "task-callback-revision" => {
            tasks::rewrite(map, kind, id, value)
        }
        "asset-reference" | "asset-task" => assets::rewrite(map, kind, id, value),
        "framework-configuration" | "framework-operation" => {
            framework::rewrite(db, map, kind, id, value)
        }
        "validationSettings"
        | "validationManifest"
        | "validationCoverage"
        | "validationFlow"
        | "validationFlowRevision"
        | "validationRun"
        | "validationBaseline"
        | "validationFeedback"
        | "validationRepairDecision"
        | "validationRelease" => validation::rewrite(map, kind, id, value),
        _ => match kind.split_once('/').map(|(prefix, _)| prefix) {
            Some("task-callback") => tasks::rewrite(map, kind, id, value),
            Some("asset-delivery" | "asset-delivery-decisions") => {
                assets::rewrite(map, kind, id, value)
            }
            Some(
                "framework-trace"
                | "framework-observation"
                | "framework-recovery"
                | "framework-check"
                | "framework-judgment"
                | "framework-judgment-history",
            ) => framework::rewrite(db, map, kind, id, value),
            _ => bail!("unsupported derivation record: {kind}/{id}"),
        },
    }
}

/// Read a quiescent offline snapshot in one transaction; never modify source or publish a partial target.
pub fn compose(source: &Connection, request: &Request, map: &IdentityMap) -> Result<Staged> {
    compose_records(source, request, map, None)
}

pub(crate) fn compose_prepared(
    source: &Connection,
    prepared: &crate::project_derivation_copy::Prepared,
) -> Result<Staged> {
    crate::project_derivation_paths::Paths(&prepared.identities).entries(&prepared.entries)?;
    crate::project_derivation_object_validation::blobs(source, &prepared.entries)?;
    crate::project_derivation_execution_inventory::validate(source, &prepared.entries)?;
    compose_records(
        source,
        &prepared.request,
        &prepared.identities,
        Some(crate::project_derivation_record_paths::Records(prepared)),
    )
}

fn compose_records(
    source: &Connection,
    request: &Request,
    map: &IdentityMap,
    paths: Option<crate::project_derivation_record_paths::Records<'_>>,
) -> Result<Staged> {
    ensure!(
        !request.request_id.trim().is_empty()
            && !request.source_project_id.trim().is_empty()
            && !request.target_project_id.trim().is_empty()
            && request.source_project_id != request.target_project_id,
        "invalid derivation identity request"
    );
    let source = source.unchecked_transaction()?;
    let entities = project_derivation_validation::validate(&source, &request.source_project_id)?;
    ensure!(
        &project_derivation_identity::build_validated(&source, request, &entities)? == map,
        "derivation identity map changed"
    );
    let plans = crate::project_derivation_plan_validation::Plans::from_entities(
        &entities,
        &request.source_project_id,
    )?;
    let prompts = Prompts::read(&source, request)?;
    let mut connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let target = connection.transaction()?;
    let mut archive = Archive {
        format: "beaver-project-derivation-archive-v1".into(),
        request_id: request.request_id.clone(),
        source_project_id: request.source_project_id.clone(),
        target_project_id: request.target_project_id.clone(),
        records: Vec::new(),
    };
    for entry in &map.entities {
        let raw: String = source.query_row(
            "SELECT value FROM entities WHERE kind=? AND id=?",
            params![entry.source.kind, entry.source.id],
            |row| row.get(0),
        )?;
        match &entry.target {
            Target::Archive => {
                ensure!(
                    entry.source.kind == "validationRequest",
                    "unsupported archive kind"
                );
                archive.records.push(ArchivedRecord {
                    source: entry.source.clone(),
                    raw_json: raw,
                });
            }
            Target::Remap { key: expected } => {
                let mut original = serde_json::from_str(&raw)?;
                if let Some(paths) = &paths {
                    paths.rewrite(&entry.source.kind, &entry.source.id, &mut original)?;
                }
                let (key, value) = rewrite(
                    &source,
                    &plans,
                    &prompts,
                    map,
                    request,
                    &entry.source.kind,
                    &entry.source.id,
                    &original,
                )?;
                ensure!(&key == expected, "derived record key mismatch");
                target.execute(
                    "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
                    params![key.kind, key.id, value.to_string()],
                )?;
            }
        }
    }
    project_derivation_history::copy(&source, map, &target)?;
    crate::project_derivation_queue_records::stage_copied(&source, &target, request)?;
    project_derivation_validation::validate(&target, &request.target_project_id)?;
    target.commit()?;
    source.rollback()?;
    Ok(Staged {
        connection,
        archive,
    })
}

#[cfg(test)]
#[path = "project_derivation_database_tests.rs"]
mod tests;
