//! Isolated-copy operations and explicit adoption into the running host.
use anyhow::{ensure, Context, Result};
use beaver_core::{
    migration_bundle, project_derivation_assembly, project_migration_activation,
    project_migration_inventory, project_migration_partition,
    project_storage_router::ProjectStorageRouter,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

// Includes inspection so a second caller cannot race preparation or activation in this host.
static OPERATION: Mutex<()> = Mutex::new(());

#[path = "migration_registration.rs"]
mod registration;

pub(crate) struct Outcome {
    pub value: Value,
    pub registration_committed: bool,
    pub runtime_ready: bool,
}

impl Outcome {
    pub(crate) fn publish(
        mut self,
        notify: impl FnOnce(),
        wake: impl FnOnce() -> std::result::Result<(), String>,
    ) -> Value {
        if self.registration_committed {
            notify();
            if self.runtime_ready {
                if let Err(error) = wake() {
                    self.value["schedulerWakeError"] = serde_json::json!(error);
                }
            }
        }
        self.value
    }
}

fn path(input: &Value, field: &str, new: bool, host: &Path) -> Result<PathBuf> {
    let value = input[field].as_str().context("migration path missing")?;
    let path = Path::new(value);
    ensure!(path.is_absolute(), "{field} must be an absolute path");
    let resolved = if new {
        ensure!(
            !path.try_exists()?,
            "destination already exists; use a new directory"
        );
        let parent = fs::canonicalize(path.parent().context("destination has no parent")?)?;
        ensure!(parent.is_dir(), "destination parent is not a directory");
        parent.join(
            path.file_name()
                .context("destination has no directory name")?,
        )
    } else {
        fs::canonicalize(path).with_context(|| format!("cannot resolve {field}"))?
    };
    ensure!(
        !resolved.starts_with(host) && !host.starts_with(&resolved),
        "{field} overlaps the running host data directory"
    );
    Ok(resolved)
}

pub(crate) fn call(
    host: &Path,
    router: &ProjectStorageRouter,
    method: &str,
    input: &Value,
) -> Result<Outcome> {
    crate::business_catalog::validate(method, input)
        .map_err(|(_, error)| anyhow::anyhow!(error))?;
    let _operation = OPERATION
        .try_lock()
        .map_err(|_| anyhow::anyhow!("another migration operation is in progress"))?;
    if method == "migration.registerAssembly" {
        return registration::register(
            host,
            router,
            input,
            crate::project_runtime_lifecycle::recover,
        );
    }
    Ok(Outcome {
        value: execute(host, method, input)?,
        registration_committed: false,
        runtime_ready: false,
    })
}

fn execute(host: &Path, method: &str, input: &Value) -> Result<Value> {
    let host = fs::canonicalize(host)?;
    if method == "migration.activateAssembly" {
        let preparation = path(input, "preparation", false, &host)?;
        let destination = path(input, "destination", false, &host)?;
        return project_derivation_assembly::activate(&preparation, &destination);
    }
    let backup = path(input, "backup", false, &host)?;
    match method {
        "migration.inspect" => Ok(serde_json::to_value(project_migration_inventory::inspect(
            &backup,
        )?)?),
        "migration.prepareProjects" => {
            let destination = path(input, "destination", true, &host)?;
            migration_bundle::restore(&backup, &destination).with_context(|| {
                format!("restore failed; preserve any partial copy at {} and retry with a new destination", destination.display())
            })?;
            let receipt = project_migration_partition::partition(&backup, &destination)
                .with_context(|| format!("partition failed; pending copy preserved at {}; retry with a new destination", destination.display()))?;
            Ok(serde_json::to_value(receipt)?)
        }
        "migration.activate" => {
            let prepared = path(input, "prepared", false, &host)?;
            ensure!(
                prepared.join("PROJECT-MIGRATION.json").is_file(),
                "project partition preparation is not complete"
            );
            let tool_paths = input
                .get("toolPaths")
                .map(|_| path(input, "toolPaths", false, &host))
                .transpose()?;
            project_migration_activation::activate_copy(&backup, &prepared, tool_paths.as_deref())
        }
        _ => anyhow::bail!("unknown migration method"),
    }
}

#[cfg(test)]
#[path = "migration_runtime_tests.rs"]
mod tests;
