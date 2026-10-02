//! Shared desktop/HTTP/MCP entry points for the offline independent-copy workflow.
use super::path;
use anyhow::{Context, Result};
use beaver_core::{project_derivation_assembly as assembly, project_derivation_copy as copy};
use serde_json::{json, Value};
use std::path::Path;

fn prepared(value: copy::Prepared) -> Value {
    json!({"request": value.request, "entities": value.identities.entities.len(),
        "calls": value.identities.calls.len()})
}

fn assembled(value: assembly::Receipt, activated: bool) -> Value {
    json!({"activated": activated, "assembly": {
        "projectId": value.project_id, "binding": value.binding,
        "preparationSha256": value.preparation_sha256,
        "tasksInterrupted": value.tasks_interrupted,
        "sessionPathsRewritten": value.session_paths_rewritten,
    }})
}

pub(super) fn execute(host: &Path, method: &str, input: &Value) -> Result<Value> {
    match method {
        "migration.inspectDerivationSource" => {
            let source = path(input, "source", false, host)?;
            Ok(serde_json::to_value(copy::inspect_source(&source)?)?)
        }
        "migration.prepareDerivation" => {
            let source = path(input, "source", false, host)?;
            let destination = path(input, "preparation", true, host)?;
            let request = copy::Request {
                source,
                source_project_id: input["sourceProjectId"]
                    .as_str()
                    .context("source ID missing")?
                    .into(),
                target_project_id: input["targetProjectId"]
                    .as_str()
                    .context("target ID missing")?
                    .into(),
                request_id: input["requestId"]
                    .as_str()
                    .context("request ID missing")?
                    .into(),
            };
            copy::prepare(request, &destination).map(prepared).with_context(|| format!(
                "preparation failed; preserve {} and inspect its receipt before retrying into a new directory",
                destination.display()
            ))
        }
        "migration.inspectDerivation" => {
            let preparation = path(input, "preparation", false, host)?;
            Ok(prepared(copy::inspect(&preparation)?))
        }
        "migration.assembleDerivation" => {
            let preparation = path(input, "preparation", false, host)?;
            let destination = path(input, "destination", true, host)?;
            assembly::create(&preparation, &destination).map(|value| assembled(value, false))
                .with_context(|| format!("assembly failed; preserve {} and inspect its receipt before retrying into a new directory", destination.display()))
        }
        "migration.inspectAssembly" => {
            let preparation = path(input, "preparation", false, host)?;
            let destination = path(input, "destination", false, host)?;
            let activated = !destination.join(".beaver-migration-pending").try_exists()?;
            let receipt = if activated {
                serde_json::from_value(
                    assembly::inspect_activated(&preparation, &destination)?["assembly"].take(),
                )?
            } else {
                assembly::inspect(&preparation, &destination)?
            };
            Ok(assembled(receipt, activated))
        }
        _ => anyhow::bail!("unknown derivation method"),
    }
}

#[cfg(all(test, windows))]
#[path = "migration_derivation_tests.rs"]
mod tests;
