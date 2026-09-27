//! Project-local idempotency for catalog mutations; committed with the object projection.
use crate::{
    object_catalog::ObjectRecord, object_version_manifest::valid_id,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const KIND: &str = "object_command_receipt";
pub(crate) const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[path = "object_acceptance_history.rs"]
mod history;
pub(crate) use history::latest_accepted;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResult {
    pub request_id: String,
    pub project_id: String,
    pub object: ObjectRecord,
    pub version_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    input_digest: String,
    result: CommandResult,
}

pub(crate) struct Command {
    key: String,
    input_digest: String,
    request_id: String,
    project_id: String,
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub(crate) fn verify_acceptance(
    connection: &Connection,
    request: &crate::object_version_acceptance::AcceptanceRequest,
    object: &ObjectRecord,
) -> Result<()> {
    let receipt: Receipt = crate::object_task_storage::read(
        connection,
        KIND,
        &digest(&(&request.project_id, &request.request_id))?,
    )?
    .ok_or_else(|| anyhow::anyhow!("OBJECT_ACCEPTANCE_RECEIPT_MISSING"))?;
    ensure!(
        receipt.input_digest == digest(&("accept", request))?
            && receipt.result
                == (CommandResult {
                    request_id: request.request_id.clone(),
                    project_id: request.project_id.clone(),
                    object: object.clone(),
                    version_id: Some(request.version_id.clone()),
                }),
        "OBJECT_ACCEPTANCE_HISTORY_INVALID"
    );
    Ok(())
}

impl Command {
    pub fn new(
        runtime: &ProjectRuntime,
        project_id: &str,
        request_id: &str,
        operation: &str,
        input: &impl Serialize,
    ) -> Result<Self> {
        ensure!(
            valid_id(project_id) && valid_id(request_id),
            "INVALID_OBJECT_REQUEST"
        );
        ensure!(
            runtime.project_id() == project_id,
            "OBJECT_PROJECT_MISMATCH"
        );
        Ok(Self {
            key: digest(&(project_id, request_id))?,
            input_digest: digest(&(operation, input))?,
            request_id: request_id.into(),
            project_id: project_id.into(),
        })
    }

    pub fn replay(&self, connection: &Connection) -> Result<Option<CommandResult>> {
        let json: Option<String> = connection
            .query_row(
                "SELECT value FROM entities WHERE kind=? AND id=?",
                params![KIND, self.key],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|json| {
            let receipt: Receipt = serde_json::from_str(&json)?;
            ensure!(
                receipt.input_digest == self.input_digest,
                "OBJECT_REQUEST_CONFLICT"
            );
            ensure!(
                receipt.result.request_id == self.request_id
                    && receipt.result.project_id == self.project_id
                    && receipt.result.object.project_id == self.project_id,
                "OBJECT_RECEIPT_IDENTITY_MISMATCH"
            );
            Ok(receipt.result)
        })
        .transpose()
    }

    pub fn save(
        &self,
        connection: &Connection,
        object: ObjectRecord,
        version_id: Option<String>,
    ) -> Result<CommandResult> {
        let result = CommandResult {
            request_id: self.request_id.clone(),
            project_id: self.project_id.clone(),
            object,
            version_id,
        };
        connection.execute(
            "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
            params![
                KIND,
                self.key,
                serde_json::to_string(&Receipt {
                    input_digest: self.input_digest.clone(),
                    result: result.clone(),
                })?
            ],
        )?;
        Ok(result)
    }
}
