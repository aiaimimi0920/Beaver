use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use serde_json::{json, Value};

fn call(f: &Fixture, method: &str, input: &Value) -> Result<Value> {
    dispatch(&f.router, method, Some(input))
        .expect("object task method")
        .map_err(anyhow::Error::msg)
}

#[test]
fn object_task_api_restores_commits_and_replays_without_creating_legacy_tasks() -> Result<()> {
    let f = Fixture::new()?;
    let draft = call(&f, "objectTask.saveDraft", &draft_input())?;
    assert_eq!(draft["revision"], 1);
    assert_eq!(
        call(
            &f,
            "objectTask.getDraft",
            &json!({"projectId":"p","draftId":"draft"})
        )?,
        draft
    );
    assert_eq!(
        call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?,
        json!({"planRevision":0,"tasks":[],"runs":[],"assumptions":[],"coarseDispatchControls":[],"dispatchControls":[],"planningStates":[]})
    );
    let receipt = call(&f, "objectTask.commit", &commit_input())?;
    assert_eq!(call(&f, "objectTask.commit", &commit_input())?, receipt);
    assert_eq!(receipt["planRevision"], 1);
    assert_eq!(receipt["taskIds"].as_array().unwrap().len(), 3);
    assert_eq!(receipt["objectIds"], json!(["hero"]));
    let snapshot = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    assert_eq!(snapshot["tasks"].as_array().unwrap().len(), 3);
    assert_eq!(snapshot["runs"], receipt["runs"]);
    assert_eq!(snapshot["assumptions"][0]["id"], "lighting");
    assert_eq!(snapshot["assumptions"][0]["planRevision"], 1);
    let locked = call(
        &f,
        "objectTask.getDraft",
        &json!({"projectId":"p","draftId":"draft"}),
    )?;
    assert_eq!(locked["committedRequestId"], receipt["requestId"]);
    assert_eq!(locked["revision"], 2);
    assert_eq!(locked["plan"], draft["plan"]);
    let unlock = json!({"projectId":"p","draftId":"draft","requestId":"next", "expectedRevision":2,"expectedPlanRevision":1});
    let next = call(&f, "objectTask.unlockDraft", &unlock)?;
    assert_eq!(next["revision"], 3);
    assert_eq!(next["plan"]["tasks"], json!([]));
    assert!(next["committedRequestId"].is_null());
    assert_eq!(call(&f, "objectTask.unlockDraft", &unlock)?, next);
    assert_eq!(
        call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?,
        snapshot
    );
    for (id, parent) in [
        ("coarse", None),
        ("medium", Some("coarse")),
        ("fine", Some("medium")),
    ] {
        let task = call(&f, "objectTask.get", &json!({"projectId":"p","taskId":id}))?;
        assert_eq!(task["parentTaskId"], json!(parent));
        assert_eq!(task["status"], "planned");
        assert_eq!(task["revision"], 0);
        if id != "coarse" {
            assert_eq!(task["runId"], receipt["runs"][0]["id"]);
        }
    }
    let run = call(
        &f,
        "objectTask.getRun",
        &json!({"projectId":"p","runId":receipt["runs"][0]["id"]}),
    )?;
    assert_eq!(run["mediumTaskId"], "medium");
    assert!(run["baselineVersionId"].is_null());
    let runtime = f.router.runtime_for_project("p")?;
    let handle = runtime.store();
    let store = handle.lock().unwrap();
    assert!(store.list::<Value>("task")?.is_empty());
    assert_eq!(store.list::<Value>("object")?.len(), 1);
    assert_eq!(store.list::<Value>("object_task_commit_receipt")?.len(), 1);
    for kind in [
        "object",
        "object_task",
        "object_run",
        "object_task_draft",
        "task",
    ] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
    }
    Ok(())
}

#[test]
fn object_task_api_requires_open_project_for_every_read_and_write() -> Result<()> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    let receipt = call(&f, "objectTask.commit", &commit_input())?;
    let requests = [
        ("objectTask.saveDraft", draft_input()),
        (
            "objectTask.unlockDraft",
            json!({"projectId":"p","draftId":"draft","requestId":"next","expectedRevision":2,"expectedPlanRevision":1}),
        ),
        ("objectTask.commit", commit_input()),
        (
            "objectTask.cancelPlanned",
            json!({
                "projectId":"p","taskId":"fine","requestId":"cancel-fine",
                "expectedTaskRevision":0,"expectedPlanRevision":1
            }),
        ),
        ("objectTask.snapshot", json!({"projectId":"p"})),
        (
            "objectTask.getDraft",
            json!({"projectId":"p","draftId":"draft"}),
        ),
        ("objectTask.get", json!({"projectId":"p","taskId":"fine"})),
        (
            "objectTask.getRun",
            json!({"projectId":"p","runId":receipt["runs"][0]["id"]}),
        ),
    ];
    for project in ["legacy", "closed", "missing"] {
        for (method, input) in &requests {
            let mut input = input.clone();
            input["projectId"] = json!(project);
            assert!(call(&f, method, &input).is_err(), "{project}: {method}");
        }
    }
    for (method, field, id) in [
        ("objectTask.getDraft", "draftId", json!("draft")),
        ("objectTask.get", "taskId", json!("fine")),
        (
            "objectTask.getRun",
            "runId",
            receipt["runs"][0]["id"].clone(),
        ),
    ] {
        let mut input = json!({"projectId":"other"});
        input[field] = id;
        assert!(call(&f, method, &input)?.is_null());
    }
    f.host.lock().unwrap().put(
        "task",
        "host-only",
        &json!({"id":"host-only","projectId":"p"}),
    )?;
    assert!(call(
        &f,
        "objectTask.get",
        &json!({"projectId":"p","taskId":"host-only"})
    )?
    .is_null());
    assert_eq!(
        call(&f, "objectTask.snapshot", &json!({"projectId":"other"}))?,
        json!({"planRevision":0,"tasks":[],"runs":[],"assumptions":[],"coarseDispatchControls":[],"dispatchControls":[],"planningStates":[]})
    );
    assert!(f.router.runtime_for_project("closed").is_err());
    Ok(())
}

#[test]
fn object_task_api_rejects_unknown_nested_fields_and_invalid_revisions() -> Result<()> {
    let f = Fixture::new()?;
    for (pointer, value) in [
        ("/plan/tasks/1", json!({"runId":"forged"})),
        ("/plan/objects/0", json!({"versions":[]})),
        ("/plan/assumptions/0", json!({"unreviewed":true})),
        ("/plan", json!({"execute":true})),
        ("", json!({"autoAccept":true})),
    ] {
        let mut input = draft_input();
        input
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
        assert!(call(&f, "objectTask.saveDraft", &input).is_err());
    }
    for revision in [json!(-1), json!(0.5), json!("0")] {
        let mut input = draft_input();
        input["expectedRevision"] = revision;
        assert!(call(&f, "objectTask.saveDraft", &input).is_err());
    }
    for position in [json!(-1), json!(0.5), json!("0"), json!(1_000_000_001)] {
        let mut input = draft_input();
        input["plan"]["tasks"][0]["position"] = position;
        assert!(call(&f, "objectTask.saveDraft", &input).is_err());
    }
    for tool in crate::object_task_catalog::tools() {
        let method = tool["name"].as_str().unwrap();
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        if matches!(
            method,
            "objectTask.attempts"
                | "objectTask.attemptFile"
                | "objectTask.attemptTrace"
                | "objectTask.checkAttempt"
                | "objectTask.attemptChecks"
                | "objectTask.prepareCandidateReview"
                | "objectTask.candidateReviews"
                | "objectTask.publicationPreview"
                | "objectTask.publishCandidate"
                | "objectTask.publications"
                | "objectTask.abortPublication"
                | "objectTask.advanceAttempt"
                | "objectTask.reworkCandidate"
                | "objectTask.deferCandidateFeedback"
                | "objectTask.createPublicationFollowup"
                | "objectTask.publicationFrames"
                | "objectTask.attemptFrames"
                | "objectTask.publicationFollowups"
                | "objectTask.interrupt"
                | "objectTask.recovery"
                | "objectTask.verifyRecovery"
                | "objectTask.disposeRecovery"
                | "objectTask.resumeRecovery"
        ) {
            assert!(dispatch(&f.router, method, None).is_none());
            assert!(dispatch(&f.router, method, Some(&json!({}))).is_none());
            continue;
        }
        assert!(dispatch(&f.router, method, None).unwrap().is_err());
        assert!(call(&f, method, &json!({})).is_err());
    }
    assert!(dispatch(&f.router, "objectTask.execute", None).is_none());
    assert!(call(
        &f,
        "objectTask.getDraft",
        &json!({"projectId":"p","draftId":"draft"})
    )?
    .is_null());
    assert_eq!(
        call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?,
        json!({"planRevision":0,"tasks":[],"runs":[],"assumptions":[],"coarseDispatchControls":[],"dispatchControls":[],"planningStates":[]})
    );
    Ok(())
}

#[test]
fn object_task_api_preserves_inner_conflict_codes_for_clients() -> Result<()> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    let error = call(&f, "objectTask.saveDraft", &draft_input()).unwrap_err();
    assert!(error
        .to_string()
        .contains("OBJECT_TASK_DRAFT_REVISION_CONFLICT"));
    let mut stale = commit_input();
    stale["expectedDraftRevision"] = json!(0);
    let error = call(&f, "objectTask.commit", &stale).unwrap_err();
    assert!(error
        .to_string()
        .contains("OBJECT_TASK_DRAFT_REVISION_CONFLICT"));
    let receipt = call(&f, "objectTask.commit", &commit_input())?;
    stale = commit_input();
    stale["requestId"] = json!("stale");
    let error = call(&f, "objectTask.commit", &stale).unwrap_err();
    assert!(error
        .to_string()
        .contains("OBJECT_TASK_PLAN_REVISION_CONFLICT"));
    assert_eq!(call(&f, "objectTask.commit", &commit_input())?, receipt);
    Ok(())
}

#[test]
fn planned_task_cancellation_is_routed_idempotently_and_keeps_its_run() -> Result<()> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    call(&f, "objectTask.commit", &commit_input())?;
    let request = json!({
        "projectId":"p","taskId":"fine","requestId":"cancel-fine",
        "expectedTaskRevision":0,"expectedPlanRevision":1
    });

    let receipt = call(&f, "objectTask.cancelPlanned", &request)?;
    assert_eq!(receipt["previousTaskRevision"], 0);
    assert_eq!(receipt["taskRevision"], 1);
    assert_eq!(receipt["previousPlanRevision"], 1);
    assert_eq!(receipt["planRevision"], 2);
    assert_eq!(call(&f, "objectTask.cancelPlanned", &request)?, receipt);

    let task = call(
        &f,
        "objectTask.get",
        &json!({"projectId":"p","taskId":"fine"}),
    )?;
    assert_eq!(task["status"], "cancelled");
    assert_eq!(task["revision"], 1);
    let snapshot = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    assert_eq!(snapshot["planRevision"], 2);
    assert_eq!(
        snapshot["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "fine"),
        Some(&task)
    );
    assert_eq!(snapshot["runs"][0]["status"], "planned");
    assert_eq!(snapshot["runs"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["tasks"].as_array().unwrap().len(), 3);
    Ok(())
}

#[test]
fn medium_cancellation_closes_owned_work_through_shared_dispatch() -> Result<()> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    call(&f, "objectTask.commit", &commit_input())?;
    let before = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    let request = json!({
        "projectId":"p","taskId":"medium","requestId":"cancel-medium",
        "expectedTaskRevision":0,"expectedPlanRevision":1
    });
    let receipt = call(&f, "objectTask.cancelPlanned", &request)?;
    assert_eq!(call(&f, "objectTask.cancelPlanned", &request)?, receipt);
    let after = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    assert_eq!(after["planRevision"], 2);
    assert_eq!(after["assumptions"], before["assumptions"]);
    assert_eq!(after["runs"][0]["id"], before["runs"][0]["id"]);
    assert_eq!(after["runs"][0]["status"], "cancelled");
    assert_eq!(after["runs"][0]["revision"], 1);
    for id in ["medium", "fine"] {
        let task = call(&f, "objectTask.get", &json!({"projectId":"p","taskId":id}))?;
        assert_eq!(task["status"], "cancelled");
        assert_eq!(task["revision"], 1);
    }
    let coarse = call(
        &f,
        "objectTask.get",
        &json!({"projectId":"p","taskId":"coarse"}),
    )?;
    assert_eq!(coarse["status"], "planned");
    Ok(())
}

#[test]
fn explicit_medium_baseline_survives_draft_commit_and_replay() -> Result<()> {
    for policy in ["empty", "latestAccepted"] {
        let f = Fixture::new()?;
        let mut input = draft_input();
        let baseline = json!({"basePolicy":policy});
        input["plan"]["tasks"][1]["baseline"] = baseline.clone();
        let draft = call(&f, "objectTask.saveDraft", &input)?;
        assert_eq!(draft["plan"]["tasks"][1]["baseline"], baseline);
        let receipt = call(&f, "objectTask.commit", &commit_input())?;
        assert_eq!(call(&f, "objectTask.commit", &commit_input())?, receipt);
        let task = call(
            &f,
            "objectTask.get",
            &json!({"projectId":"p","taskId":"medium"}),
        )?;
        assert_eq!(task["identity"]["baseline"], baseline);
        assert!(receipt["runs"][0]["baselineVersionId"].is_null());
    }
    Ok(())
}

#[test]
fn invalid_baselines_are_rejected_before_a_draft_is_written() -> Result<()> {
    let f = Fixture::new()?;
    for (index, baseline) in [
        (0, json!({"basePolicy":"empty"})),
        (2, json!({"basePolicy":"empty"})),
        (1, json!({"basePolicy":"unknown"})),
        (
            1,
            json!({"basePolicy":"pinnedVersion","selectedVersionId":""}),
        ),
        (
            1,
            json!({"basePolicy":"pinnedVersion","selectedVersionId":"missing"}),
        ),
        (
            1,
            json!({"basePolicy":"empty","resolvedVersionId":"forged"}),
        ),
    ] {
        let mut input = draft_input();
        input["plan"]["tasks"][index]["baseline"] = baseline;
        assert!(call(&f, "objectTask.saveDraft", &input).is_err());
    }
    assert!(call(
        &f,
        "objectTask.getDraft",
        &json!({"projectId":"p","draftId":"draft"})
    )?
    .is_null());
    Ok(())
}
