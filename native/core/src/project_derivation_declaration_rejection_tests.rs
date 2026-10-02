use super::*;

const HEAD: &str = "object_task_planning_declaration";
const RECEIPT: &str = "object_task_planning_declaration_receipt";

fn rejected(f: &Fixture, label: &str, expected: &str) -> Result<()> {
    let before = data_backup::inventory(&f.base.source)?;
    let destination = f.base.temp.path().join("rejected-declaration");
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
fn project_derivation_declaration_rejects_malformed_and_impossible_history_before_copy(
) -> Result<()> {
    for (pointer, replacement, expected) in [
        ("/futureField", json!(true), "unknown field"),
        ("/request/futureField", json!(true), "unknown field"),
        ("/request/projectId", json!("other"), "DECLARATION_IDENTITY"),
        ("/request/requestId", json!("other"), "DECLARATION_KEY"),
        ("/request/taskId", json!("missing"), "DECLARATION_KEY"),
        ("/request/reason", json!(" "), "DECLARATION_REASON"),
        (
            "/request/reason",
            json!("x".repeat(2001)),
            "DECLARATION_REASON",
        ),
        (
            "/request/expectedPlanRevision",
            json!(5),
            "SNAPSHOT_REVISION",
        ),
        (
            "/request/expectedPlanRevision",
            json!(0),
            "DECLARATION_TASK_MISSING",
        ),
        (
            "/request/expectedPlanRevision",
            json!(3),
            "DECLARATION_SCOPE_HASH",
        ),
        (
            "/request/expectedScopeHash",
            json!("0".repeat(64)),
            "DECLARATION_SCOPE_HASH",
        ),
        ("/declaredBy", json!("ai"), "unknown variant"),
        ("/createdAt", json!("invalid"), "DECLARATION_TIMESTAMP"),
        ("/taskIds", json!(["root"]), "DECLARATION_TASK_SET"),
        (
            "/taskIds",
            json!(["build", "discard", "discard-fine", "shape", "root"]),
            "DECLARATION_TASK_SET",
        ),
        (
            "/taskIds",
            json!(["build", "discard", "discard-fine", "root", "shape", "shape"]),
            "DECLARATION_TASK_SET",
        ),
    ] {
        let f = Fixture::with_declarations()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut value = storage.store().get::<Value>(HEAD, "root")?.unwrap();
        if pointer == "/futureField" {
            value["futureField"] = replacement;
        } else if pointer == "/request/futureField" {
            value["request"]["futureField"] = replacement;
        } else {
            *value.pointer_mut(pointer).unwrap() = replacement;
        }
        storage.store().put(HEAD, "root", &value)?;
        storage.store().put(RECEIPT, "revised-root", &value)?;
        drop(storage);
        rejected(&f, pointer, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_declaration_requires_exact_latest_receipt_without_timestamp_ordering(
) -> Result<()> {
    for (kind, id, expected) in [
        (HEAD, "root", "HEAD_MISSING"),
        (RECEIPT, "revised-root", "RECEIPT_MISSING_OR_MISMATCH"),
    ] {
        let f = Fixture::with_declarations()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage.store().remove(kind, id)?;
        drop(storage);
        rejected(&f, id, expected)?;
    }
    let f = Fixture::with_declarations()?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let older = storage
        .store()
        .get::<Value>(RECEIPT, "early-root")?
        .unwrap();
    storage.store().put(HEAD, "root", &older)?;
    drop(storage);
    rejected(&f, "regressed head", "HEAD_REGRESSED")?;
    let f = Fixture::with_declarations()?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let mut value = storage.store().get::<Value>(HEAD, "root")?.unwrap();
    value["request"]["reason"] = json!("Changed head only");
    storage.store().put(HEAD, "root", &value)?;
    drop(storage);
    rejected(&f, "head content differs", "RECEIPT_MISSING_OR_MISMATCH")?;
    let f = Fixture::with_declarations()?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    // A wall clock may move backwards. Two receipts in the same revision remain valid.
    for id in ["early-root", "early-root-again"] {
        let mut value = storage.store().get::<Value>(RECEIPT, id)?.unwrap();
        value["createdAt"] = json!(if id == "early-root" {
            "2030-01-01T00:00:00Z"
        } else {
            "2020-01-01T00:00:00Z"
        });
        storage.store().put(RECEIPT, id, &value)?;
    }
    drop(storage);
    copy::prepare(f.base.request(), &f.base.temp.path().join("valid-clock"))?;
    Ok(())
}

#[test]
fn project_derivation_declaration_rejects_current_hash_for_old_scope_and_blocked_confirmation(
) -> Result<()> {
    let f = Fixture::with_declarations()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let snapshot = object_tasks::snapshot(&runtime, "original")?;
    let mut value = serde_json::to_value(receipt(&runtime, "revised-root")?)?;
    value["request"]["expectedScopeHash"] = json!(state(&snapshot, "root").scope_hash);
    runtime.store().lock().unwrap().put(HEAD, "root", &value)?;
    runtime
        .store()
        .lock()
        .unwrap()
        .put(RECEIPT, "revised-root", &value)?;
    drop(runtime);
    rejected(&f, "hash silently made current", "DECLARATION_SCOPE_HASH")?;
    for task in ["discard", "locked-task", "shape"] {
        let f = Fixture::with_declarations()?;
        let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
        let tasks = object_tasks::snapshot(&runtime, "original")?.tasks;
        let root = tasks.iter().find(|t| t.id == task).unwrap();
        let projected = declaration::scope_state(&tasks, root)?;
        let mut value = serde_json::to_value(receipt(&runtime, "early-root")?)?;
        value["request"]["taskId"] = json!(task);
        value["request"]["requestId"] = json!("forged");
        value["request"]["expectedPlanRevision"] = json!(4);
        value["request"]["expectedScopeHash"] = json!(projected.scope_hash);
        value["taskIds"] = json!(declaration::scope(&tasks, root)
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>());
        runtime.store().lock().unwrap().put(HEAD, task, &value)?;
        runtime
            .store()
            .lock()
            .unwrap()
            .put(RECEIPT, "forged", &value)?;
        drop(runtime);
        rejected(
            &f,
            task,
            if task == "shape" {
                "PARENT_REQUIRED"
            } else {
                "DECLARATION_INCOMPLETE"
            },
        )?;
    }
    Ok(())
}
