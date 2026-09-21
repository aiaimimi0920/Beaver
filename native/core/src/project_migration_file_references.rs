//! Verify declared file paths against the offline archive, never the live filesystem.
use crate::{
    files::Files,
    migration_import,
    project_migration_files::FileIndex,
    project_migration_ownership::{Entity, Ownership},
    project_migration_references::{Checks, Issue},
};
use serde_json::Value;
use std::path::Path;

type Check<T> = Result<T, &'static str>;

pub(crate) struct FileReferences<'a> {
    pub checks: Checks,
    root: &'a Path,
    index: &'a FileIndex<'a>,
    ownership: &'a Ownership,
}

fn component(value: &str) -> bool {
    !value.is_empty()
        && !value.ends_with(['.', ' '])
        && !value.contains(['/', '\\', ':', '\0', '<', '>', '"', '|', '?', '*'])
}

fn identifier<'a>(value: &'a Value, field: &str) -> Check<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|id| component(id))
        .ok_or("INVALID_FILE_REFERENCE")
}

impl<'a> FileReferences<'a> {
    pub fn new(root: &'a Path, index: &'a FileIndex<'a>, ownership: &'a Ownership) -> Self {
        Self {
            checks: Checks::default(),
            root,
            index,
            ownership,
        }
    }

    fn record(&mut self, source: &Entity, field: &str, result: Check<()>) {
        self.checks.checked += 1;
        if let Err(reason) = result {
            self.checks.issues.push(Issue::new(source, field, reason));
        }
    }

    pub fn invalid(&mut self, source: &Entity, field: &str) {
        self.record(source, field, Err("INVALID_FILE_REFERENCE"));
    }

    fn absolute(&self, value: Option<&Value>) -> Check<String> {
        let value = value
            .and_then(Value::as_str)
            .ok_or("INVALID_FILE_REFERENCE")?;
        migration_import::relative(self.root, value)
            .map_err(|_| "INVALID_FILE_REFERENCE")?
            .filter(|path| !path.is_empty())
            .ok_or("FILE_OUTSIDE_APPLICATION")
    }

    fn check(
        &self,
        source: &Entity,
        path: &str,
        directory: bool,
        hash: Option<&Value>,
    ) -> Check<()> {
        let entry = self.index.entry(path)?;
        if directory != entry.sha256.is_none() {
            return Err("FILE_TYPE_MISMATCH");
        }
        let owner = self
            .ownership
            .entity(source)
            .map_err(|_| "FILE_OWNER_UNRESOLVED")?;
        if owner.is_none() {
            return Err("FILE_OWNER_UNRESOLVED");
        }
        if self
            .index
            .owner(path)
            .map_err(|_| "FILE_OWNER_UNRESOLVED")?
            != owner
        {
            return Err("FILE_PROJECT_MISMATCH");
        }
        if let Some(hash) = hash {
            let hash = hash.as_str().ok_or("INVALID_FILE_HASH")?;
            Files::new(Default::default())
                .blob(hash)
                .map_err(|_| "INVALID_FILE_HASH")?;
            if entry.sha256.as_deref() != Some(hash) {
                return Err("FILE_HASH_MISMATCH");
            }
        }
        Ok(())
    }

    fn workspace_path(&self, source: &Entity, task: &Value) -> Check<String> {
        let id = identifier(task, "id")?;
        let path = self.absolute(task.get("workspace"))?;
        if !path.eq_ignore_ascii_case(&format!("workspaces/{id}")) {
            return Err("FILE_REFERENCE_PATH_MISMATCH");
        }
        self.check(source, &path, true, None)?;
        Ok(path)
    }

    pub fn workspace(&mut self, source: &Entity, task: &Value, prefix: &str) {
        let result = self.workspace_path(source, task).map(|_| ());
        self.record(source, &format!("{prefix}/workspace"), result);
    }

    pub fn task_reference(&mut self, source: &Entity, task: &Value, value: &Value, field: &str) {
        let result = (|| {
            let relative = value.as_str().ok_or("INVALID_FILE_REFERENCE")?;
            if !relative.split('/').all(component) {
                return Err("INVALID_FILE_REFERENCE");
            }
            let workspace = self.workspace_path(source, task)?;
            // Task resources read the task's copy, never a later project file or another task.
            self.check(source, &format!("{workspace}/{relative}"), false, None)
                .map_err(|reason| match reason {
                    // Existing tasks may deliberately reference files which do not exist yet.
                    "FILE_NOT_FOUND" => "TASK_REFERENCE_UNAVAILABLE",
                    other => other,
                })
        })();
        self.record(source, field, result);
    }

    pub fn reference(&mut self, source: &Entity, reference: &Value, prefix: &str) {
        let result = (|| {
            let id = identifier(reference, "id")?;
            let task = identifier(reference, "taskId")?;
            let project = identifier(reference, "projectId")?;
            if self
                .ownership
                .task(task)
                .map_err(|_| "FILE_OWNER_UNRESOLVED")?
                .as_deref()
                != Some(project)
            {
                return Err("FILE_PROJECT_MISMATCH");
            }
            let path = self.absolute(reference.get("imagePath"))?;
            if !path.eq_ignore_ascii_case(&format!("asset-observer/{task}/references/{id}.png")) {
                return Err("FILE_REFERENCE_PATH_MISMATCH");
            }
            self.check(source, &path, false, Some(&reference["sha256"]))
        })();
        self.record(source, &format!("{prefix}/imagePath"), result);
    }

    pub fn checkpoint(&mut self, source: &Entity, value: &Value, field: &str) {
        let result = (|| {
            let path = self.absolute(Some(value))?;
            let parts: Vec<_> = path.split('/').collect();
            if parts.len() < 4
                || !parts[0].eq_ignore_ascii_case("codex")
                || !parts[2].eq_ignore_ascii_case("asset-checkpoints")
            {
                return Err("FILE_REFERENCE_PATH_MISMATCH");
            }
            // A follow-up can restore an earlier task's checkpoint within the same project.
            self.check(source, &path, false, None)
        })();
        self.record(source, field, result);
    }

    pub fn evidence(&mut self, source: &Entity, value: &Value, field: &str) {
        let result = (|| {
            if !component(&source.id) {
                return Err("INVALID_FILE_REFERENCE");
            }
            let file = value["file"].as_str().ok_or("INVALID_FILE_REFERENCE")?;
            if !file.split('/').all(component) {
                return Err("INVALID_FILE_REFERENCE");
            }
            self.check(
                source,
                &format!("validation/{}/{file}", source.id),
                false,
                Some(&value["sha256"]),
            )
        })();
        self.record(source, &format!("{field}/file"), result);
    }
}
