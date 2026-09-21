//! Versioned identity boundary. Execution stays closed until the object queue owns dispatch.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: u32 = 1;
pub const MARKER: &str = "objectFramework";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "basePolicy", rename_all = "camelCase", deny_unknown_fields)]
pub enum Baseline {
    LatestAccepted {},
    PinnedVersion {
        #[serde(rename = "selectedVersionId")]
        selected_version_id: String,
    },
    Empty {},
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "layer", rename_all = "camelCase", deny_unknown_fields)]
pub enum Identity {
    Coarse {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
    },
    Medium {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        #[serde(rename = "objectId")]
        object_id: String,
        baseline: Baseline,
    },
    Fine {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        #[serde(rename = "objectId")]
        object_id: String,
        #[serde(rename = "mediumTaskId")]
        medium_task_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "stageId")]
        stage_id: String,
    },
}

fn id(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b)),
        "INVALID_OBJECT_FRAMEWORK: invalid identity"
    );
    Ok(())
}

/// Presence is checked separately from validity: null and unknown versions never mean legacy.
pub fn marked(value: &Value) -> bool {
    value.get(MARKER).is_some()
}

pub fn identity(task: &Value) -> Result<Option<Identity>> {
    let Some(marker) = task.get(MARKER) else {
        return Ok(None);
    };
    let identity: Identity = serde_json::from_value(marker.clone())
        .context("INVALID_OBJECT_FRAMEWORK: invalid task marker")?;
    let version = match &identity {
        Identity::Coarse { schema_version } => *schema_version,
        Identity::Medium {
            schema_version,
            object_id,
            baseline,
        } => {
            id(object_id)?;
            if let Baseline::PinnedVersion {
                selected_version_id,
            } = baseline
            {
                id(selected_version_id)?;
            }
            *schema_version
        }
        Identity::Fine {
            schema_version,
            object_id,
            medium_task_id,
            run_id,
            stage_id,
        } => {
            for value in [object_id, medium_task_id, run_id, stage_id] {
                id(value)?;
            }
            *schema_version
        }
    };
    ensure!(
        version == VERSION,
        "INVALID_OBJECT_FRAMEWORK: unsupported schema version {version}"
    );
    Ok(Some(identity))
}

pub fn require_legacy(task: &Value) -> Result<()> {
    ensure!(
        identity(task)?.is_none(),
        "OBJECT_FRAMEWORK_DISABLED: 对象队列和制造门槛尚未接通，不能使用旧任务执行流程"
    );
    Ok(())
}
