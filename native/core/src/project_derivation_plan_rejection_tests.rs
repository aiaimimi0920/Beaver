use super::*;

fn rejected(f: &Fixture, label: &str, expected: &str) -> Result<()> {
    let before = data_backup::inventory(&f.base.source)?;
    let destination = f.base.temp.path().join("rejected");
    for error in [
        copy::inspect_source(&f.base.source).unwrap_err(),
        copy::prepare(f.base.request(), &destination).unwrap_err(),
    ] {
        assert!(
            format!("{error:#}").contains(expected),
            "{label}: {error:#}"
        );
    }
    assert!(!destination.exists(), "created destination for {label}");
    assert_eq!(data_backup::inventory(&f.base.source)?, before, "{label}");
    Ok(())
}

#[test]
fn project_derivation_plan_rejects_damaged_identity_references_and_history_before_copy(
) -> Result<()> {
    // Mutate one boundary at a time; all other records come from real business commands.
    for (kind, id, pointer, replacement, expected) in [
        (
            "object_task",
            "root",
            "/id",
            json!("different"),
            "IDENTITY_MISMATCH",
        ),
        (
            "object_task",
            "root",
            "/projectId",
            json!("other"),
            "IDENTITY_MISMATCH",
        ),
        (
            "object_task",
            "root",
            "/identity/schemaVersion",
            json!(2),
            "TASK_IDENTITY",
        ),
        (
            "object_task",
            "root",
            "/identity/layer",
            json!("future"),
            "unknown variant",
        ),
        (
            "object_task",
            "build",
            "/identity/objectId",
            json!("missing"),
            "TASK_IDENTITY",
        ),
        (
            "object_task",
            "build",
            "/objectId",
            json!("missing"),
            "OBJECT_MISSING",
        ),
        (
            "object_task",
            "build",
            "/runId",
            json!("missing"),
            "RUN_MISSING",
        ),
        (
            "object_task",
            "build",
            "/parentTaskId",
            json!("missing"),
            "PARENT_MISSING",
        ),
        (
            "object_task",
            "build",
            "/identity/baseline/selectedVersionId",
            json!("missing"),
            "BASELINE_VERSION_MISSING",
        ),
        (
            "object_task",
            "shape",
            "/identity/stageId",
            json!("other"),
            "TASK_IDENTITY",
        ),
        (
            "object_task",
            "root",
            "/status",
            json!("running"),
            "EXECUTED_TASK",
        ),
        (
            "object_task",
            "root",
            "/revision",
            json!(3),
            "TASK_HISTORY_MISMATCH",
        ),
        (
            "object_task",
            "root",
            "/dependsOn",
            json!(["missing"]),
            "DEPENDENCY_MISSING",
        ),
        (
            "object_task",
            "build",
            "/dependsOn",
            json!(["discard"]),
            "DEPENDENCY_CYCLE",
        ),
        (
            "object_task_plan_state",
            "original",
            "/revision",
            json!(5),
            "HISTORY_GAP",
        ),
        (
            "object_task_plan_state",
            "original",
            "/assumptions/0/planRevision",
            json!(5),
            "ASSUMPTION_REVISION",
        ),
        (
            "object_task_commit_receipt",
            "commit",
            "/taskIds/0",
            json!("missing"),
            "TASK_MISSING",
        ),
        (
            "object_task_commit_receipt",
            "commit",
            "/runs/0/objectId",
            json!("missing"),
            "RUN_IDENTITY",
        ),
        (
            "object_task_cancel_receipt",
            "cancel",
            "/taskRevision",
            json!(3),
            "CANCEL_REVISION",
        ),
        (
            "object_task_definition_revision",
            "revise",
            "/after/title",
            json!("not current"),
            "TASK_HISTORY_MISMATCH",
        ),
        (
            "object_task_definition_revision",
            "revise",
            "/before/dependsOn",
            json!(["missing"]),
            "DEPENDENCY_MISSING",
        ),
        (
            "object_task_draft",
            "locked",
            "/committedRequestId",
            json!("missing"),
            "COMMIT_MISSING",
        ),
        (
            "object_task_draft_unlock_receipt",
            "unlock",
            "/draft/projectId",
            json!("other"),
            "DRAFT_REVISION",
        ),
        (
            "object_task_draft_unlock_receipt",
            "unlock",
            "/request/expectedRevision",
            json!(0),
            "UNLOCK_RECEIPT",
        ),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut value: Value = storage.store().get(kind, id)?.unwrap();
        *value.pointer_mut(pointer).unwrap() = replacement;
        storage.store().put(kind, id, &value)?;
        drop(storage);
        rejected(&f, &format!("{kind}/{id}{pointer}"), expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_plan_rejects_unknown_fields_and_missing_history() -> Result<()> {
    for (kind, id) in [
        ("object_task", "root"),
        ("object_task_draft", "object-task-plan"),
        ("object_task_plan_state", "original"),
        ("object_task_commit_receipt", "commit"),
        ("object_task_cancel_receipt", "cancel"),
        ("object_task_definition_revision", "revise"),
        ("object_task_draft_unlock_receipt", "unlock"),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut value: Value = storage.store().get(kind, id)?.unwrap();
        value["futureField"] = json!("must not be lost");
        storage.store().put(kind, id, &value)?;
        drop(storage);
        rejected(
            &f,
            kind,
            if kind.ends_with("unlock_receipt") {
                "UNKNOWN_UNLOCK_FIELD"
            } else {
                "unknown field"
            },
        )?;
    }
    for (kind, id) in [
        ("object_task_commit_receipt", "commit"),
        ("object_task_cancel_receipt", "cancel"),
        ("object_task_definition_revision", "revise"),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage.store().remove(kind, id)?;
        drop(storage);
        rejected(&f, kind, "HISTORY_GAP")?;
    }
    Ok(())
}

#[test]
fn project_derivation_plan_rejects_execution_and_unhandled_records_without_creating_target(
) -> Result<()> {
    for field in [
        "status",
        "revision",
        "baselineVersionId",
        "mediumTaskId",
        "futureField",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut run = serde_json::to_value(&f.commit.runs[0])?;
        let id = run["id"].as_str().unwrap().to_owned();
        run[field] = match field {
            "status" => json!("running"),
            "revision" => json!(2),
            _ => json!("missing"),
        };
        storage.store().put("object_run", &id, &run)?;
        drop(storage);
        rejected(
            &f,
            field,
            match field {
                "futureField" => "unknown field",
                "mediumTaskId" => "TASK_IDENTITY",
                _ => "EXECUTED_RUN",
            },
        )?;
    }
    for kind in [
        "object_run_preparation",
        "object_attempt",
        "future_workflow",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage.store().put(kind, "unsupported", &json!({}))?;
        drop(storage);
        rejected(
            &f,
            kind,
            if kind == "future_workflow" {
                "UNKNOWN_ENTITY_KIND"
            } else {
                "missing field"
            },
        )?;
    }
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let queued = object_tasks::enqueue(&runtime, "original", &["build".into()])?;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].state, "queued");
    assert!(object_tasks::claim_next(&runtime, "original", "negative-control")?.is_some());
    drop(runtime);
    rejected(&f, "real claimed task", "EXECUTED_TASK")
}
