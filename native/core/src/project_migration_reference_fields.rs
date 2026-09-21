//! Explicit legacy entity fields; absence is not a full record-schema validation.
use crate::project_migration_ownership::Entity;
use crate::project_migration_references::References;
use serde_json::Value;

fn task(refs: &mut References<'_>, source: &Entity, value: &Value, prefix: &str) {
    for (field, kind) in [
        ("/parentTaskId", "task"),
        ("/validationFeedbackId", "validationFeedback"),
        ("/validationRepair/runId", "validationRun"),
        ("/codeValidation/runId", "validationRun"),
        ("/feature/previous/taskId", "task"),
    ] {
        refs.one(
            source,
            &format!("{prefix}{field}"),
            kind,
            value.pointer(field),
        );
    }
    for field in ["/subtaskIds", "/dependsOn"] {
        refs.many(
            source,
            &format!("{prefix}{field}"),
            "task",
            value.pointer(field),
        );
    }
}

pub(crate) fn inspect(refs: &mut References<'_>, source: &Entity, value: &Value) {
    let (singles, arrays): (&[(&str, &str)], &[(&str, &str)]) = match source.kind.as_str() {
        "task" => {
            task(refs, source, value, "");
            return;
        }
        "operation" => {
            if let Some(after) = value.get("taskAfter") {
                task(refs, source, after, "/taskAfter");
            }
            (&[("/taskId", "task")], &[])
        }
        "validationCoverage" => (
            &[("/taskId", "task")],
            &[("/flowIds", "validationFlow"), ("/runIds", "validationRun")],
        ),
        "validationFlow" => (&[], &[("/definition/taskIds", "task")]),
        "validationFlowRevision" => (
            &[("/id", "validationFlow")],
            &[("/definition/taskIds", "task")],
        ),
        "validationRun" => (
            &[
                ("/taskId", "task"),
                ("/releaseId", "validationRelease"),
                ("/baselineId", "validationBaseline"),
            ],
            &[],
        ),
        "validationBaseline" => (
            &[
                ("/flowId", "validationFlow"),
                ("/runId", "validationRun"),
                ("/previousId", "validationBaseline"),
            ],
            &[],
        ),
        "validationFeedback" => (
            &[
                ("/taskId", "task"),
                ("/runId", "validationRun"),
                ("/rerunId", "validationRun"),
                ("/flowId", "validationFlow"),
            ],
            &[],
        ),
        "validationRelease" => (
            &[],
            &[("/flowIds", "validationFlow"), ("/runIds", "validationRun")],
        ),
        "validationRepairDecision" => {
            refs.one(
                source,
                "$key",
                "validationRun",
                Some(&Value::String(source.id.clone())),
            );
            (
                &[
                    ("/decision/result/taskId", "task"),
                    ("/decision/result/feedbackId", "validationFeedback"),
                ],
                &[],
            )
        }
        kind if kind.starts_with("asset-delivery/") => {
            refs.many(
                source,
                "/inputCandidates",
                kind,
                value.get("inputCandidates"),
            );
            return;
        }
        _ => return,
    };
    for (field, kind) in singles {
        refs.one(source, field, kind, value.pointer(field));
    }
    for (field, kind) in arrays {
        refs.many(source, field, kind, value.pointer(field));
    }
    if source.kind == "validationRun" {
        if let Some(flow) = value.get("flow").filter(|flow| !flow.is_null()) {
            refs.frozen_flow(source, "/flow", flow);
        }
    }
    if source.kind == "validationRelease" {
        for field in ["flows", "excluded"] {
            let Some(items) = value.get(field) else {
                continue;
            };
            let Some(items) = items.as_array() else {
                refs.invalid(source, &format!("/{field}"));
                continue;
            };
            for (index, item) in items.iter().enumerate() {
                let path = format!("/{field}/{index}");
                if field == "flows" {
                    refs.frozen_flow(source, &path, item);
                } else if item.get("flowId").is_some_and(Value::is_string) {
                    refs.one(
                        source,
                        &format!("{path}/flowId"),
                        "validationFlow",
                        item.get("flowId"),
                    );
                } else {
                    refs.invalid(source, &path);
                }
            }
        }
    }
}
