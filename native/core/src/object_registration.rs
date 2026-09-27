//! Registration and ownership edits. These commands cannot create accepted versions or tasks.
pub use crate::object_command_receipt::CommandResult;
use crate::{
    object_catalog::{self, ObjectComponent, ObjectFile, ObjectRecord, ObjectReference},
    object_command_receipt::{Command, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterRequest {
    pub project_id: String,
    pub request_id: String,
    pub name: String,
    #[serde(default = "object_catalog::default_category")]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub parent_object_id: Option<String>,
    #[serde(default)]
    pub components: Vec<ObjectComponent>,
    #[serde(default)]
    pub files: Vec<ObjectFile>,
    #[serde(default)]
    pub references: Vec<ObjectReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRequest {
    pub project_id: String,
    pub request_id: String,
    pub object_id: String,
    pub expected_revision: u64,
    pub name: String,
    #[serde(default = "object_catalog::default_category")]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub parent_object_id: Option<String>,
    pub components: Vec<ObjectComponent>,
    pub files: Vec<ObjectFile>,
    pub references: Vec<ObjectReference>,
}

pub(crate) fn current(
    connection: &Connection,
    project_id: &str,
    object_id: &str,
    expected_revision: u64,
) -> Result<ObjectRecord> {
    let object = object_catalog::read(connection, object_id)?.context("OBJECT_NOT_FOUND")?;
    ensure!(object.project_id == project_id, "OBJECT_PROJECT_MISMATCH");
    ensure!(expected_revision < MAX_REVISION, "INVALID_OBJECT_REVISION");
    ensure!(
        object.revision == expected_revision,
        "OBJECT_REVISION_CONFLICT"
    );
    Ok(object)
}

pub fn register(runtime: &ProjectRuntime, request: &RegisterRequest) -> Result<CommandResult> {
    let command = Command::new(
        runtime,
        &request.project_id,
        &request.request_id,
        "register",
        request,
    )?;
    let store = runtime.store();
    let mut store = store
        .lock()
        .map_err(|_| anyhow::anyhow!("object store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = command.replay(connection)? {
            return Ok(receipt);
        }
        let object = ObjectRecord {
            id: format!("object-{}", uuid::Uuid::new_v4()),
            project_id: request.project_id.clone(),
            name: request.name.clone(),
            category: request.category.clone(),
            tags: request.tags.clone(),
            thumbnail_path: request.thumbnail_path.clone(),
            parent_object_id: request.parent_object_id.clone(),
            revision: 0,
            components: request.components.clone(),
            files: request.files.clone(),
            references: request.references.clone(),
            versions: vec![],
        };
        object_catalog::insert(connection, &object)?;
        command.save(connection, object, None)
    })
}

pub fn update(runtime: &ProjectRuntime, request: &UpdateRequest) -> Result<CommandResult> {
    let command = Command::new(
        runtime,
        &request.project_id,
        &request.request_id,
        "update",
        request,
    )?;
    let store = runtime.store();
    let mut store = store
        .lock()
        .map_err(|_| anyhow::anyhow!("object store lock poisoned"))?;
    store.transaction(|connection| {
        if let Some(receipt) = command.replay(connection)? {
            return Ok(receipt);
        }
        let mut object = current(
            connection,
            &request.project_id,
            &request.object_id,
            request.expected_revision,
        )?;
        object.name = request.name.clone();
        object.category = request.category.clone();
        object.tags = request.tags.clone();
        object.thumbnail_path = request.thumbnail_path.clone();
        object.parent_object_id = request.parent_object_id.clone();
        object.components = request.components.clone();
        object.files = request.files.clone();
        object.references = request.references.clone();
        object.revision += 1;
        object_catalog::validate_write(connection, &object)?;
        object_catalog::update_projection(connection, &object)?;
        command.save(connection, object, None)
    })
}

#[cfg(test)]
#[path = "object_registration_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "object_ownership_tests.rs"]
mod ownership_tests;
