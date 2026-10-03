use anyhow::{bail, ensure, Result};
use serde_json::Value;

// The legacy project slice and explicitly opted-in external runs share Core validation.
pub fn validate(method: &str, input: &Value) -> Result<()> {
    let (required, optional) = match method {
        "state" | "settings.get" | "shutdown" | "external.pending" => ("", ""),
        "project.create" => ("parent:s,name:s,template:s", "design:o,blueprint:o,npr:o"),
        "project.npr.install" => ("id:s,godot:s", ""),
        "workflow.list" | "task.events" | "task.interrupt" => ("id:s", ""),
        "workflow.run" => ("id:s,workflow:s,action:s", "definition:s,camera:o,grayscale:b"),
        "external.context" | "external.history" => ("taskId:s", ""),
        "external.receipt" | "external.jobEvidence" => ("taskId:s,runId:s,requestId:s", ""),
        "external.submitPlan" | "external.tool" | "external.finish" => ("taskId:s,runId:s,revision:i,requestId:s,arguments:o", ""),
        "task.create" => ("projectId:s,prompt:s", "executionMode:s,title:s,direction:s,stopConditions:s,references:a,maxMinutes:i,capability:s,askRatio:r,decompose:b,autoAccept:b,assetTask:b,objectFramework:o"),
        "task.answer" => ("id:s,questionId:s,answers:o", "automatic:a"),
        "task.continue" => ("id:s,text:s", "freshContext:b"),
        "logs.query" => ("", "after:i,limit:i,taskId:s,projectId:s,method:s"),
        "settings.save" => ("settings:o", "keys:o"),
        _ => bail!("Unknown headless method"),
    };
    let input = input
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Input must be an object"))?;
    let fields: Vec<_> = required
        .split(',')
        .chain(optional.split(','))
        .filter(|field| !field.is_empty())
        .map(|field| field.split_once(':').unwrap())
        .collect();
    ensure!(
        input
            .keys()
            .all(|key| fields.iter().any(|(name, _)| *name == key)),
        "Unknown input field"
    );
    for field in required.split(',').filter(|field| !field.is_empty()) {
        ensure!(
            input.contains_key(field.split_once(':').unwrap().0),
            "Missing required input field"
        );
    }
    for (name, kind) in fields {
        if let Some(value) = input.get(name) {
            let valid = match kind {
                "s" => value.is_string(),
                "o" => value.is_object(),
                "a" => value.is_array(),
                "b" => value.is_boolean(),
                "i" => value.as_u64().is_some(),
                "r" => value.is_null() || value.as_u64().is_some(),
                _ => false,
            };
            ensure!(valid, "Invalid input field type: {name}");
        }
    }
    if method == "settings.save" {
        ensure!(
            input
                .get("keys")
                .is_none_or(|keys| keys.as_object().is_some_and(|keys| keys.is_empty())),
            "Headless accepts key-free settings only; no plaintext credential fallback"
        );
    }
    if method == "task.create" {
        beaver_core::object_framework::require_legacy(&Value::Object(input.clone()))?;
    }
    Ok(())
}
