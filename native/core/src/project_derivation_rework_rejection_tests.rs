use super::*;

#[test]
fn project_derivation_rework_rejects_incomplete_and_forged_receipts() -> Result<()> {
    for (request, pointer, value, expected) in [
        (
            "z-rework-true",
            "/operation/result",
            Value::Null,
            "DERIVATION_RECOVERY_RESUME_INCOMPLETE",
        ),
        (
            "z-rework-true",
            "/next/prompt",
            json!("forged prompt"),
            "OBJECT_CANDIDATE_REWORK_RECEIPT_MISMATCH",
        ),
        (
            "z-rework-true",
            "/operation/request/rework/feedback",
            json!("forged feedback"),
            "OBJECT_CANDIDATE_REWORK_RECEIPT_MISMATCH",
        ),
        (
            "z-rework-true",
            "/operation/request/rework/reviewRequestId",
            json!("forged-review"),
            "OBJECT_CANDIDATE_REWORK_RECEIPT_MISMATCH",
        ),
        (
            "z-rework-true",
            "/operation/request/rework/attemptId",
            json!("forged-attempt"),
            "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH",
        ),
        (
            "z-rework-false",
            "/started/fine/prompt",
            json!("forged started prompt"),
            "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH",
        ),
        (
            "z-rework-true",
            "/verification/records/history",
            json!([]),
            "OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH",
        ),
    ] {
        let f = ReworkFixture::new(false, 2, State::Failed, 2)?;
        f.candidate.execution.mutate(
            "object_recovery_resume",
            &RetryFixture::key(request)?,
            |saved| {
                *saved.pointer_mut(pointer).unwrap() = value;
            },
        )?;
        f.candidate
            .execution
            .rejected("forged-rework-receipt", expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_rework_rejects_attachment_boundaries_before_copy() -> Result<()> {
    // These are deliberate journal corruptions, not claims of supported image migration.
    // A blocked historical request must not bypass the text-only boundary either.
    for attachment in [
        json!({"image":{"path":"image.png","sha256":"0".repeat(64),"width":1,"height":1,
            "regions":[{"x":0.0,"y":0.0,"width":1.0,"height":1.0,"prompt":"region"}]}}),
        json!({"previewFrame":{"runId":"run","frameId":"frame"}}),
        json!({"previewFrame":{"runId":"run","frameId":"frame"},"relocation":{
            "sourceAttemptId":"attempt","sourceFrame":{"runId":"run","frameId":"frame"},
            "regions":[{"status":"matched","sourceRegion":0,"targetRegion":0}],"confirmed":true}}),
    ] {
        let f = ReworkFixture::new(false, 1, State::Failed, 2)?;
        f.candidate.execution.mutate(
            "object_recovery_resume",
            &RetryFixture::key("z-rework-true")?,
            |saved| {
                saved["operation"]["request"]["rework"]
                    .as_object_mut()
                    .unwrap()
                    .extend(attachment.as_object().unwrap().clone());
            },
        )?;
        f.candidate
            .execution
            .rejected("nontext-rework", "DERIVATION_REWORK_TEXT_ONLY")?;
    }
    Ok(())
}

#[test]
fn project_derivation_rework_rejects_missing_receipt_or_successor() -> Result<()> {
    for receipt in [true, false] {
        let f = ReworkFixture::new(true, 2, State::AwaitingGate, 2)?;
        let store = ProjectStore::open(&f.candidate.execution.base.source, "original")?;
        if receipt {
            store.store().remove(
                "object_recovery_resume",
                &RetryFixture::key("z-rework-false")?,
            )?;
        } else {
            store.store().remove(
                "object_recovery_attempt_successor",
                &f.candidate.terminal.id,
            )?;
        }
        drop(store);
        f.candidate.execution.rejected(
            "incomplete-rework-chain",
            if receipt {
                "OBJECT_RECOVERY_HISTORY_MISSING"
            } else {
                "OBJECT_RECOVERY_HISTORY_MISMATCH"
            },
        )?;
    }
    Ok(())
}

#[test]
fn project_derivation_rework_rejects_forged_historical_review() -> Result<()> {
    for (pointer, expected) in [
        (
            "/source/records/history",
            "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH",
        ),
        ("/source/fines", "OBJECT_CANDIDATE_REPORT_MISMATCH"),
    ] {
        let f = ReworkFixture::new(true, 2, State::AwaitingGate, 2)?;
        f.candidate
            .mutate(|saved| *saved.pointer_mut(pointer).unwrap() = json!([]))?;
        f.candidate
            .execution
            .rejected("forged-rework-review", expected)?;
    }
    Ok(())
}
