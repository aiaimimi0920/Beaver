//! Registration commits before recovery; later failures must remain visible and retryable.
use super::{path, Outcome};
use anyhow::{Context, Result};
use beaver_core::{files::Files, project_storage_router::ProjectStorageRouter, store::Store};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub(super) fn register(
    host: &Path,
    router: &ProjectStorageRouter,
    input: &Value,
    recover: impl FnOnce(&mut Store, &Files) -> Result<()>,
) -> Result<Outcome> {
    let host = fs::canonicalize(host)?;
    let preparation = path(input, "preparation", false, &host)?;
    let destination = path(input, "destination", false, &host)?;
    let mut value = router.register_assembly(&preparation, &destination, &host)?;
    let id = value["projectId"]
        .as_str()
        .context("assembly registered but project ID missing")?;
    let runtime = router.open_registered_with(id, recover);
    let runtime_ready = runtime.is_ok();
    value["registrationCommitted"] = json!(true);
    value["runtimeReady"] = json!(runtime_ready);
    value["ok"] = json!(runtime_ready);
    value["runtimeError"] = match runtime {
        Ok(_) => Value::Null,
        Err(error) => json!(format!("{error:#}")),
    };
    Ok(Outcome {
        value,
        registration_committed: true,
        runtime_ready,
    })
}

#[cfg(all(test, windows))]
#[path = "migration_registration_tests.rs"]
mod tests;
