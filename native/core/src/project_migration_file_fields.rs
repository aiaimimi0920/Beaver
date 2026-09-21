//! Explicit legacy file-bearing fields; opaque session and context payloads are not scanned.
use crate::{
    project_migration_file_references::FileReferences,
    project_migration_files::FileIndex,
    project_migration_ownership::{Entity, Ownership},
    project_migration_references::Checks,
};
use serde_json::Value;
use std::path::Path;

fn optional<'a>(value: &'a Value, field: &str) -> Option<&'a Value> {
    value.get(field).filter(|value| !value.is_null())
}

fn task(checks: &mut FileReferences<'_>, source: &Entity, value: &Value, prefix: &str) {
    checks.workspace(source, value, prefix);
    if let Some(references) = value.get("references") {
        if let Some(items) = references.as_array() {
            for (index, item) in items.iter().enumerate() {
                checks.task_reference(
                    source,
                    value,
                    &item["path"],
                    &format!("{prefix}/references/{index}/path"),
                );
            }
        } else {
            checks.invalid(source, &format!("{prefix}/references"));
        }
    }
    if let Some(path) = optional(value, "assetRestore") {
        checks.checkpoint(source, path, &format!("{prefix}/assetRestore"));
    }
    if let Some(seed) = optional(value, "assetFeedbackSeed") {
        feedback(checks, source, seed, &format!("{prefix}/assetFeedbackSeed"));
    }
}

fn feedback(checks: &mut FileReferences<'_>, source: &Entity, value: &Value, prefix: &str) {
    if !value.is_object() {
        return checks.invalid(source, prefix);
    }
    checks.reference(source, &value["reference"], &format!("{prefix}/reference"));
    if let Some(path) = optional(value, "checkpoint") {
        checks.checkpoint(source, path, &format!("{prefix}/checkpoint"));
    }
}

fn attempts(checks: &mut FileReferences<'_>, source: &Entity, value: &Value) {
    let Some(work) = value.get("work") else {
        return;
    };
    if !work.is_object() {
        return checks.invalid(source, "/work");
    }
    let Some(items) = work.get("attempts").and_then(Value::as_array) else {
        return checks.invalid(source, "/work/attempts");
    };
    for (index, item) in items.iter().enumerate() {
        let prefix = format!("/work/attempts/{index}");
        if !item.is_object() {
            checks.invalid(source, &prefix);
            continue;
        }
        for field in ["checkpoint", "endCheckpoint"] {
            if let Some(path) = optional(item, field) {
                checks.checkpoint(source, path, &format!("{prefix}/{field}"));
            }
        }
    }
}

fn asset(checks: &mut FileReferences<'_>, source: &Entity, value: &Value) {
    if let Some(path) = optional(value, "checkpoint") {
        checks.checkpoint(source, path, "/checkpoint");
    }
    if let Some(reference) = optional(value, "lastFrame") {
        checks.reference(source, reference, "/lastFrame");
    }
    if let Some(items) = value.get("feedback") {
        if let Some(items) = items.as_array() {
            for (index, item) in items.iter().enumerate() {
                feedback(checks, source, item, &format!("/feedback/{index}"));
            }
        } else {
            checks.invalid(source, "/feedback");
        }
    }
    attempts(checks, source, value);
}

pub(crate) fn inspect(
    entities: &[Entity],
    ownership: &Ownership,
    root: &Path,
    index: &FileIndex<'_>,
) -> Checks {
    let mut checks = FileReferences::new(root, index, ownership);
    for source in entities {
        let Some(value) = &source.value else { continue };
        match source.kind.as_str() {
            "task" => task(&mut checks, source, value, ""),
            "operation" => {
                if let Some(value) = value.get("taskAfter") {
                    task(&mut checks, source, value, "/taskAfter");
                }
            }
            "asset-reference" => checks.reference(source, value, ""),
            "asset-task" => asset(&mut checks, source, value),
            "validationRun" => {
                if let Some(evidence) = value.get("evidence") {
                    if let Some(items) = evidence.as_array() {
                        for (index, item) in items.iter().enumerate() {
                            checks.evidence(source, item, &format!("/evidence/{index}"));
                        }
                    } else {
                        checks.invalid(source, "/evidence");
                    }
                }
            }
            _ => {}
        }
    }
    checks.checks
}
