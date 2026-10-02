use super::*;

#[path = "project_derivation_dispatch_rejection_tests.rs"]
mod dispatch_history;

fn rejected(f: &QueueFixture, label: &str, expected: &str) -> Result<()> {
    let before = data_backup::inventory(&f.base.source)?;
    let destination = f.base.temp.path().join("rejected-queue");
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

fn corrupt(
    f: &QueueFixture,
    kind: &str,
    id: &str,
    pointer: &str,
    replacement: Value,
    expected: &str,
) -> Result<()> {
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let mut value: Value = storage.store().get(kind, id)?.unwrap();
    *value.pointer_mut(pointer).unwrap() = replacement;
    storage.store().put(kind, id, &value)?;
    drop(storage);
    rejected(f, &format!("{kind}/{id}{pointer}"), expected)
}

#[test]
fn project_derivation_queue_rejects_ownership_state_and_identity_before_copy() -> Result<()> {
    for (pointer, value, expected) in [
        ("/owner", json!("worker"), "EXECUTED_ENTRY"),
        ("/claimToken", json!("lease"), "EXECUTED_ENTRY"),
        ("/generation", json!(1), "EXECUTED_ENTRY"),
        ("/state", json!("claimed"), "EXECUTED_ENTRY"),
        ("/state", json!("future"), "EXECUTED_ENTRY"),
        ("/state", json!("cancelled"), "STATE_MISMATCH"),
        ("/projectId", json!("other"), "IDENTITY_MISMATCH"),
        ("/taskId", json!("missing"), "IDENTITY_MISMATCH"),
        ("/id", json!("other:build"), "IDENTITY_MISMATCH"),
    ] {
        let f = QueueFixture::new()?;
        corrupt(
            &f,
            "object_task_queue",
            "original:build",
            pointer,
            value,
            expected,
        )?;
    }
    let f = QueueFixture::new()?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let value: Value = storage
        .store()
        .get("object_task_queue", "original:build")?
        .unwrap();
    storage
        .store()
        .remove("object_task_queue", "original:build")?;
    storage
        .store()
        .put("object_task_queue", "wrong-key", &value)?;
    drop(storage);
    rejected(&f, "queue key", "IDENTITY_MISMATCH")?;
    let f = QueueFixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let claimed = preparation::claim_next(&runtime, "original", "scheduler-negative")?.unwrap();
    assert_eq!(claimed.record().medium.id, "build");
    assert!(preparation::get(&runtime, "original", &claimed.record().id)?.is_some());
    drop(claimed);
    drop(runtime);
    rejected(
        &f,
        "real preparation claim",
        "DERIVATION_EXECUTION_ATTEMPT_REQUIRED",
    )
}

#[test]
fn project_derivation_queue_rejects_damaged_reorder_history_before_copy() -> Result<()> {
    for (pointer, value, expected) in [
        ("/request/projectId", json!("other"), "RECEIPT_MISMATCH"),
        ("/result/projectId", json!("other"), "RECEIPT_MISMATCH"),
        ("/request/requestId", json!("other"), "RECEIPT_MISMATCH"),
        (
            "/request/expectedVersion",
            json!("invalid"),
            "RECEIPT_MISMATCH",
        ),
        ("/result/version", json!("A".repeat(64)), "RECEIPT_MISMATCH"),
        ("/request/nextTaskId", json!("missing"), "ANCHOR_CONFLICT"),
        ("/request/taskId", json!("missing"), "NOT_REORDERABLE"),
        ("/result/items/0/taskId", json!("missing"), "TASK_MISSING"),
        (
            "/result/items/0/objectId",
            json!("missing"),
            "HISTORY_IDENTITY_MISMATCH",
        ),
        (
            "/result/items/0/title",
            json!(" "),
            "HISTORY_IDENTITY_MISMATCH",
        ),
        (
            "/result/items/0/state",
            json!("claimed"),
            "HISTORY_IDENTITY_MISMATCH",
        ),
        (
            "/result/items/0/state",
            json!("cancelled"),
            "HISTORY_IDENTITY_MISMATCH",
        ),
        (
            "/result/items/0/blockers",
            json!(["future"]),
            "BLOCKERS_MISMATCH",
        ),
        (
            "/result/items/0/blockers",
            json!(["paused", "paused"]),
            "BLOCKERS_MISMATCH",
        ),
        (
            "/result/items/0/blockers",
            json!(["dependencies", "paused"]),
            "BLOCKERS_MISMATCH",
        ),
        ("/result/items/2/blockers", json!([]), "BLOCKERS_MISMATCH"),
    ] {
        let f = QueueFixture::new()?;
        let key = crate::project_derivation_queue_records::receipt_key("original", "move-free")?;
        corrupt(
            &f,
            "object_task_queue_reorder_receipt",
            &key,
            pointer,
            value,
            expected,
        )?;
    }
    for change in [
        "equal-token",
        "wrong-key",
        "missing-entry",
        "missing-order",
        "wrong-order",
        "order-gap",
    ] {
        let f = QueueFixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let kind = "object_task_queue_reorder_receipt";
        let key = crate::project_derivation_queue_records::receipt_key("original", "move-free")?;
        let mut receipt: Value = storage.store().get(kind, &key)?.unwrap();
        match change {
            "equal-token" => {
                receipt["result"]["version"] = receipt["request"]["expectedVersion"].clone();
                storage.store().put(kind, &key, &receipt)?;
            }
            "wrong-key" => {
                storage.store().remove(kind, &key)?;
                storage.store().put(kind, "wrong", &receipt)?;
            }
            "missing-entry" => storage
                .store()
                .remove("object_task_queue", "original:free")?,
            "missing-order" => storage.store().remove(view::ORDER_KIND, "original")?,
            "wrong-order" => {
                storage.store().remove(view::ORDER_KIND, "original")?;
                storage.store().put(view::ORDER_KIND, "other", &1_u64)?;
            }
            _ => storage.store().put(view::ORDER_KIND, "original", &2_u64)?,
        }
        drop(storage);
        rejected(
            &f,
            change,
            match change {
                "missing-entry" => "HISTORY_ENTRY_MISSING",
                "missing-order" | "order-gap" => "ORDER_HISTORY_GAP",
                "wrong-order" => "ORDER_MISMATCH",
                _ => "RECEIPT_MISMATCH",
            },
        )?;
    }
    Ok(())
}

#[test]
fn project_derivation_queue_rejects_unknown_queue_and_dispatch_fields() -> Result<()> {
    for kind in [
        "object_task_queue",
        "object_task_queue_reorder_receipt",
        "object_task_dispatch_control",
        "object_task_dispatch_receipt",
        "object_task_coarse_dispatch_control",
        "object_task_coarse_dispatch_receipt",
    ] {
        let f = QueueFixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let (id, mut value) = storage.store().list_with_ids::<Value>(kind)?.remove(0);
        value["futureField"] = json!("must not be lost");
        storage.store().put(kind, &id, &value)?;
        drop(storage);
        rejected(&f, kind, "unknown field")?;
    }
    for kind in [
        "object_task_dispatch_control",
        "object_task_dispatch_receipt",
        "object_task_coarse_dispatch_control",
        "object_task_coarse_dispatch_receipt",
    ] {
        let f = QueueFixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage.store().put(kind, "malformed", &json!({}))?;
        drop(storage);
        rejected(&f, kind, "missing field")?;
    }
    Ok(())
}
