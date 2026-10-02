use super::*;
use std::fs;

#[test]
fn project_derivation_gate_anchors_each_historical_status_to_its_attempt() -> Result<()> {
    for (id, status) in [
        ("paused-verify", "awaitingAcceptance"),
        ("retry-verify-0", "awaitingAcceptance"),
        ("terminal-verify", "failed"),
    ] {
        for owner in ["medium", "run", "fine", "queue"] {
            let f = RetryFixture::with_terminal("empty", State::AwaitingGate)?;
            f.verification(id, |saved| {
                let field = if owner == "queue" { "state" } else { "status" };
                saved["records"][owner][field] = json!(status);
            })?;
            f.first.rejected(
                &format!("forged-gate-status-{id}-{owner}"),
                if id == "retry-verify-0" && owner == "fine" {
                    // A consumed verifier must also match its immutable successor's fine snapshot.
                    "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH"
                } else if owner == "queue" {
                    "DERIVATION_RECOVERY_QUEUE_SNAPSHOT_MISMATCH"
                } else {
                    "DERIVATION_RECOVERY_TASK_SNAPSHOT_MISMATCH"
                },
            )?;
        }
    }
    Ok(())
}

#[test]
fn project_derivation_gate_rejects_missing_output_and_candidate_drift() -> Result<()> {
    for change in ["missing-output", "workspace-drift", "missing-blob"] {
        let f = ExecutionFixture::new(State::AwaitingGate, "empty")?;
        let expected = match change {
            "missing-output" => {
                f.mutate("object_attempt", &f.attempt.id, |a| {
                    a["output"] = Value::Null
                })?;
                "OBJECT_ATTEMPT_STATE_MISMATCH"
            }
            "workspace-drift" => {
                fs::write(f.workspace().join("partial.txt"), "changed candidate")?;
                "WORKSPACE_DRIFT"
            }
            _ => {
                let hash = &f.attempt.output.as_ref().unwrap()["partial.txt"];
                fs::remove_file(f.base.source.join(".beaver/content/blobs").join(hash))?;
                "BLOB_MISSING"
            }
        };
        f.rejected(change, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_gate_does_not_admit_gate_predecessors() -> Result<()> {
    let f = RetryFixture::with_terminal("empty", State::AwaitingGate)?;
    f.first.mutate("object_attempt", &f.attempts[1].id, |a| {
        a["state"] = json!("awaitingGate");
    })?;
    f.first
        .rejected("gate-before-retry", "OBJECT_RECOVERY_HISTORY_MISMATCH")
}
