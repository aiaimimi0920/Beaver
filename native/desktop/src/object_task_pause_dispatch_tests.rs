use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use serde_json::{json, Value};

fn call(f: &Fixture, method: &str, input: &Value) -> Result<Value> {
    dispatch(&f.router, method, Some(input))
        .expect("object task method")
        .map_err(anyhow::Error::msg)
}

fn setup() -> Result<(Fixture, Value)> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    let committed = call(&f, "objectTask.commit", &commit_input())?;
    let input = json!({
        "projectId":"p","taskId":"medium","objectId":"hero",
        "runId":committed["runs"][0]["id"],"requestId":"pause",
        "expectedTaskRevision":0,"expectedControlRevision":0,"paused":true
    });
    Ok((f, input))
}

#[test]
fn object_task_coarse_pause_dispatch_is_strict_project_local_and_independent() -> Result<()> {
    let (f, medium) = setup()?;
    let input = json!({"projectId":"p","taskId":"coarse","requestId":"coarse-pause",
        "expectedTaskRevision":0,"expectedControlRevision":0,"paused":true});
    for field in input.as_object().unwrap().keys() {
        let mut missing = input.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(call(&f, "objectTask.setCoarsePaused", &missing).is_err());
    }
    for (field, value) in [
        ("paused", json!("true")),
        ("runId", json!("run")),
        ("expectedControlRevision", json!(-1)),
        ("expectedControlRevision", json!(9_007_199_254_740_991_u64)),
        ("taskId", json!("medium")),
        ("projectId", json!("other")),
        ("projectId", json!("closed")),
        ("projectId", json!("legacy")),
    ] {
        let mut invalid = input.clone();
        invalid[field] = value;
        assert!(call(&f, "objectTask.setCoarsePaused", &invalid).is_err());
    }
    let receipt = call(&f, "objectTask.setCoarsePaused", &input)?;
    call(
        &f,
        "objectTask.enqueue",
        &json!({"projectId":"p","taskIds":["medium"]}),
    )?;
    let claim = json!({"projectId":"p","owner":"worker"});
    assert!(call(&f, "objectTask.claim", &claim)?.is_null());
    call(&f, "objectTask.setPaused", &medium)?;
    let mut unpause = input.clone();
    unpause["requestId"] = json!("coarse-unpause");
    unpause["paused"] = json!(false);
    unpause["expectedControlRevision"] = json!(1);
    call(&f, "objectTask.setCoarsePaused", &unpause)?;
    assert!(call(&f, "objectTask.claim", &claim)?.is_null());
    assert_eq!(call(&f, "objectTask.setCoarsePaused", &input)?, receipt);
    let snapshot = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    assert_eq!(snapshot["coarseDispatchControls"][0]["paused"], false);
    assert_eq!(snapshot["dispatchControls"][0]["paused"], true);
    let mut resume = medium;
    resume["requestId"] = json!("medium-unpause");
    resume["paused"] = json!(false);
    resume["expectedControlRevision"] = json!(1);
    call(&f, "objectTask.setPaused", &resume)?;
    assert_eq!(
        call(&f, "objectTask.claim", &claim)?["task"]["id"],
        "medium"
    );
    for kind in [
        "object_task_coarse_dispatch_control",
        "object_task_coarse_dispatch_receipt",
    ] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        assert!(f
            .router
            .runtime_for_project("other")?
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .is_empty());
    }
    Ok(())
}

#[test]
fn object_task_pause_dispatch_blocks_claim_and_replays_receipt_after_unpause() -> Result<()> {
    let (f, input) = setup()?;
    let receipt = call(&f, "objectTask.setPaused", &input)?;
    call(
        &f,
        "objectTask.enqueue",
        &json!({"projectId":"p","taskIds":["medium"]}),
    )?;
    let claim = json!({"projectId":"p","owner":"worker"});
    assert!(call(&f, "objectTask.claim", &claim)?.is_null());
    let snapshot = call(&f, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    assert_eq!(snapshot["dispatchControls"], json!([receipt["result"]]));
    let mut unpause = input.clone();
    unpause["requestId"] = json!("unpause");
    unpause["paused"] = json!(false);
    unpause["expectedControlRevision"] = json!(1);
    call(&f, "objectTask.setPaused", &unpause)?;
    assert_eq!(
        call(&f, "objectTask.claim", &claim)?["task"]["id"],
        "medium"
    );
    assert_eq!(call(&f, "objectTask.setPaused", &input)?, receipt);
    Ok(())
}

#[test]
fn object_task_pause_dispatch_rejects_bad_shapes_and_unrelated_project_storage() -> Result<()> {
    let (f, input) = setup()?;
    for project in ["other", "closed", "legacy", "missing"] {
        let mut foreign = input.clone();
        foreign["projectId"] = json!(project);
        assert!(
            call(&f, "objectTask.setPaused", &foreign).is_err(),
            "{project}"
        );
    }
    for field in input.as_object().unwrap().keys() {
        let mut missing = input.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            call(&f, "objectTask.setPaused", &missing)
                .unwrap_err()
                .to_string()
                .contains("INVALID_INPUT"),
            "{field}"
        );
    }
    for (field, value) in [
        ("paused", json!("true")),
        ("extra", json!(1)),
        ("expectedControlRevision", json!(-1)),
        ("expectedControlRevision", json!(9_007_199_254_740_991_u64)),
    ] {
        let mut invalid = input.clone();
        invalid[field] = value;
        assert!(call(&f, "objectTask.setPaused", &invalid).is_err());
    }
    for kind in [
        "object_task_dispatch_control",
        "object_task_dispatch_receipt",
    ] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
        for project in ["p", "other"] {
            assert!(f
                .router
                .runtime_for_project(project)?
                .store()
                .lock()
                .unwrap()
                .list::<Value>(kind)?
                .is_empty());
        }
    }
    Ok(())
}
