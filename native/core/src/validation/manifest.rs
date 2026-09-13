use super::{flow::Definition, model::Flow, repository, requests};
use crate::{
    files::{Files, Snapshot},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Validate the entire manifest before writing any revisions. Unchanged manifests
/// must not undo a flow revision made through the user interface.
pub fn import_manifest(
    store: &mut Store,
    files: &Files,
    project_id: &str,
    snapshot: &Snapshot,
) -> Result<Vec<Flow>> {
    let Some(hash) = snapshot.get("beaver.validation.json") else {
        return Ok(vec![]);
    };
    if store
        .get::<Value>("validationManifest", project_id)?
        .is_some_and(|v| v["hash"] == *hash)
    {
        return repository::flows(store, project_id);
    }
    let bytes = std::fs::read(files.blob(hash)?)?;
    if bytes.len() > 2 * 1024 * 1024 {
        bail!("Validation manifest exceeds 2 MiB");
    }
    let manifest: Value = serde_json::from_slice(&bytes)?;
    if manifest["schemaVersion"] != 1 {
        bail!("Unsupported beaver.validation.json schemaVersion");
    }
    let mut definitions: Vec<Definition> =
        serde_json::from_value(manifest["flows"].clone()).context("Invalid validation flows")?;
    if definitions.len() > 200 {
        bail!("At most 200 flows are supported per manifest");
    }
    let mut keys = BTreeSet::new();
    for definition in &mut definitions {
        if definition.roaming.is_some() && definition.steps.is_empty() {
            super::roaming::generate(definition, definition.config.seed)?;
        }
        definition.validate()?;
        for task_id in &definition.task_ids {
            let task: Value = repository::get(store, "task", task_id)?;
            anyhow::ensure!(
                task["projectId"] == project_id,
                "Flow task belongs to another project"
            );
        }
        if !keys.insert(definition.key.clone()) {
            bail!("Duplicate flow key: {}", definition.key);
        }
    }
    let existing = repository::flows(store, project_id)?;
    let mut records = vec![];
    let mut result = vec![];
    for definition in definitions {
        let current = existing.iter().find(|f| f.definition.key == definition.key);
        if let Some(current) = current.filter(|f| f.definition == definition) {
            result.push(current.clone());
            continue;
        }
        let reason = manifest["revisionReason"]
            .as_str()
            .unwrap_or("Updated by delivered game manifest");
        let flow = repository::prepare_flow(
            store,
            project_id,
            definition,
            current.map_or(0, |f| f.revision),
            reason,
        )?;
        records.extend(repository::flow_records(&flow)?);
        result.push(flow);
    }
    records.push((
        "validationManifest",
        project_id.into(),
        json!({"hash":hash,"at":repository::now()}),
    ));
    requests::commit(store, records)?;
    Ok(result)
}
