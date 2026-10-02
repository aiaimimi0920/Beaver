use super::*;

#[test]
fn project_derivation_execution_rejects_unfinished_or_unsupported_boundaries() -> Result<()> {
    {
        let f = ExecutionFixture::new(State::Failed, "empty")?;
        f.mutate("object_attempt", &f.attempt.id, |a| {
            a["state"] = json!("running");
            a["output"] = Value::Null;
        })?;
        for (kind, id) in [
            ("object_task", "build"),
            ("object_task", "fine"),
            ("object_run", f.attempt.preparation.run.id.as_str()),
        ] {
            f.mutate(kind, id, |t| {
                t["status"] = json!("running");
                t["revision"] = json!(t["revision"].as_u64().unwrap() - 1);
            })?;
        }
        f.mutate("object_task_queue", "original:build", |q| {
            q["state"] = json!("running")
        })?;
        f.rejected("unfinished", "FIRST_STOPPED_ATTEMPT_REQUIRED")?;
    }
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    storage.store().remove("object_attempt", &f.attempt.id)?;
    storage
        .store()
        .put("object_task", "build", &f.attempt.preparation.medium)?;
    storage.store().put(
        "object_run",
        &f.attempt.preparation.run.id,
        &f.attempt.preparation.run,
    )?;
    drop(storage);
    f.mutate("object_task_queue", "original:build", |q| {
        q["state"] = json!("claimed")
    })?;
    f.rejected("no-attempt", "ATTEMPT_REQUIRED")?;
    for kind in [
        "object_recovery_disposition",
        "object_recovery_disposition_head",
        "object_publication",
        "object_publication_deferred",
        "object_publication_followup",
    ] {
        let f = ExecutionFixture::new(State::Failed, "empty")?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage
            .store()
            .put(kind, "unsupported", &json!({"projectId":"original"}))?;
        drop(storage);
        f.rejected("recovery-side-record", "UNKNOWN_ENTITY_KIND")?;
    }
    for (kind, expected) in [
        ("object_candidate_review", "unknown field `projectId`"),
        ("object_recovery_verification", "unknown field `projectId`"),
        ("object_recovery_head", "missing field"),
        ("object_recovery_resume", "unknown field `projectId`"),
        ("object_recovery_resume_head", "invalid type"),
        ("object_recovery_attempt_successor", "invalid type"),
    ] {
        let f = ExecutionFixture::new(State::Failed, "empty")?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        storage
            .store()
            .put(kind, "malformed", &json!({"projectId":"original"}))?;
        drop(storage);
        f.rejected("malformed-recovery-record", expected)?;
    }
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let mut other = f.attempt.clone();
    other.id = "second-attempt".into();
    storage.store().put("object_attempt", &other.id, &other)?;
    drop(storage);
    f.rejected("extra-attempt", "OBJECT_RECOVERY_HISTORY_MISMATCH")
}

#[test]
fn project_derivation_execution_rejects_frozen_baseline_and_claim_drift() -> Result<()> {
    for (pointer, value, expected) in [
        ("/generation", json!(2), "OBJECT_TASK_STALE_CLAIM"),
        ("/owner", json!("other-worker"), "OBJECT_TASK_STALE_CLAIM"),
        (
            "/claimToken",
            json!("other-token"),
            "OBJECT_TASK_STALE_CLAIM",
        ),
        (
            "/workspace",
            json!(".beaver/workspaces/other"),
            "OBJECT_RUN_WORKSPACE_MISMATCH",
        ),
        (
            "/baseline/contentDigest",
            json!("0".repeat(64)),
            "BASELINE_DIGEST_MISMATCH",
        ),
        (
            "/baseline/acceptedVersionIdAtClaim",
            Value::Null,
            "CLAIM_ACCEPTANCE_MISMATCH",
        ),
        (
            "/baseline/objectRevisionAtClaim",
            json!(999),
            "CLAIM_REVISION_INVALID",
        ),
        (
            "/baseline/objectRevisionAtClaim",
            json!(0),
            "CLAIM_ACCEPTANCE_MISMATCH",
        ),
    ] {
        let f = ExecutionFixture::new(State::Failed, "latest")?;
        f.preparation(pointer, value)?;
        f.rejected("frozen-corruption", expected)?;
    }
    let f = ExecutionFixture::new(State::Failed, "latest")?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let key = crate::project_derivation_queue_records::receipt_key("original", "update-child")?;
    let receipt: Value = storage
        .store()
        .get("object_command_receipt", &key)?
        .unwrap();
    let revision = receipt["result"]["object"]["revision"].clone();
    storage.store().remove("object_command_receipt", &key)?;
    drop(storage);
    f.preparation("/baseline/objectRevisionAtClaim", revision)?;
    f.rejected("missing-claim-history", "CLAIM_HISTORY_MISSING")?;
    let f = ExecutionFixture::new(State::Failed, "latest")?;
    f.mutate("object_attempt", &f.attempt.id, |a| a["input"] = json!({}))?;
    f.rejected("wrong-input", "OBJECT_RECOVERY_INPUT_MISMATCH")?;
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    f.mutate("object_task", "later-fine", |t| t["position"] = json!(0))?;
    f.rejected("tied-fine", "FINE_POSITION_AMBIGUOUS")?;
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    f.mutate("object_task", "fine", |t| t["position"] = json!(2))?;
    f.mutate("object_attempt", &f.attempt.id, |a| {
        a["fine"]["position"] = json!(2)
    })?;
    f.mutate("object_task", "later-fine", |t| t["dependsOn"] = json!([]))?;
    f.rejected("wrong-first-fine", "FIRST_FINE_MISMATCH")
}

#[test]
fn project_derivation_execution_rejects_orphan_and_damaged_evidence() -> Result<()> {
    for (kind, source_id, field, value, expected) in [
        (
            "object_attempt_check_report",
            "check",
            "attemptDigest",
            json!("0".repeat(64)),
            "OBJECT_CHECK_REPORT_MISMATCH",
        ),
        (
            "object_attempt_check_report",
            "check",
            "futureField",
            json!(true),
            "unknown field",
        ),
        (
            control::KIND,
            "interrupt",
            "result",
            Value::Null,
            "INTERRUPT_INCOMPLETE",
        ),
    ] {
        let f = ExecutionFixture::new(State::Interrupted, "pinned")?;
        f.mutate(kind, source_id, |v| v[field] = value)?;
        f.rejected("bad-evidence", expected)?;
    }
    for kind in [
        "object_attempt_check_report",
        control::KIND,
        "object_attempt_trace",
        "object_attempt",
    ] {
        let f = ExecutionFixture::new(State::Interrupted, "pinned")?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let id = match kind {
            "object_attempt_check_report" => "check",
            control::KIND => "interrupt",
            _ => &f.attempt.id,
        };
        let mut orphan: Value = storage.store().get(kind, id)?.unwrap();
        match kind {
            "object_attempt_trace" => orphan["request"]["attemptId"] = json!("orphan"),
            "object_attempt" => {
                orphan["id"] = json!("orphan");
                orphan["preparation"]["run"]["id"] = json!("missing-run");
            }
            _ => {
                orphan["request"]["requestId"] = json!("orphan");
                orphan["request"]["target"]["attemptId"] = json!("orphan");
            }
        }
        if kind == control::KIND {
            orphan["result"]["target"]["attemptId"] = json!("orphan");
        }
        storage.store().put(kind, "orphan", &orphan)?;
        drop(storage);
        f.rejected("orphan", "DERIVATION_EXECUTION_ORPHAN")?;
    }
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    f.mutate("object_attempt_trace", &f.attempt.id, |t| {
        t["entries"][0]["sequence"] = json!(7)
    })?;
    f.rejected("trace-sequence", "OBJECT_ATTEMPT_TRACE_IDENTITY_MISMATCH")?;
    let f = ExecutionFixture::new(State::Interrupted, "pinned")?;
    f.mutate(control::KIND, "interrupt", |r| {
        r["result"]["error"] = json!("different terminal result")
    })?;
    f.rejected("interrupt-result", "INTERRUPT_INCOMPLETE")
}
