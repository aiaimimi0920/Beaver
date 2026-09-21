//! Identity-only conversion of validation records. Does not write or activate a project.
use crate::{
    project_derivation_identity::{IdentityMap, Key, Target},
    validation::{model::Release, release, repository},
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

pub(crate) struct Rewrite<'a>(pub(crate) &'a IdentityMap);

impl Rewrite<'_> {
    pub(crate) fn key(&self, kind: &str, id: &str) -> Result<Key> {
        let mut entries = self
            .0
            .entities
            .iter()
            .filter(|entry| entry.source.kind == kind && entry.source.id == id);
        let entry = entries.next().context("derivation mapping missing")?;
        ensure!(entries.next().is_none(), "ambiguous derivation mapping");
        match &entry.target {
            Target::Remap { key } => Ok(key.clone()),
            Target::Archive => bail!("archived validation receipt cannot become replay state"),
        }
    }

    pub(crate) fn one(&self, value: &mut Value, field: &str, kind: &str) -> Result<()> {
        let Some(slot) = value.pointer_mut(field).filter(|v| !v.is_null()) else {
            return Ok(());
        };
        let id = slot.as_str().context("invalid derivation reference")?;
        *slot = Value::String(self.key(kind, id)?.id);
        Ok(())
    }

    pub(crate) fn many(&self, value: &mut Value, field: &str, kind: &str) -> Result<()> {
        let Some(items) = value.pointer_mut(field) else {
            return Ok(());
        };
        for item in items.as_array_mut().context("invalid reference array")? {
            let id = item.as_str().context("invalid reference array item")?;
            *item = Value::String(self.key(kind, id)?.id);
        }
        Ok(())
    }

    fn flow(&self, value: &mut Value) -> Result<()> {
        self.one(value, "/id", "validationFlow")?;
        self.one(value, "/projectId", "project")?;
        self.many(value, "/definition/taskIds", "task")
    }

    fn records(&self, value: &mut Value, field: &str, fields: &[(&str, &str)]) -> Result<()> {
        let Some(items) = value.pointer_mut(field) else {
            return Ok(());
        };
        for item in items.as_array_mut().context("invalid nested records")? {
            for (path, kind) in fields {
                self.one(item, path, kind)?;
            }
        }
        Ok(())
    }
}

/// Converts a record in memory using a verified preparation's map. The caller must still
/// rewrite files, interrupt imported work, validate the complete database and keep it pending.
/// Original request receipts belong in provenance storage, never the target replay table.
pub fn rewrite(map: &IdentityMap, kind: &str, id: &str, original: &Value) -> Result<(Key, Value)> {
    ensure!(
        map.format == "beaver-project-derivation-identities-v1",
        "unsupported identity map"
    );
    let rewrite = Rewrite(map);
    let target = rewrite.key(kind, id)?;
    let mut value = original.clone();
    ensure!(value.is_object(), "invalid validation record");
    match kind {
        "validationSettings" | "validationManifest" => {}
        "validationCoverage" => ensure!(
            value["id"] == id && value["taskId"] == id,
            "coverage key mismatch"
        ),
        "validationFlowRevision" => ensure!(
            id == format!(
                "{}:{}",
                value["id"].as_str().context("flow ID missing")?,
                value["revision"]
                    .as_u64()
                    .context("flow revision missing")?
            ),
            "flow revision key mismatch"
        ),
        _ => ensure!(value["id"] == id, "validation identity mismatch"),
    }
    let flow = matches!(kind, "validationFlow" | "validationFlowRevision");
    if flow {
        rewrite.flow(&mut value)?;
        return Ok((target, value));
    }
    rewrite.one(&mut value, "/projectId", "project")?;
    match kind {
        "validationSettings" | "validationManifest" => {}
        "validationCoverage" => {
            rewrite.one(&mut value, "/id", "task")?;
            rewrite.one(&mut value, "/taskId", "task")?;
            rewrite.one(&mut value, "/codeRunId", "validationRun")?;
            rewrite.many(&mut value, "/flowIds", "validationFlow")?;
            rewrite.many(&mut value, "/runIds", "validationRun")?;
            // This cache includes project flows. Force the normal watcher to recompute it.
            value.as_object_mut().unwrap().remove("watchSignature");
        }
        "validationRun" => {
            rewrite.one(&mut value, "/id", kind)?;
            for (path, target) in [
                ("/taskId", "task"),
                ("/releaseId", "validationRelease"),
                ("/baselineId", "validationBaseline"),
            ] {
                rewrite.one(&mut value, path, target)?;
            }
            if let Some(flow) = value.get_mut("flow").filter(|v| !v.is_null()) {
                rewrite.flow(flow)?;
            }
            for field in ["/judgments", "/confirmations"] {
                rewrite.records(
                    &mut value,
                    field,
                    &[
                        ("/runId", "validationRun"),
                        ("/baselineId", "validationBaseline"),
                    ],
                )?;
            }
            ensure!(
                value["snapshotId"] == repository::digest(&value["snapshot"])?,
                "run snapshot digest mismatch"
            );
        }
        "validationBaseline" => {
            rewrite.one(&mut value, "/id", kind)?;
            for (path, target) in [
                ("/flowId", "validationFlow"),
                ("/runId", "validationRun"),
                ("/previousId", "validationBaseline"),
            ] {
                rewrite.one(&mut value, path, target)?;
            }
            // Definition signatures exclude identity/task IDs; content snapshots do not change.
        }
        "validationFeedback" => {
            rewrite.one(&mut value, "/id", kind)?;
            for (path, target) in [
                ("/taskId", "task"),
                ("/runId", "validationRun"),
                ("/rerunId", "validationRun"),
                ("/flowId", "validationFlow"),
            ] {
                rewrite.one(&mut value, path, target)?;
            }
        }
        "validationRepairDecision" => {
            rewrite.one(&mut value, "/id", "validationRun")?;
            rewrite.one(&mut value, "/decision/result/taskId", "task")?;
            rewrite.one(
                &mut value,
                "/decision/result/feedbackId",
                "validationFeedback",
            )?;
        }
        "validationRelease" => {
            let source: Release = serde_json::from_value(original.clone())?;
            ensure!(
                source.scope_id == release::scope(&source)?
                    && source.snapshot_id == repository::digest(&source.snapshot)?,
                "source release integrity mismatch"
            );
            rewrite.one(&mut value, "/id", kind)?;
            rewrite.many(&mut value, "/flowIds", "validationFlow")?;
            rewrite.many(&mut value, "/runIds", "validationRun")?;
            for flow in value["flows"]
                .as_array_mut()
                .context("release flows missing")?
            {
                rewrite.flow(flow)?;
            }
            rewrite.records(&mut value, "/excluded", &[("/flowId", "validationFlow")])?;
            let converted: Release = serde_json::from_value(value.clone())?;
            value["scopeId"] = Value::String(release::scope(&converted)?);
        }
        _ => bail!("unsupported validation derivation record: {kind}"),
    }
    Ok((target, value))
}

#[cfg(test)]
#[path = "project_derivation_validation_records_tests.rs"]
mod tests;
