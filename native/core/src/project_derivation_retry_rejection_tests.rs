use super::*;
use std::fs;

#[test]
fn project_derivation_retry_rejects_damaged_historical_snapshots_before_copying() -> Result<()> {
    for (pointer, value, expected) in [
        ("/records/history", json!([]), "PREFIX_MISMATCH"),
        (
            "/records/history/0/error",
            json!("forged history"),
            "PREFIX_MISMATCH",
        ),
        (
            "/records/attempt/threadId",
            json!("forged thread"),
            "PREFIX_MISMATCH",
        ),
        (
            "/records/medium/title",
            json!("forged medium"),
            "TASK_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/run/baselineVersionId",
            json!("forged-version"),
            "TASK_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/fine/prompt",
            json!("forged fine"),
            "TASK_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/queue/enqueuedAt",
            json!(99999),
            "QUEUE_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/queue/generation",
            json!(2),
            "QUEUE_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/queue/owner",
            json!("forged-owner"),
            "QUEUE_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/queue/claimToken",
            json!("forged-token"),
            "QUEUE_SNAPSHOT_MISMATCH",
        ),
        (
            "/records/object/name",
            json!("forged object"),
            "OBJECT_RECEIPT_PROJECTION_MISMATCH",
        ),
        (
            "/records/baselineSources",
            json!([]),
            "BASELINE_SOURCES_MISMATCH",
        ),
        (
            "/records/baselineSources/0/name",
            json!("forged baseline source"),
            "OBJECT_RECEIPT_PROJECTION_MISMATCH",
        ),
        (
            "/records/interrupts/0/result/error",
            json!("forged interrupt"),
            "INTERRUPT_SNAPSHOT_MISMATCH",
        ),
    ] {
        let f = RetryFixture::new("latest")?;
        f.verification("terminal-verify", |saved| {
            *saved.pointer_mut(pointer).unwrap() = value.clone()
        })?;
        f.first.rejected("snapshot-corruption", expected)?;
    }
    for change in [
        "history-order",
        "baseline-order",
        "baseline-duplicate",
        "old-object",
        "control",
    ] {
        let f = RetryFixture::new("latest")?;
        let id = if change == "old-object" {
            "retry-verify-0"
        } else {
            "terminal-verify"
        };
        f.verification(id, |saved| {
            let records = &mut saved["records"];
            match change {
                "history-order" => records["history"].as_array_mut().unwrap().reverse(),
                "baseline-order" => records["baselineSources"].as_array_mut().unwrap().reverse(),
                "baseline-duplicate" => {
                    let sources = records["baselineSources"].as_array_mut().unwrap();
                    sources.push(sources[0].clone());
                }
                "old-object" => records["object"]["name"] = json!("forged historical metadata"),
                _ => {
                    records["control"]["revision"] = json!(999);
                    saved["operation"]["request"]["target"]["controlRevision"] = json!(999);
                }
            }
        })?;
        let expected = match change {
            "history-order" => "PREFIX_MISMATCH",
            "old-object" => "OBJECT_HISTORY_MISMATCH",
            "control" => "CONTROL_HISTORY_MISMATCH",
            _ => "BASELINE_SOURCES_MISMATCH",
        };
        f.first.rejected(change, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_retry_rejects_pending_or_orphan_journals_and_incomplete_edges() -> Result<()>
{
    for change in [
        "pending-verify",
        "pending-resume",
        "nested-verify",
        "wrong-head",
        "wrong-resume-head",
        "missing-edge",
        "wrong-edge",
        "orphan-edge",
        "orphan-head",
        "wrong-business-key",
    ] {
        let f = RetryFixture::new("empty")?;
        let storage = ProjectStore::open(&f.first.base.source, "original")?;
        let expected = match change {
            "pending-verify" | "wrong-business-key" => {
                let key = RetryFixture::key("terminal-verify")?;
                let mut saved: Value = storage
                    .store()
                    .get("object_recovery_verification", &key)?
                    .unwrap();
                if change == "pending-verify" {
                    saved["operation"]["result"] = Value::Null;
                } else {
                    storage
                        .store()
                        .remove("object_recovery_verification", &key)?;
                }
                storage.store().put(
                    "object_recovery_verification",
                    if change == "pending-verify" {
                        &key
                    } else {
                        "stale-project-hash"
                    },
                    &saved,
                )?;
                "VERIFICATION_INCOMPLETE"
            }
            "pending-resume" | "nested-verify" => {
                let key = RetryFixture::key("blocked-resume")?;
                let mut saved: Value = storage
                    .store()
                    .get("object_recovery_resume", &key)?
                    .unwrap();
                if change == "pending-resume" {
                    saved["operation"]["result"] = Value::Null;
                } else {
                    saved["verification"]["records"]["queue"]["enqueuedAt"] = json!(99999);
                }
                storage
                    .store()
                    .put("object_recovery_resume", &key, &saved)?;
                if change == "pending-resume" {
                    "RESUME_INCOMPLETE"
                } else {
                    "RESUME_RECEIPT_MISMATCH"
                }
            }
            "wrong-head" | "orphan-head" => {
                let run = &f.first.attempt.preparation.run.id;
                let mut head: Value = storage.store().get("object_recovery_head", run)?.unwrap();
                if change == "wrong-head" {
                    head["requestId"] = json!("paused-verify");
                }
                storage.store().put(
                    "object_recovery_head",
                    if change == "wrong-head" {
                        run
                    } else {
                        "orphan"
                    },
                    &head,
                )?;
                if change == "wrong-head" {
                    "HEAD_MISMATCH"
                } else {
                    "ORPHAN_HEAD"
                }
            }
            "wrong-resume-head" => {
                storage
                    .store()
                    .put("object_recovery_resume_head", "build", &"blocked-resume")?;
                "RESUME_HEAD_MISMATCH"
            }
            "missing-edge" => {
                storage
                    .store()
                    .remove("object_recovery_attempt_successor", &f.attempts[0].id)?;
                "OBJECT_RECOVERY_HISTORY_MISMATCH"
            }
            "wrong-edge" => {
                storage.store().put(
                    "object_recovery_attempt_successor",
                    &f.attempts[0].id,
                    &"blocked-resume",
                )?;
                "OBJECT_RECOVERY_HISTORY_MISMATCH"
            }
            _ => {
                storage.store().put(
                    "object_recovery_attempt_successor",
                    "orphan",
                    &"retry-resume-0",
                )?;
                "ORPHAN_SUCCESSOR_OR_HEAD"
            }
        };
        drop(storage);
        f.first.rejected(change, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_retry_rejects_forged_purposes_and_missing_old_content() -> Result<()> {
    for (pointer, value, expected) in [
        (
            "/operation/request/advance",
            json!({"attemptId":"old","checkRequestId":"check","nextFineTaskId":"next","nextFineRevision":0,"acceptanceNote":"advance"}),
            "OBJECT_STAGE_RECEIPT_MISMATCH",
        ),
        (
            "/operation/request/rework",
            json!({"reviewRequestId":"review","attemptId":"old","fineTaskId":"fine","feedback":"rework"}),
            "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH",
        ),
        ("/freshChecks", json!([]), "OBJECT_STAGE_RECEIPT_MISMATCH"),
    ] {
        let f = RetryFixture::new("empty")?;
        f.first.mutate(
            "object_recovery_resume",
            &RetryFixture::key("blocked-resume")?,
            |saved| {
                // Optional fields are absent in the plain retry serialization.
                let (parent, field) = pointer.rsplit_once('/').unwrap();
                saved.pointer_mut(parent).unwrap()[field] = value;
            },
        )?;
        f.first.rejected("forged-retry-purpose", expected)?;
    }
    let f = RetryFixture::new("empty")?;
    f.first.mutate(
        "object_recovery_resume",
        &RetryFixture::key("blocked-resume")?,
        |saved| {
            saved["next"] = serde_json::to_value(&f.first.attempt.fine).unwrap();
        },
    )?;
    f.first
        .rejected("next-fine", "OBJECT_STAGE_RECEIPT_MISMATCH")?;
    let f = RetryFixture::new("empty")?;
    let hash = &f.attempts[0].output.as_ref().unwrap()["partial.txt"];
    assert_ne!(
        Some(hash),
        f.attempts
            .last()
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .get("partial.txt")
    );
    fs::remove_file(f.first.base.source.join(".beaver/content/blobs").join(hash))?;
    f.first.rejected("missing-historical-blob", "BLOB_MISSING")
}
