use super::*;

#[test]
fn project_derivation_queue_rejects_damaged_dispatch_commands_and_results() -> Result<()> {
    for coarse in [false, true] {
        for (pointer, replacement, expected) in [
            ("/request/projectId", json!("other"), "RECEIPT_MISMATCH"),
            ("/request/requestId", json!("other"), "COMMAND_MISMATCH"),
            ("/request/taskId", json!("missing"), "TASK_MISSING"),
            (
                "/request/expectedTaskRevision",
                json!(999),
                "COMMAND_MISMATCH",
            ),
            (
                "/request/expectedControlRevision",
                json!(999),
                "RECEIPT_MISMATCH",
            ),
            ("/request/paused", json!(true), "RECEIPT_MISMATCH"),
            ("/result/schemaVersion", json!(2), "IDENTITY_MISMATCH"),
            ("/result/projectId", json!("other"), "IDENTITY_MISMATCH"),
            ("/result/taskId", json!("missing"), "TASK_MISSING"),
            ("/result/paused", json!(true), "RECEIPT_MISMATCH"),
            ("/result/revision", json!(3), "RECEIPT_MISMATCH"),
        ] {
            let f = QueueFixture::new()?;
            let (kind, request) = if coarse {
                ("object_task_coarse_dispatch_receipt", "resume-root")
            } else {
                ("object_task_dispatch_receipt", "resume-build")
            };
            let key = crate::project_derivation_queue_records::receipt_key("original", request)?;
            corrupt(&f, kind, &key, pointer, replacement, expected)?;
        }
    }
    for pointer in [
        "/request/objectId",
        "/request/runId",
        "/result/objectId",
        "/result/runId",
    ] {
        let f = QueueFixture::new()?;
        let key = crate::project_derivation_queue_records::receipt_key("original", "resume-build")?;
        corrupt(
            &f,
            "object_task_dispatch_receipt",
            &key,
            pointer,
            json!("missing"),
            if pointer.starts_with("/request") {
                "RECEIPT_MISMATCH"
            } else if pointer.ends_with("runId") {
                "RUN_MISSING"
            } else {
                "IDENTITY_MISMATCH"
            },
        )?;
    }
    let f = QueueFixture::new()?;
    let key = crate::project_derivation_queue_records::receipt_key("original", "pause-build")?;
    corrupt(
        &f,
        "object_task_dispatch_receipt",
        &key,
        "/request/expectedTaskRevision",
        json!(1),
        "TASK_REVISION_REVERSED",
    )?;
    Ok(())
}

#[test]
fn project_derivation_queue_rejects_dispatch_gaps_duplicates_heads_and_orphans() -> Result<()> {
    for coarse in [false, true] {
        for change in [
            "missing-receipt",
            "missing-control",
            "wrong-key",
            "duplicate",
            "wrong-head",
            "revision-gap",
        ] {
            let f = QueueFixture::new()?;
            let storage = ProjectStore::open(&f.base.source, "original")?;
            let (control_kind, receipt_kind, request, control_id) = if coarse {
                (
                    "object_task_coarse_dispatch_control",
                    "object_task_coarse_dispatch_receipt",
                    "resume-root",
                    "root".into(),
                )
            } else {
                (
                    "object_task_dispatch_control",
                    "object_task_dispatch_receipt",
                    "resume-build",
                    f.unpaused.result.run_id.clone(),
                )
            };
            let key = crate::project_derivation_queue_records::receipt_key("original", request)?;
            match change {
                "missing-receipt" => storage.store().remove(receipt_kind, &key)?,
                "missing-control" => storage.store().remove(control_kind, &control_id)?,
                "wrong-key" => {
                    let control: Value = storage.store().get(control_kind, &control_id)?.unwrap();
                    storage.store().remove(control_kind, &control_id)?;
                    storage.store().put(control_kind, "wrong", &control)?;
                }
                "duplicate" => {
                    let mut receipt: Value = storage.store().get(receipt_kind, &key)?.unwrap();
                    receipt["request"]["requestId"] = json!("duplicate-command");
                    let duplicate = crate::project_derivation_queue_records::receipt_key(
                        "original",
                        "duplicate-command",
                    )?;
                    storage.store().put(receipt_kind, &duplicate, &receipt)?;
                }
                _ => {
                    let mut control: Value =
                        storage.store().get(control_kind, &control_id)?.unwrap();
                    if change == "wrong-head" {
                        control["paused"] = json!(true);
                    } else {
                        control["revision"] = json!(3);
                    }
                    storage.store().put(control_kind, &control_id, &control)?;
                }
            }
            drop(storage);
            rejected(
                &f,
                change,
                match change {
                    "missing-control" => "CONTROL_MISSING",
                    "wrong-key" => "KEY_MISMATCH",
                    "duplicate" => "DUPLICATE_REVISION",
                    _ => "HISTORY_HEAD_MISMATCH",
                },
            )?;
        }
    }
    Ok(())
}
