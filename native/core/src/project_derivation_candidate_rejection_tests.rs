use super::*;

#[test]
fn project_derivation_candidate_rejects_forged_accepted_stage_history() -> Result<()> {
    for mode in ["title", "revision", "missing", "order"] {
        let f = CandidateFixture::staged(2)?;
        f.mutate(|v| {
            let fines = v["source"]["fines"].as_array_mut().unwrap();
            match mode {
                "title" => fines[0]["title"] = json!("Invented accepted stage"),
                "revision" => fines[0]["revision"] = json!(99),
                "missing" => {
                    fines.remove(0);
                }
                _ => fines.reverse(),
            }
        })?;
        f.execution
            .rejected(mode, "CANDIDATE_FINAL_FINE_HISTORY_MISMATCH")?;
    }
    Ok(())
}

#[test]
fn project_derivation_candidate_rejects_orphan_identity_and_unknown_history() -> Result<()> {
    for (pointer, value, expected) in [
        (
            "/report/request/projectId",
            json!("other"),
            "CANDIDATE_IDENTITY_MISMATCH",
        ),
        (
            "/report/request/requestId",
            json!("other"),
            "CANDIDATE_IDENTITY_MISMATCH",
        ),
        (
            "/report/request/target/attemptId",
            json!("orphan"),
            "CANDIDATE_ORPHAN_REVIEW",
        ),
        (
            "/report/request/target/claimToken",
            json!("other"),
            "CANDIDATE_TERMINAL_GATE_REQUIRED",
        ),
        (
            "/report/request/checkRequestId",
            json!("missing"),
            "OBJECT_STAGE_PASSING_REPORT_REQUIRED",
        ),
        (
            "/report/request/checkRequestId",
            json!(" "),
            "CANDIDATE_IDENTITY_MISMATCH",
        ),
        (
            "/report/schemaVersion",
            json!(3),
            "OBJECT_CANDIDATE_SCHEMA_UNSUPPORTED",
        ),
        (
            "/report/sourceDigest",
            json!("0".repeat(64)),
            "OBJECT_CANDIDATE_REPORT_MISMATCH",
        ),
        (
            "/report/outputDigest",
            json!("0".repeat(64)),
            "OBJECT_CANDIDATE_REPORT_MISMATCH",
        ),
        (
            "/report/files/0/path",
            json!("changed.txt"),
            "OBJECT_CANDIDATE_REPORT_MISMATCH",
        ),
        (
            "/report/rules",
            json!([]),
            "OBJECT_CANDIDATE_RULES_MISMATCH",
        ),
        (
            "/report/rules/0/version",
            json!(2),
            "OBJECT_CANDIDATE_RULES_MISMATCH",
        ),
        (
            "/report/rules/0/issues",
            json!(["contradicts passed"]),
            "OBJECT_CANDIDATE_RULES_MISMATCH",
        ),
        (
            "/source/fines",
            json!([]),
            "CANDIDATE_FINAL_FINE_HISTORY_MISMATCH",
        ),
        (
            "/source/objects",
            json!([]),
            "CANDIDATE_CATALOG_MEMBERSHIP_MISMATCH",
        ),
        (
            "/source/records/history",
            json!([]),
            "DERIVATION_RECOVERY_PREFIX_MISMATCH",
        ),
        (
            "/source/records/control/paused",
            json!(true),
            "DERIVATION_RECOVERY_CONTROL_HISTORY_MISMATCH",
        ),
        (
            "/source/records/fine/title",
            json!("forged"),
            "DERIVATION_RECOVERY_TASK_SNAPSHOT_MISMATCH",
        ),
        (
            "/source/records/queue/claimToken",
            json!("forged"),
            "DERIVATION_RECOVERY_QUEUE_SNAPSHOT_MISMATCH",
        ),
    ] {
        let f = CandidateFixture::new(true, "pinned", 2)?;
        f.mutate(|v| *v.pointer_mut(pointer).unwrap() = value)?;
        f.execution.rejected("review-corruption", expected)?;
    }
    for section in ["source", "report"] {
        let f = CandidateFixture::new(false, "empty", 2)?;
        f.mutate(|v| v[section]["futureField"] = json!(true))?;
        f.execution
            .rejected("unknown-review-field", "unknown field `futureField`")?;
    }
    let f = CandidateFixture::new(true, "pinned", 2)?;
    f.mutate(|v| v["report"]["request"]["checkRequestId"] = json!("check"))?;
    f.execution
        .rejected("another-attempt-check", "OBJECT_CHECK_REQUEST_CONFLICT")?;
    let f = CandidateFixture::new(true, "pinned", 2)?;
    f.mutate(|v| {
        v["report"]["request"]["target"] = serde_json::to_value(
            crate::object_attempt_view::Target::from_record(&f.execution.attempt),
        )
        .unwrap()
    })?;
    f.execution
        .rejected("non-gate-review", "CANDIDATE_TERMINAL_GATE_REQUIRED")
}

#[test]
fn project_derivation_candidate_rejects_incomplete_catalog_and_forged_historical_owners(
) -> Result<()> {
    for mode in ["missing", "duplicate", "reverse", "forged"] {
        let f = CandidateFixture::new(false, "pinned", 2)?;
        f.mutate(|v| {
            let objects = v["source"]["objects"].as_array_mut().unwrap();
            match mode {
                "missing" => {
                    objects.pop();
                }
                "duplicate" => {
                    objects.push(objects[0].clone());
                }
                "reverse" => {
                    objects.reverse();
                }
                _ => {
                    objects[0]["name"] = json!("forged name");
                }
            }
        })?;
        f.execution.rejected(
            mode,
            if mode == "forged" {
                "DERIVATION_OBJECT_RECEIPT_PROJECTION_MISMATCH"
            } else {
                "CANDIDATE_CATALOG_MEMBERSHIP_MISMATCH"
            },
        )?;
    }
    let f = CandidateFixture::new(false, "pinned", 2)?;
    let runtime = ProjectStore::open(&f.execution.base.source, "original")?.into_runtime();
    crate::object_registration::register(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original", "requestId":"after-review", "name":"Added after review"
        }))?,
    )?;
    drop(runtime);
    f.execution.rejected(
        "changed-catalog-membership",
        "CANDIDATE_CATALOG_MEMBERSHIP_MISMATCH",
    )?;
    // A real older revision of an owner is still invalid if other parts of the same
    // frozen review describe that object at a different revision.
    let f = CandidateFixture::new(false, "pinned", 2)?;
    let storage = ProjectStore::open(&f.execution.base.source, "original")?;
    let receipt: Value = storage
        .store()
        .get(
            "object_command_receipt",
            &crate::project_derivation_queue_records::receipt_key("original", "update-child")?,
        )?
        .unwrap();
    drop(storage);
    f.mutate(|v| {
        let objects = v["source"]["objects"].as_array_mut().unwrap();
        let owner = objects
            .iter_mut()
            .find(|o| o["id"] == f.execution.base.child.id)
            .unwrap();
        *owner = receipt["result"]["object"].clone();
    })?;
    f.execution.rejected(
        "inconsistent-owner-time",
        "CANDIDATE_OBJECT_SNAPSHOT_MISMATCH",
    )
}

#[test]
fn project_derivation_candidate_failed_historical_recheck_stays_failed() -> Result<()> {
    let f = CandidateFixture::new(false, "empty", 1)?;
    // Historical checks can differ from the earlier passing check named by the request.
    f.mutate(|v| {
        v["report"]["rules"][0]["passed"] = json!(false);
        v["report"]["rules"][0]["issues"] =
            json!(["original source-review: historical content unavailable"]);
        v["report"]["blockers"]
            .as_array_mut()
            .unwrap()
            .insert(3, json!("TECHNICAL_CHECK_FAILED"));
    })?;
    let base = &f.execution.base;
    let preparation = base.temp.path().join("failed-check-prepared");
    let prepared = copy::prepare(base.request(), &preparation)?;
    let target = base.temp.path().join("failed-check-target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let storage = ProjectStore::open(&target.join("project"), "derived")?;
    let id = Rewrite(&prepared.identities).key(KIND, "source-review")?.id;
    let saved: Value = storage.store().get(KIND, &id)?.unwrap();
    assert_eq!(saved["report"]["rules"][0]["passed"], false);
    assert_eq!(
        saved["report"]["rules"][0]["issues"],
        json!(["original source-review: historical content unavailable"])
    );
    assert!(saved["report"]["blockers"]
        .as_array()
        .unwrap()
        .contains(&json!("TECHNICAL_CHECK_FAILED")));
    Ok(())
}
