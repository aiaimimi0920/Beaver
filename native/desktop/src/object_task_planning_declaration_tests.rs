use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::project_storage_router::ProjectStorageRouter;
use serde_json::{json, Value};

fn call(router: &ProjectStorageRouter, method: &str, input: &Value) -> Result<Value> {
    dispatch(router, method, Some(input))
        .expect("route")
        .map_err(anyhow::Error::msg)
}
fn snapshot(router: &ProjectStorageRouter) -> Result<Value> {
    call(router, "objectTask.snapshot", &json!({"projectId":"p"}))
}
fn state<'a>(snapshot: &'a Value, id: &str) -> &'a Value {
    snapshot["planningStates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["taskId"] == id)
        .unwrap()
}
fn request(snapshot: &Value, id: &str, request_id: &str) -> Value {
    json!({"projectId":"p","taskId":id,"requestId":request_id,
        "expectedPlanRevision":snapshot["planRevision"],"expectedScopeHash":state(snapshot,id)["scopeHash"],
        "reason":"Reviewed all requirements and their planned tasks"})
}
fn fixture(input: &Value) -> Result<Fixture> {
    let f = Fixture::new()?;
    call(&f.router, "objectTask.saveDraft", input)?;
    call(&f.router, "objectTask.commit", &commit_input())?;
    Ok(f)
}

#[test]
fn declaration_survives_reopen_without_accepting_tasks_and_retries_exactly() -> Result<()> {
    let f = fixture(&draft_input())?;
    let before = snapshot(&f.router)?;
    let input = request(&before, "coarse", "declare");
    let receipt = call(&f.router, "objectTask.declarePlanningComplete", &input)?;
    assert_eq!(receipt["declaredBy"], "owner");
    assert_eq!(receipt["taskIds"], json!(["coarse", "fine", "medium"]));
    let after = snapshot(&f.router)?;
    assert_eq!(after["tasks"], before["tasks"]);
    assert_eq!(after["runs"], before["runs"]);
    assert_eq!(after["planRevision"], before["planRevision"]);
    assert_eq!(state(&after, "coarse")["current"], true);
    assert_eq!(state(&after, "medium")["current"], false);
    let Fixture {
        router,
        host,
        temp: _temp,
    } = f;
    drop(router);
    let router = ProjectStorageRouter::new(host.clone());
    drop(router.open_registered("p")?);
    assert_eq!(snapshot(&router)?, after);
    assert_eq!(
        call(&router, "objectTask.declarePlanningComplete", &input)?,
        receipt
    );
    let mut conflict = input;
    conflict["reason"] = json!("different");
    assert!(
        call(&router, "objectTask.declarePlanningComplete", &conflict)
            .unwrap_err()
            .to_string()
            .contains("REQUEST_CONFLICT")
    );
    assert!(host
        .lock()
        .unwrap()
        .list::<Value>("object_task_planning_declaration")?
        .is_empty());
    Ok(())
}

#[test]
fn edits_additions_and_cancellation_invalidate_but_execution_does_not() -> Result<()> {
    let f = fixture(&draft_input())?;
    let original = request(&snapshot(&f.router)?, "coarse", "declare");
    let receipt = call(&f.router, "objectTask.declarePlanningComplete", &original)?;
    call(
        &f.router,
        "objectTask.revisePlanned",
        &json!({
            "projectId":"p","taskId":"fine","requestId":"revise","expectedTaskRevision":0,"expectedPlanRevision":1,
            "definition":{"title":"Movement","prompt":"Add jumping","acceptance":"Jumps","dependsOn":[]},"reason":"More scope"
        }),
    )?;
    let revised = snapshot(&f.router)?;
    assert_eq!(state(&revised, "coarse")["current"], false);
    let mut stale = original.clone();
    stale["requestId"] = json!("stale");
    stale["expectedPlanRevision"] = revised["planRevision"].clone();
    assert!(
        call(&f.router, "objectTask.declarePlanningComplete", &stale)
            .unwrap_err()
            .to_string()
            .contains("SCOPE_CONFLICT")
    );
    call(
        &f.router,
        "objectTask.declarePlanningComplete",
        &request(&revised, "coarse", "redeclare"),
    )?;
    let addition = json!({"projectId":"p","draftId":"extra","expectedRevision":0,"expectedPlanRevision":2,
        "plan":{"objects":[],"tasks":[{"id":"extra","granularity":"fine","title":"Audio","prompt":"Add audio","acceptance":"Audible","objectId":"hero","parentTaskId":"medium","stageId":"audio"}]}});
    call(&f.router, "objectTask.saveDraft", &addition)?;
    call(
        &f.router,
        "objectTask.commit",
        &json!({"projectId":"p","requestId":"extra-commit","draftId":"extra","expectedDraftRevision":1,"expectedPlanRevision":2}),
    )?;
    let added = snapshot(&f.router)?;
    assert_eq!(state(&added, "coarse")["current"], false);
    call(
        &f.router,
        "objectTask.declarePlanningComplete",
        &request(&added, "coarse", "expanded"),
    )?;
    call(
        &f.router,
        "objectTask.cancelPlanned",
        &json!({"projectId":"p","taskId":"extra","requestId":"cancel","expectedTaskRevision":0,"expectedPlanRevision":3}),
    )?;
    let cancelled = snapshot(&f.router)?;
    assert_eq!(state(&cancelled, "coarse")["current"], false);
    call(
        &f.router,
        "objectTask.declarePlanningComplete",
        &request(&cancelled, "coarse", "after-cancel"),
    )?;
    call(
        &f.router,
        "objectTask.enqueue",
        &json!({"projectId":"p","taskIds":["medium"]}),
    )?;
    assert_eq!(state(&snapshot(&f.router)?, "coarse")["current"], true);
    assert_eq!(
        call(&f.router, "objectTask.declarePlanningComplete", &original)?,
        receipt
    );
    Ok(())
}

#[test]
fn planning_gaps_empty_branches_and_fine_targets_cannot_be_declared() -> Result<()> {
    for variant in ["notes", "empty"] {
        let mut input = draft_input();
        if variant == "notes" {
            input["plan"]["tasks"][1]["pendingPlanning"] = json!("Export");
        } else {
            input["plan"]["tasks"].as_array_mut().unwrap().pop();
        }
        let f = fixture(&input)?;
        let before = snapshot(&f.router)?;
        for id in ["coarse", "medium"] {
            assert!(!state(&before, id)["blockers"]
                .as_array()
                .unwrap()
                .is_empty());
            assert!(call(
                &f.router,
                "objectTask.declarePlanningComplete",
                &request(&before, id, id)
            )
            .unwrap_err()
            .to_string()
            .contains("INCOMPLETE"));
        }
        assert_eq!(snapshot(&f.router)?, before);
    }
    let f = fixture(&draft_input())?;
    let mut input = request(&snapshot(&f.router)?, "medium", "fine-request");
    input["taskId"] = json!("fine");
    assert!(
        call(&f.router, "objectTask.declarePlanningComplete", &input)
            .unwrap_err()
            .to_string()
            .contains("PARENT_REQUIRED")
    );
    Ok(())
}
