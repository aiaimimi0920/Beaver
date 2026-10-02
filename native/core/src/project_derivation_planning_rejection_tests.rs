use super::*;

pub(super) fn rejected(f: &Fixture, label: &str, expected: &str) -> Result<()> {
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
    assert!(!destination.exists(), "created target for {label}");
    assert_eq!(data_backup::inventory(&f.base.source)?, before, "{label}");
    Ok(())
}

#[test]
fn project_derivation_planning_rejects_unknown_wrapper_and_typed_nested_fields() -> Result<()> {
    for (kind, id, pointer, expected) in [
        ("object_task_planning", "start-adopted", "", "UNKNOWN_FIELD"),
        ("object_task_planning_head", "adopted", "", "UNKNOWN_FIELD"),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "",
            "UNKNOWN_FIELD",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/input",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/baseline",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/objects/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/tasks/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/runs/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/assumptions/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/decisions/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/proposal/tasks/0",
            "unknown field",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/adoptedDraft",
            "unknown field",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/request",
            "unknown field",
        ),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut value: Value = storage.store().get(kind, id)?.unwrap();
        value.pointer_mut(pointer).unwrap()["futureField"] = json!("must not be lost");
        storage.store().put(kind, id, &value)?;
        drop(storage);
        rejected(&f, &format!("{kind}/{id}{pointer}"), expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_planning_rejects_invalid_identity_references_and_provenance() -> Result<()> {
    for (kind, id, pointer, replacement, expected) in [
        (
            "object_task_planning",
            "start-adopted",
            "/session/id",
            json!("other"),
            "IDENTITY",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/projectId",
            json!("other"),
            "IDENTITY",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/input/requestId",
            json!("other"),
            "IDENTITY",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/input/projectId",
            json!("other"),
            "IDENTITY",
        ),
        (
            "object_task_planning_head",
            "adopted",
            "/sessionId",
            json!("missing"),
            "SESSION_MISSING",
        ),
        (
            "object_task_planning_head",
            "adopted",
            "/projectId",
            json!("other"),
            "HEAD_IDENTITY",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/sessionId",
            json!("missing"),
            "SESSION_MISSING",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/request/requestId",
            json!("other"),
            "RECEIPT_IDENTITY",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/request/projectId",
            json!("other"),
            "RECEIPT_IDENTITY",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/request/expectedRevision",
            json!(0),
            "RECEIPT_IDENTITY",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/request/answers/user",
            json!("tampered answer"),
            "ANSWER_RECEIPT",
        ),
        (
            "object_task_planning_receipt",
            "answer-adopted",
            "/operation",
            json!("execute"),
            "UNKNOWN_OPERATION",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/tasks/0/id",
            json!("missing"),
            "TASK_MISSING",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/tasks/0/prompt",
            json!("changed history"),
            "CONTEXT_DEFINITION",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/tasks/0/status",
            json!("cancelled"),
            "CONTEXT_TASK",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/runs/0/id",
            json!("missing"),
            "RUN_MISSING",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/objects/0/id",
            json!("missing"),
            "IMPORT_VERSION_IDENTITY_MISMATCH",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/context/assumptions/0/statement",
            json!("changed"),
            "CONTEXT_ASSUMPTION",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/adoptedDraft/plan/tasks/0/prompt",
            json!("changed"),
            "ADOPTION",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/proposal/tasks/0/dependsOn",
            json!(["missing"]),
            "DEPENDENCY_MISSING",
        ),
        (
            "object_task_planning",
            "start-adopted",
            "/session/proposal/assumptions/0/sourceDetail",
            json!("planning:missing"),
            "ADOPTION",
        ),
        (
            "object_task_planning",
            "start-object-task-plan",
            "/session/proposal/assumptions/0/sourceDetail",
            json!("planning:missing"),
            "SOURCE_SESSION_MISSING",
        ),
        (
            "object_task_planning",
            "start-object-task-plan",
            "/session/proposal/assumptions/0/sourceDetail",
            json!("planning:start-adopted/decision:missing"),
            "SOURCE_DECISION_MISSING",
        ),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut value: Value = storage.store().get(kind, id)?.unwrap();
        let pointer = if pointer == "/context/objects/0/id" {
            // IDs are generated: select the frozen-version owner, not an arbitrary sort head.
            let index = value["context"]["objects"]
                .as_array()
                .unwrap()
                .iter()
                .position(|object| object["id"] == f.base.parent.id)
                .unwrap();
            format!("/context/objects/{index}/id")
        } else {
            pointer.to_owned()
        };
        *value.pointer_mut(&pointer).unwrap() = replacement;
        storage.store().put(kind, id, &value)?;
        drop(storage);
        rejected(&f, &format!("{kind}/{id}{pointer}"), expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_planning_requires_complete_heads_and_receipts() -> Result<()> {
    for (kind, id, expected) in [
        ("object_task_planning_head", "adopted", "HEAD_MISSING"),
        (
            "object_task_planning_receipt",
            "start-adopted",
            "START_MISSING",
        ),
        (
            "object_task_planning_receipt",
            "adopt-history",
            "ADOPT_MISSING",
        ),
        ("object_task_planning", "start-cancelled", "SESSION_MISSING"),
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage.store().remove(kind, id)?;
        drop(storage);
        rejected(&f, kind, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_planning_running_is_unsupported_and_malformed_declarations_are_rejected(
) -> Result<()> {
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    fixture::start(&runtime, "running")?;
    drop(runtime);
    rejected(&f, "running", "RUNNING_UNSUPPORTED")?;
    for kind in [
        "object_task_planning_declaration",
        "object_task_planning_declaration_receipt",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage
            .store()
            .put(kind, "declaration", &json!({"projectId":"original"}))?;
        drop(storage);
        rejected(&f, kind, "unknown field")?;
    }
    Ok(())
}
