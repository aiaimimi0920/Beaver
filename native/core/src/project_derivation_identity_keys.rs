//! Business key rules; task-scoped semantic keys remain local to their remapped kind.
use crate::{
    project_derivation_copy::Request,
    project_derivation_identity::{generated, Key, Target},
    project_migration_ownership::Entity,
    validation::repository::digest,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn flow_id(value: &Value, project: &str) -> Result<String> {
    let key = value
        .pointer("/definition/key")
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
        .context("derivation flow key missing")?;
    digest(&json!([project, key]))
}

pub(crate) fn flows(entities: &[Entity], request: &Request) -> Result<BTreeMap<String, String>> {
    let mut flows = BTreeMap::new();
    for entity in entities
        .iter()
        .filter(|entity| entity.kind == "validationFlow")
    {
        let value = entity.value.as_ref().context("invalid flow JSON")?;
        ensure!(
            value["id"] == entity.id && entity.id == flow_id(value, &request.source_project_id)?,
            "derivation flow identity does not match business key"
        );
        flows.insert(
            entity.id.clone(),
            flow_id(value, &request.target_project_id)?,
        );
    }
    Ok(flows)
}

pub(crate) fn target(
    entity: &Entity,
    request: &Request,
    flows: &BTreeMap<String, String>,
) -> Result<Target> {
    let value = entity.value.as_ref().context("invalid entity JSON")?;
    let kind = entity.kind.as_str();
    let id = entity.id.as_str();
    let mapped = match kind {
        "project" | "validationSettings" | "validationManifest" => {
            request.target_project_id.clone()
        }
        "task-callback-revision"
        | "asset-task"
        | "framework-configuration"
        | "validationCoverage" => generated(request, "task", id)?,
        "feature" => format!(
            "{}:{}",
            request.target_project_id,
            id.strip_prefix(&format!("{}:", request.source_project_id))
                .context("invalid feature key")?
        ),
        "validationFlow" => flows.get(id).context("flow mapping missing")?.clone(),
        "validationFlowRevision" => {
            let flow = value["id"]
                .as_str()
                .context("flow revision identity missing")?;
            let target = flows.get(flow).context("flow revision mapping missing")?;
            ensure!(
                *target == flow_id(value, &request.target_project_id)?,
                "flow revision business key mismatch"
            );
            format!(
                "{target}:{}",
                value["revision"]
                    .as_u64()
                    .context("flow revision missing")?
            )
        }
        "validationRequest" => return Ok(Target::Archive),
        "validationRepairDecision" => {
            ensure!(value["id"] == id, "repair decision identity mismatch");
            generated(request, "validationRun", id)?
        }
        "task"
        | "operation"
        | "framework-operation"
        | "asset-reference"
        | "validationRun"
        | "validationBaseline"
        | "validationFeedback"
        | "validationRelease" => {
            ensure!(
                value["id"] == id,
                "derivation entity identity mismatch: {kind}/{id}"
            );
            generated(request, kind, id)?
        }
        _ => {
            let (prefix, task) = kind.split_once('/').context("unmapped derivation kind")?;
            if ![
                "asset-delivery",
                "asset-delivery-decisions",
                "task-callback",
                "framework-trace",
                "framework-observation",
                "framework-recovery",
                "framework-check",
                "framework-judgment",
                "framework-judgment-history",
            ]
            .contains(&prefix)
            {
                bail!("unmapped derivation kind: {kind}");
            }
            return Ok(Target::Remap {
                key: Key {
                    kind: format!("{prefix}/{}", generated(request, "task", task)?),
                    id: if prefix == "framework-trace" {
                        ensure!(value["id"] == id, "trace key mismatch");
                        // Traces outlive bounded calls; derive the same ID even after call pruning.
                        generated(request, "calls", id)?
                    } else if prefix == "framework-observation" {
                        ensure!(
                            value["referenceId"] == id,
                            "observation reference key mismatch"
                        );
                        generated(request, "asset-reference", id)?
                    } else {
                        id.into()
                    },
                },
            });
        }
    };
    Ok(Target::Remap {
        key: Key {
            kind: kind.into(),
            id: mapped,
        },
    })
}
