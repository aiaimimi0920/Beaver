use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use serde_json::{json, Value};

fn call(f: &Fixture, method: &str, input: &Value) -> Result<Value> {
    dispatch(&f.router, method, Some(input))
        .expect("object task method")
        .map_err(anyhow::Error::msg)
}

fn request() -> Value {
    json!({
        "projectId":"p","taskId":"medium","requestId":"revise",
        "expectedTaskRevision":0,"expectedPlanRevision":1,
        "definition":{"title":"Playable hero","prompt":"Add keyboard movement","acceptance":"Moves with WASD","dependsOn":[]},
        "reason":"Clarify the interaction"
    })
}

fn fixture() -> Result<Fixture> {
    let f = Fixture::new()?;
    call(&f, "objectTask.saveDraft", &draft_input())?;
    call(&f, "objectTask.commit", &commit_input())?;
    Ok(f)
}

#[test]
fn object_task_revision_api_returns_persistent_history_and_stable_retries() -> Result<()> {
    let f = fixture()?;
    let receipt = call(&f, "objectTask.revisePlanned", &request())?;
    assert_eq!(receipt["before"]["title"], "Hero");
    assert_eq!(receipt["after"], request()["definition"]);
    assert_eq!(receipt["reason"], request()["reason"]);
    assert_eq!(receipt["adoptedBy"], "owner");
    assert_eq!(receipt["affectedTaskIds"], json!(["fine", "medium"]));
    assert_eq!(receipt["previousTaskRevision"], 0);
    assert_eq!(receipt["taskRevision"], 1);
    assert_eq!(receipt["previousPlanRevision"], 1);
    assert_eq!(receipt["planRevision"], 2);
    assert_eq!(call(&f, "objectTask.revisePlanned", &request())?, receipt);
    call(
        &f,
        "objectTask.cancelPlanned",
        &json!({
            "projectId":"p","taskId":"medium","requestId":"cancel",
            "expectedTaskRevision":1,"expectedPlanRevision":2
        }),
    )?;
    assert_eq!(
        call(
            &f,
            "objectTask.revisions",
            &json!({"projectId":"p","taskId":"medium"})
        )?,
        json!([receipt.clone()])
    );
    assert_eq!(call(&f, "objectTask.revisePlanned", &request())?, receipt);
    assert!(call(
        &f,
        "objectTask.revisions",
        &json!({"projectId":"other","taskId":"medium"})
    )
    .unwrap_err()
    .to_string()
    .contains("OBJECT_TASK_NOT_FOUND"));
    for kind in ["object_task", "object_task_definition_revision", "task"] {
        assert!(f.host.lock().unwrap().list::<Value>(kind)?.is_empty());
    }
    let other = f.router.runtime_for_project("other")?;
    assert!(other
        .store()
        .lock()
        .unwrap()
        .list::<Value>("object_task_definition_revision")?
        .is_empty());
    Ok(())
}

#[test]
fn object_task_revision_api_requires_open_explicit_project_and_strict_editable_fields() -> Result<()>
{
    let f = fixture()?;
    for (method, input) in [
        ("objectTask.revisePlanned", request()),
        (
            "objectTask.revisions",
            json!({"projectId":"p","taskId":"medium"}),
        ),
    ] {
        for project in ["closed", "legacy", "missing"] {
            let mut bad = input.clone();
            bad["projectId"] = json!(project);
            assert!(call(&f, method, &bad).is_err());
        }
        let mut bad = input;
        bad.as_object_mut().unwrap().remove("projectId");
        assert!(call(&f, method, &bad).is_err());
    }
    for field in [
        "objectId",
        "runId",
        "stageId",
        "parentTaskId",
        "identity",
        "status",
        "revision",
        "adoptedBy",
    ] {
        let mut bad = request();
        bad["definition"][field] = json!("forged");
        assert!(
            call(&f, "objectTask.revisePlanned", &bad).is_err(),
            "accepted {field}"
        );
    }
    let mut bad = request();
    bad["adoptedBy"] = json!("codex");
    assert!(call(&f, "objectTask.revisePlanned", &bad).is_err());
    let mut bad = request();
    bad["definition"]
        .as_object_mut()
        .unwrap()
        .remove("dependsOn");
    assert!(call(&f, "objectTask.revisePlanned", &bad).is_err());
    let runtime = f.router.runtime_for_project("p")?;
    assert!(runtime
        .store()
        .lock()
        .unwrap()
        .list::<Value>("object_task_definition_revision")?
        .is_empty());
    Ok(())
}

#[test]
fn object_task_revision_api_preserves_conflict_errors_and_catalog_annotations() -> Result<()> {
    let f = fixture()?;
    for (field, value, code) in [
        (
            "expectedPlanRevision",
            0,
            "OBJECT_TASK_PLAN_REVISION_CONFLICT",
        ),
        ("expectedTaskRevision", 1, "OBJECT_TASK_REVISION_CONFLICT"),
    ] {
        let mut bad = request();
        bad[field] = json!(value);
        assert!(call(&f, "objectTask.revisePlanned", &bad)
            .unwrap_err()
            .to_string()
            .contains(code));
    }
    let catalog = crate::object_task_catalog::tools();
    for (method, read) in [
        ("objectTask.revisePlanned", false),
        ("objectTask.revisions", true),
    ] {
        let tool = catalog.iter().find(|tool| tool["name"] == method).unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], read);
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    Ok(())
}
