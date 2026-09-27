//! Shared import journal: frozen blobs, durable write intent, atomic registration.
use crate::{
    object_catalog, project_runtime::ProjectRuntime, project_storage_router::ProjectStorageRouter,
};
use anyhow::{ensure, Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[path = "object_file_import_files.rs"]
mod files;
#[path = "object_file_import_plan.rs"]
mod plan;
#[path = "object_import_commit_source.rs"]
mod source;
use source::Source;
#[cfg(test)]
#[path = "object_project_import_tests.rs"]
mod project_tests;
#[cfg(test)]
#[path = "object_file_import_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Applying,
    Aborting,
    ImportedPendingValidation,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub project_id: String,
    pub preparation_id: String,
    pub state: State,
    pub object_ids: Vec<String>,
    pub writes: Vec<String>,
    pub error: Option<String>,
}

pub fn get(runtime: &ProjectRuntime, id: &str) -> Result<Option<Operation>> {
    let receipt = Source::load(runtime, id)?;
    let store = runtime.store();
    let store = store
        .lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
    let operation: Option<Operation> = store.get(source::kind(id), id)?;
    if let Some(operation) = &operation {
        let objects = receipt.objects()?;
        let paths: Vec<_> = objects
            .iter()
            .flat_map(|object| object.files.iter().map(|file| &file.path))
            .collect();
        ensure!(
            operation.project_id == runtime.project_id()
                && operation.preparation_id == id
                && operation.object_ids
                    == objects
                        .iter()
                        .map(|object| object.id.clone())
                        .collect::<Vec<_>>()
                && paths.starts_with(&operation.writes.iter().collect::<Vec<_>>())
                && (operation.state != State::ImportedPendingValidation
                    || operation.writes.len() == paths.len()),
            "IMPORT_OPERATION_CORRUPT"
        );
    }
    Ok(operation)
}

fn save(runtime: &ProjectRuntime, operation: &Operation) -> Result<()> {
    runtime
        .store()
        .lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?
        .put(
            source::kind(&operation.preparation_id),
            &operation.preparation_id,
            operation,
        )
}

fn validate_catalog(
    db: &rusqlite::Connection,
    objects: &[object_catalog::ObjectRecord],
) -> Result<()> {
    for object in objects {
        object_catalog::validate(object)?;
        ensure!(
            object_catalog::read(db, &object.id)?.is_none(),
            "IMPORT_OBJECT_ALREADY_EXISTS"
        );
        let mut shell = object.clone();
        shell.references.clear();
        shell.parent_object_id = None;
        object_catalog::validate_write(db, &shell)?;
    }
    Ok(())
}

pub fn execute(runtime: &ProjectRuntime, id: &str) -> Result<Operation> {
    execute_with(runtime, id, &mut |_| Ok(()))
}

pub fn execute_project(
    runtime: &ProjectRuntime,
    router: &ProjectStorageRouter,
    id: &str,
) -> Result<Operation> {
    execute_source(runtime, id, Some(router), &mut |_| Ok(()))
}

fn execute_with(
    runtime: &ProjectRuntime,
    id: &str,
    checkpoint: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<Operation> {
    execute_source(runtime, id, None, checkpoint)
}

fn execute_source(
    runtime: &ProjectRuntime,
    id: &str,
    router: Option<&ProjectStorageRouter>,
    checkpoint: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<Operation> {
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_BUSY"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_BUSY"))?;
    let receipt = Source::load(runtime, id)?;
    let objects = receipt.objects()?;
    let mut operation = match get(runtime, id)? {
        Some(operation) => operation,
        None => {
            {
                let store = runtime.store();
                let store = store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
                validate_catalog(&store.connection, &objects)?;
            }
            receipt.freeze(runtime, router)?;
            let operation = Operation {
                project_id: runtime.project_id().into(),
                preparation_id: id.into(),
                state: State::Applying,
                object_ids: objects.iter().map(|object| object.id.clone()).collect(),
                writes: vec![],
                error: None,
            };
            save(runtime, &operation)?;
            operation
        }
    };
    if operation.state == State::ImportedPendingValidation {
        return Ok(operation);
    }
    ensure!(operation.state == State::Applying, "IMPORT_NOT_APPLYING");
    let outcome = (|| -> Result<()> {
        checkpoint("prepared")?;
        receipt.verify_frozen(runtime)?;
        {
            let store = runtime.store();
            let store = store
                .lock()
                .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
            validate_catalog(&store.connection, &objects)?;
        }
        files::apply(runtime, &objects, &mut operation, checkpoint)?;
        checkpoint("beforeCommit")?;
        let store = runtime.store();
        let mut store = store
            .lock()
            .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
        store.transaction(|db| {
            validate_catalog(db, &objects)?;
            files::verify(runtime, &objects)?;
            receipt.verify_frozen(runtime)?;
            for object in &objects {
                let mut initial = object.clone();
                initial.versions.clear();
                initial.revision = 0;
                initial.references.clear();
                initial.parent_object_id = None;
                object_catalog::insert(db, &initial)?;
                for version in &object.versions {
                    db.execute(
                        "INSERT INTO entities(kind,id,value) VALUES('object_version',?,?)",
                        params![
                            version.version_id,
                            serde_json::to_string(&(&object.id, version))?
                        ],
                    )?;
                }
                object_catalog::update_projection(db, object)?;
            }
            receipt.provenance(db)?;
            let mut completed = operation.clone();
            completed.state = State::ImportedPendingValidation;
            completed.error = None;
            db.execute(
                "UPDATE entities SET value=? WHERE kind=? AND id=?",
                params![serde_json::to_string(&completed)?, source::kind(id), id],
            )?;
            Ok(())
        })?;
        operation.state = State::ImportedPendingValidation;
        operation.error = None;
        Ok(())
    })();
    if let Err(error) = outcome {
        operation.error = Some(error.to_string());
        save(runtime, &operation)?;
    }
    Ok(operation)
}

pub fn abort(runtime: &ProjectRuntime, id: &str) -> Result<Operation> {
    let _recovery = runtime
        .recovery_verification
        .try_lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_BUSY"))?;
    let _materialization = runtime
        .materialization
        .try_lock()
        .map_err(|_| anyhow::anyhow!("IMPORT_BUSY"))?;
    let mut operation = get(runtime, id)?.context("IMPORT_OPERATION_NOT_FOUND")?;
    if operation.state == State::Aborted {
        return Ok(operation);
    }
    ensure!(
        matches!(operation.state, State::Applying | State::Aborting),
        "IMPORT_ALREADY_COMMITTED"
    );
    let objects = Source::load(runtime, id)?.objects()?;
    operation.state = State::Aborting;
    save(runtime, &operation)?;
    let cleanup = (|| {
        // A newly registered owner also blocks deletion, even if its bytes still match.
        let store = runtime.store();
        let store = store
            .lock()
            .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
        validate_catalog(&store.connection, &objects)?;
        files::reverse(runtime, &objects, &operation)
    })();
    match cleanup {
        Ok(()) => {
            operation.state = State::Aborted;
            operation.error = None;
        }
        Err(error) => operation.error = Some(error.to_string()),
    }
    save(runtime, &operation)?;
    Ok(operation)
}
