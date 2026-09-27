use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use serde_json::{json, Value};

fn call(f: &Fixture, method: &str, input: &Value) -> Result<Value> {
    dispatch(&f.router, method, Some(input))
        .expect("object task method")
        .map_err(anyhow::Error::msg)
}

fn committed_fixture() -> Result<Fixture> {
    let f = Fixture::new()?;
    let mut input = draft_input();
    let tasks = input["plan"]["tasks"].as_array_mut().unwrap();
    for (index, task) in tasks.iter_mut().enumerate() {
        task["position"] = json!(index * 10);
    }
    tasks.push(json!({
        "id":"followup","granularity":"medium","position":30,
        "title":"Follow up","prompt":"Improve hero","acceptance":"Improved","objectId":"hero"
    }));
    let draft = call(&f, "objectTask.saveDraft", &input)?;
    for (index, task) in draft["plan"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(task["position"], input["plan"]["tasks"][index]["position"]);
    }
    call(&f, "objectTask.commit", &commit_input())?;
    Ok(f)
}

fn read(f: &Fixture, method: &str) -> Result<Value> {
    call(f, method, &json!({"projectId":"p"}))
}

fn enqueue(f: &Fixture) -> Result<Value> {
    call(
        f,
        "objectTask.enqueue",
        &json!({"projectId":"p","taskIds":["medium","followup"]}),
    )
}

fn claim(f: &Fixture) -> Result<Value> {
    call(
        f,
        "objectTask.claim",
        &json!({"projectId":"p","owner":"worker-a"}),
    )
}

fn owned_input(claim: &Value) -> Value {
    json!({
        "projectId":"p","taskId":claim["task"]["id"],"owner":"worker-a",
        "claimToken":claim["claimToken"],"generation":claim["generation"]
    })
}

fn task<'a>(snapshot: &'a Value, id: &str) -> &'a Value {
    snapshot["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["id"] == id)
        .unwrap()
}

fn run<'a>(snapshot: &'a Value, medium_id: &str) -> &'a Value {
    snapshot["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|run| run["mediumTaskId"] == medium_id)
        .unwrap()
}

#[test]
fn object_queue_dispatch_finishes_without_releasing_object_ownership() -> Result<()> {
    for (success, status) in [(true, "awaitingAcceptance"), (false, "failed")] {
        let f = committed_fixture()?;
        let queued = enqueue(&f)?;
        assert_eq!(queued.as_array().unwrap().len(), 2);
        assert_eq!(enqueue(&f)?, queued);
        assert_eq!(read(&f, "objectTask.queue")?, queued);
        assert_eq!(queued[0]["taskId"], "medium");
        assert_eq!(queued[0]["position"], 10);
        assert_eq!(queued[1]["position"], 30);
        let claimed = claim(&f)?;
        assert_eq!(claimed["task"]["id"], "medium");
        assert_eq!(claimed["generation"], 1);
        let before = read(&f, "objectTask.snapshot")?;
        let before_queue = read(&f, "objectTask.queue")?;
        assert_eq!(task(&before, "medium")["status"], "queued");
        assert_eq!(task(&before, "medium")["revision"], 1);
        assert_eq!(run(&before, "medium")["status"], "queued");
        assert_eq!(run(&before, "medium")["revision"], 1);
        let mut input = owned_input(&claimed);
        input["success"] = json!(success);
        let mut stale = input.clone();
        stale["claimToken"] = json!("stale");
        assert!(call(&f, "objectTask.finish", &stale).is_err());
        assert_eq!(read(&f, "objectTask.snapshot")?, before);
        assert_eq!(read(&f, "objectTask.queue")?, before_queue);
        let finished = call(&f, "objectTask.finish", &input)?;
        assert_eq!(finished["status"], status);
        assert_eq!(finished["revision"], 2);
        let after = read(&f, "objectTask.snapshot")?;
        let after_queue = read(&f, "objectTask.queue")?;
        assert_eq!(task(&after, "medium"), &finished);
        assert_eq!(run(&after, "medium")["status"], status);
        assert_eq!(run(&after, "medium")["revision"], 2);
        assert_eq!(task(&after, "fine")["status"], "planned");
        assert_eq!(after["planRevision"], 1);
        assert_eq!(after_queue[0]["state"], status);
        assert!(claim(&f)?.is_null());
        assert!(call(&f, "objectTask.finish", &input).is_err());
        assert_eq!(read(&f, "objectTask.snapshot")?, after);
        assert_eq!(read(&f, "objectTask.queue")?, after_queue);
    }
    Ok(())
}

#[test]
fn object_queue_dispatch_reorders_only_medium_tasks_and_persists_claim_order() -> Result<()> {
    let f = committed_fixture()?;
    let mut reorder = json!({"projectId":"p","requestId":"move","taskId":"followup","expectedVersion":read(&f, "objectTask.queueView")?["version"],"previousTaskId":null,"nextTaskId":"medium"});
    assert!(call(&f, "objectTask.reorder", &reorder).is_err());
    for invalid in ["coarse", "fine"] {
        assert!(call(
            &f,
            "objectTask.enqueue",
            &json!({"projectId":"p","taskIds":["medium",invalid]})
        )
        .is_err());
        assert_eq!(read(&f, "objectTask.queue")?, json!([]));
    }
    enqueue(&f)?;
    reorder["expectedVersion"] = read(&f, "objectTask.queueView")?["version"].clone();
    for field in ["previousTaskId", "nextTaskId"] {
        let mut missing = reorder.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(call(&f, "objectTask.reorder", &missing).is_err());
    }
    let reordered = call(&f, "objectTask.reorder", &reorder)?;
    assert_eq!(reordered["result"]["items"][0]["taskId"], "followup");
    assert_eq!(reordered["result"]["items"][1]["taskId"], "medium");
    assert_eq!(read(&f, "objectTask.queueView")?, reordered["result"]);
    assert_eq!(claim(&f)?["task"]["id"], "followup");
    assert_eq!(call(&f, "objectTask.reorder", &reorder)?, reordered);
    assert!(claim(&f)?.is_null());
    Ok(())
}

#[test]
fn object_queue_dispatch_cancel_claim_closes_owned_metadata_and_unblocks_followup() -> Result<()> {
    for success in [None, Some(true), Some(false)] {
        let f = committed_fixture()?;
        enqueue(&f)?;
        let claimed = claim(&f)?;
        let input = owned_input(&claimed);
        let mut finish = input.clone();
        finish["success"] = json!(success.unwrap_or(true));
        if success.is_some() {
            call(&f, "objectTask.finish", &finish)?;
        }
        let before = read(&f, "objectTask.snapshot")?;
        let cancelled = call(&f, "objectTask.cancelClaim", &input)?;
        let after = read(&f, "objectTask.snapshot")?;
        let queue = read(&f, "objectTask.queue")?;
        assert_eq!(cancelled["status"], "cancelled");
        assert_eq!(cancelled["revision"], if success.is_some() { 3 } else { 2 });
        assert_eq!(task(&after, "medium"), &cancelled);
        assert_eq!(task(&after, "fine")["status"], "cancelled");
        assert_eq!(task(&after, "fine")["revision"], 1);
        assert_eq!(run(&after, "medium")["status"], "cancelled");
        assert_eq!(run(&after, "medium")["revision"], cancelled["revision"]);
        assert_eq!(after["planRevision"], 2);
        assert_eq!(queue[0]["state"], "cancelled");
        assert_eq!(queue[1]["state"], "queued");
        assert_eq!(after["assumptions"], before["assumptions"]);
        for id in ["coarse", "followup"] {
            assert_eq!(task(&after, id), task(&before, id));
        }
        assert_eq!(run(&after, "followup"), run(&before, "followup"));
        assert!(call(&f, "objectTask.finish", &finish).is_err());
        assert!(call(&f, "objectTask.cancelClaim", &input).is_err());
        assert_eq!(read(&f, "objectTask.snapshot")?, after);
        assert_eq!(read(&f, "objectTask.queue")?, queue);
        assert_eq!(claim(&f)?["task"]["id"], "followup");
    }
    Ok(())
}

#[test]
fn object_queue_dispatch_is_project_local_and_requires_an_open_project() -> Result<()> {
    let f = committed_fixture()?;
    enqueue(&f)?;
    let claimed = claim(&f)?;
    let mut finish = owned_input(&claimed);
    finish["success"] = json!(true);
    let requests = [
        (
            "objectTask.enqueue",
            json!({"projectId":"p","taskIds":["medium"]}),
        ),
        ("objectTask.queue", json!({"projectId":"p"})),
        ("objectTask.queueView", json!({"projectId":"p"})),
        (
            "objectTask.claim",
            json!({"projectId":"p","owner":"worker-a"}),
        ),
        (
            "objectTask.reorder",
            json!({"projectId":"p","requestId":"move","taskId":"followup","expectedVersion":read(&f, "objectTask.queueView")?["version"],"previousTaskId":null,"nextTaskId":null}),
        ),
        ("objectTask.finish", finish),
        ("objectTask.cancelClaim", owned_input(&claimed)),
    ];
    let before = read(&f, "objectTask.snapshot")?;
    let before_queue = read(&f, "objectTask.queue")?;
    for project in ["legacy", "closed", "missing"] {
        for (method, input) in &requests {
            let mut input = input.clone();
            input["projectId"] = json!(project);
            assert!(call(&f, method, &input).is_err(), "{project}: {method}");
        }
    }
    for (method, input) in &requests {
        let mut input = input.clone();
        input["projectId"] = json!("other");
        match *method {
            "objectTask.queue" => assert_eq!(call(&f, method, &input)?, json!([])),
            "objectTask.queueView" => {
                let view = call(&f, method, &input)?;
                assert_eq!(view["projectId"], "other");
                assert_eq!(view["items"], json!([]));
            }
            "objectTask.claim" => assert!(call(&f, method, &input)?.is_null()),
            _ => assert!(call(&f, method, &input).is_err(), "{method}"),
        }
    }
    assert_eq!(read(&f, "objectTask.snapshot")?, before);
    assert_eq!(read(&f, "objectTask.queue")?, before_queue);
    assert_eq!(
        call(&f, "objectTask.snapshot", &json!({"projectId":"other"}))?,
        json!({"planRevision":0,"tasks":[],"runs":[],"assumptions":[],"coarseDispatchControls":[],"dispatchControls":[],"planningStates":[]})
    );
    for kind in ["object_task", "object_run", "object_task_queue", "task"] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
    }
    let runtime = f.router.runtime_for_project("p")?;
    assert!(runtime
        .store()
        .lock()
        .unwrap()
        .list::<Value>("task")?
        .is_empty());
    Ok(())
}
