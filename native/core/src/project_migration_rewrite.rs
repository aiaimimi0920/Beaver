//! Rewrite only the explicit legacy path fields into project-relative locations.
use crate::{migration_import, project_migration_plan::relocate};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::path::Path;

pub(crate) struct Rewriter<'a> {
    original: &'a Path,
    pub rewritten: usize,
}

fn present<'a>(parent: &'a mut Value, field: &str) -> Option<&'a mut Value> {
    parent.get_mut(field).filter(|value| !value.is_null())
}

impl<'a> Rewriter<'a> {
    pub fn new(original: &'a Path) -> Self {
        Self {
            original,
            rewritten: 0,
        }
    }

    fn path(&mut self, value: &mut Value, field: &str) -> Result<String> {
        let Some(text) = value.as_str() else {
            bail!("{field} is not a stored path");
        };
        let relative = migration_import::relative(self.original, text)?
            .with_context(|| format!("{field} is outside the legacy data directory"))?;
        let target =
            relocate(&relative).with_context(|| format!("{field} has no project location"))?;
        *value = Value::String(target.clone());
        self.rewritten += 1;
        Ok(target)
    }

    fn optional(&mut self, parent: &mut Value, field: &str, prefix: &str) -> Result<()> {
        if let Some(value) = present(parent, field) {
            self.path(value, &format!("{prefix}/{field}"))?;
        }
        Ok(())
    }

    fn feedback(&mut self, value: &mut Value, prefix: &str) -> Result<()> {
        if let Some(reference) = present(value, "reference") {
            self.path(
                &mut reference["imagePath"],
                &format!("{prefix}/reference/imagePath"),
            )?;
        }
        self.optional(value, "checkpoint", prefix)
    }

    fn task(&mut self, task: &mut Value, prefix: &str) -> Result<()> {
        let id = task["id"]
            .as_str()
            .with_context(|| format!("{prefix}/id missing"))?
            .to_owned();
        let workspace = self.path(&mut task["workspace"], &format!("{prefix}/workspace"))?;
        ensure!(
            workspace == format!(".beaver/workspaces/{id}"),
            "{prefix}/workspace does not belong to task {id}"
        );
        self.optional(task, "assetRestore", prefix)?;
        if let Some(seed) = present(task, "assetFeedbackSeed") {
            self.feedback(seed, &format!("{prefix}/assetFeedbackSeed"))?;
        }
        Ok(())
    }

    fn asset(&mut self, value: &mut Value) -> Result<()> {
        self.optional(value, "checkpoint", "")?;
        if let Some(frame) = present(value, "lastFrame") {
            self.path(&mut frame["imagePath"], "/lastFrame/imagePath")?;
        }
        if let Some(items) = value.get_mut("feedback").and_then(Value::as_array_mut) {
            for (index, item) in items.iter_mut().enumerate() {
                self.feedback(item, &format!("/feedback/{index}"))?;
            }
        }
        let attempts = value
            .get_mut("work")
            .and_then(|work| work.get_mut("attempts"))
            .and_then(Value::as_array_mut);
        if let Some(items) = attempts {
            for (index, item) in items.iter_mut().enumerate() {
                let prefix = format!("/work/attempts/{index}");
                self.optional(item, "checkpoint", &prefix)?;
                self.optional(item, "endCheckpoint", &prefix)?;
            }
        }
        Ok(())
    }

    /// Entities without path fields keep their exact stored bytes.
    pub fn entity(&mut self, kind: &str, id: &str, raw: &str) -> Result<String> {
        let mut value: Value = match kind {
            "task" | "operation" | "asset-reference" | "asset-task" => serde_json::from_str(raw)
                .with_context(|| format!("{kind} {id} is not valid JSON"))?,
            _ => return Ok(raw.to_owned()),
        };
        match kind {
            "task" => self.task(&mut value, "")?,
            "operation" => self.task(&mut value["taskAfter"], "/taskAfter")?,
            "asset-reference" => {
                self.path(&mut value["imagePath"], "/imagePath")?;
            }
            _ => self.asset(&mut value)?,
        }
        Ok(value.to_string())
    }
}
