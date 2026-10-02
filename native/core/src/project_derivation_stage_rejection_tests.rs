use super::*;
use std::fs;

#[test]
fn project_derivation_stage_rejects_forged_selection_and_authority() -> Result<()> {
    for (pointer, value, expected) in [
        (
            "/operation/request/advance/checkRequestId",
            json!("missing"),
            "OBJECT_STAGE_PASSING_REPORT_REQUIRED",
        ),
        (
            "/operation/request/advance/checkRequestId",
            json!("check"),
            "OBJECT_CHECK_REQUEST_CONFLICT",
        ),
        (
            "/operation/request/advance/acceptanceNote",
            json!(" "),
            "INVALID_OBJECT_STAGE_APPROVAL",
        ),
        (
            "/next/title",
            json!("invented"),
            "DERIVATION_STAGE_SUCCESSOR_SNAPSHOT_MISMATCH",
        ),
        (
            "/next/dependsOn",
            json!(["final-fine"]),
            "DERIVATION_STAGE_SUCCESSOR_SNAPSHOT_MISMATCH",
        ),
        ("/freshChecks", json!([]), "OBJECT_STAGE_RECEIPT_MISMATCH"),
        (
            "/freshChecks/0/version",
            json!(9),
            "OBJECT_STAGE_RECEIPT_MISMATCH",
        ),
    ] {
        let f = StageFixture::new(1, State::Failed)?;
        f.retry.first.mutate(
            "object_recovery_resume",
            &RetryFixture::key("stage-advance-0-true")?,
            |saved| {
                *saved.pointer_mut(pointer).unwrap() = value;
            },
        )?;
        f.retry.first.rejected("forged-stage", expected)?;
    }
    let f = StageFixture::new(1, State::Failed)?;
    let storage = ProjectStore::open(&f.retry.first.base.source, "original")?;
    let final_fine: Value = storage.store().get("object_task", "final-fine")?.unwrap();
    drop(storage);
    // A self-consistent blocked receipt still cannot skip the original adjacent stage.
    f.retry.first.mutate(
        "object_recovery_resume",
        &RetryFixture::key("stage-advance-0-true")?,
        |saved| {
            saved["next"] = final_fine;
            saved["operation"]["request"]["advance"]["nextFineTaskId"] = json!("final-fine");
        },
    )?;
    f.retry
        .first
        .rejected("stage-skip", "OBJECT_STAGE_SUCCESSOR_MISMATCH")
}

#[test]
fn project_derivation_stage_rejects_accepted_drift_ambiguous_positions_and_pending_operations(
) -> Result<()> {
    for mode in [
        "accepted-title",
        "accepted-revision",
        "position",
        "pending",
        "edge",
        "fresh-check",
    ] {
        let f = StageFixture::new(1, State::Failed)?;
        let first = &f.retry.first;
        let expected = match mode {
            "accepted-title" => {
                first.mutate("object_task", "fine", |t| {
                    t["title"] = json!("Changed accepted stage")
                })?;
                "OBJECT_STAGE_ACCEPTED_RECORD_MISMATCH"
            }
            "accepted-revision" => {
                first.mutate("object_task", "fine", |t| t["revision"] = json!(99))?;
                "OBJECT_STAGE_ACCEPTED_RECORD_MISMATCH"
            }
            "position" => {
                first.mutate("object_task", "final-fine", |t| t["position"] = json!(1))?;
                "DERIVATION_EXECUTION_FINE_POSITION_AMBIGUOUS"
            }
            "pending" => {
                first.mutate(
                    "object_recovery_resume",
                    &RetryFixture::key("stage-advance-0-true")?,
                    |s| {
                        s["operation"]["result"] = Value::Null;
                        s["freshChecks"] = Value::Null;
                    },
                )?;
                "DERIVATION_RECOVERY_RESUME_INCOMPLETE"
            }
            "edge" => {
                first.mutate(
                    "object_recovery_attempt_successor",
                    &f.retry.attempts[2].id,
                    |s| *s = json!("missing"),
                )?;
                "OBJECT_RECOVERY_HISTORY_MISSING"
            }
            _ => {
                first.mutate(
                    "object_recovery_resume",
                    &RetryFixture::key("stage-advance-0-false")?,
                    |s| {
                        s["freshChecks"][0]["passed"] = json!(false);
                    },
                )?;
                "OBJECT_STAGE_RECEIPT_MISMATCH"
            }
        };
        first.rejected(mode, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_stage_inventory_uses_chain_tail_and_keeps_old_blobs_required() -> Result<()> {
    for old_blob in [false, true] {
        let f = StageFixture::new(1, State::Failed)?;
        let first = &f.retry.first;
        if old_blob {
            let hash = &first.attempt.output.as_ref().unwrap()["partial.txt"];
            fs::remove_file(first.base.source.join(".beaver/content/blobs").join(hash))?;
        } else {
            fs::write(first.workspace().join("partial.txt"), "retry output 1")?;
        }
        first.rejected(
            "stage-inventory",
            if old_blob {
                "BLOB_MISSING"
            } else {
                "WORKSPACE_DRIFT"
            },
        )?;
    }
    Ok(())
}
