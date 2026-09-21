//! Convert declared paths against the verified source inventory, preserving opaque payloads.
use crate::{
    data_backup::Entry, project_derivation_copy::Prepared, project_derivation_paths::Paths,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;

pub(crate) struct Records<'a>(pub(crate) &'a Prepared);

impl Records<'_> {
    fn entry(&self, path: &str, directory: bool) -> Result<&Entry> {
        Paths(&self.0.identities).relative(path)?;
        let entry = self
            .0
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .context("declared derivation file missing")?;
        ensure!(
            entry.sha256.is_none() == directory,
            "declared file type mismatch"
        );
        Ok(entry)
    }

    fn checkpoint(&self, value: &mut Value, field: &str) -> Result<()> {
        let Some(value) = value.pointer_mut(field).filter(|v| !v.is_null()) else {
            return Ok(());
        };
        let path = value.as_str().context("invalid checkpoint path")?;
        let parts: Vec<_> = path.split('/').collect();
        ensure!(
            parts.len() == 6
                && parts[..3] == [".beaver", "workspaces", ".codex"]
                && parts[4] == "asset-checkpoints"
                && parts[5].ends_with(".blend"),
            "noncanonical checkpoint path"
        );
        self.entry(path, false)?;
        *value = Paths(&self.0.identities).relative(path)?.into();
        Ok(())
    }

    fn reference(&self, value: &mut Value) -> Result<()> {
        if value.is_null() {
            return Ok(());
        }
        let id = value["id"].as_str().context("reference ID missing")?;
        let task = value["taskId"].as_str().context("reference task missing")?;
        ensure!(
            value["projectId"] == self.0.request.source_project_id,
            "reference project mismatch"
        );
        let path = format!(".beaver/evidence/asset-observer/{task}/references/{id}.png");
        ensure!(
            value["imagePath"] == path,
            "noncanonical reference image path"
        );
        let entry = self.entry(&path, false)?;
        ensure!(
            value["sha256"].as_str() == entry.sha256.as_deref(),
            "reference image hash mismatch"
        );
        value["imagePath"] = Paths(&self.0.identities).relative(&path)?.into();
        Ok(())
    }

    fn feedback(&self, value: &mut Value) -> Result<()> {
        ensure!(value.is_object(), "invalid feedback path record");
        self.reference(&mut value["reference"])?;
        self.checkpoint(value, "/checkpoint")
    }

    fn task(&self, value: &mut Value) -> Result<()> {
        if let Some(workspace) = value.get("workspace").filter(|v| !v.is_null()) {
            let id = value["id"].as_str().context("workspace owner missing")?;
            let path = format!(".beaver/workspaces/{id}");
            ensure!(*workspace == path, "noncanonical task workspace");
            self.entry(&path, true)?;
            value["workspace"] = Paths(&self.0.identities).relative(&path)?.into();
        }
        self.checkpoint(value, "/assetRestore")?;
        if let Some(seed) = value.get_mut("assetFeedbackSeed").filter(|v| !v.is_null()) {
            self.feedback(seed)?;
        }
        Ok(())
    }

    pub(crate) fn rewrite(&self, kind: &str, id: &str, value: &mut Value) -> Result<()> {
        match kind {
            "task" => self.task(value)?,
            "operation" => self.task(&mut value["taskAfter"])?,
            "asset-reference" => self.reference(value)?,
            "asset-task" => {
                self.checkpoint(value, "/checkpoint")?;
                if let Some(frame) = value.get_mut("lastFrame") {
                    self.reference(frame)?;
                }
                if let Some(items) = value.get_mut("feedback") {
                    for item in items.as_array_mut().context("invalid feedback list")? {
                        self.feedback(item)?;
                    }
                }
                if let Some(items) = value.pointer_mut("/work/attempts") {
                    for item in items.as_array_mut().context("invalid attempt list")? {
                        self.checkpoint(item, "/checkpoint")?;
                        self.checkpoint(item, "/endCheckpoint")?;
                    }
                }
            }
            "validationRun" => {
                if let Some(items) = value.get("evidence") {
                    for item in items.as_array().context("invalid evidence list")? {
                        let file = item["file"].as_str().context("evidence filename missing")?;
                        ensure!(!file.contains('/'), "invalid evidence filename");
                        let path = format!(".beaver/evidence/{id}/{file}");
                        let entry = self.entry(&path, false)?;
                        ensure!(
                            item["sha256"].as_str() == entry.sha256.as_deref(),
                            "evidence hash mismatch"
                        );
                    }
                }
            }
            "framework-operation" => {
                let operation: crate::framework_operations::Operation =
                    serde_json::from_value(value.clone())?;
                if !matches!(
                    operation.job,
                    crate::framework_contract::Job::Callback { .. }
                ) {
                    self.checkpoint(value, "/result/context/checkpoint")?;
                }
            }
            _ if matches!(
                kind.split_once('/').map(|(prefix, _)| prefix),
                Some("framework-trace" | "framework-observation" | "framework-recovery")
            ) =>
            {
                self.checkpoint(value, "/context/checkpoint")?;
            }
            _ => {}
        }
        Ok(())
    }
}
