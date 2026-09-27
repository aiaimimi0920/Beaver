use super::dispatch;
use crate::object_task_test_fixture::{commit_input, draft_input, Fixture};
use anyhow::Result;
use beaver_core::project_storage_router::ProjectStorageRouter;
use serde_json::{json, Value};

fn call(router: &ProjectStorageRouter, method: &str, input: &Value) -> Result<Value> {
    dispatch(router, method, Some(input))
        .expect("object task route")
        .map_err(anyhow::Error::msg)
}

#[test]
fn planning_metadata_crosses_desktop_save_commit_revision_and_reopen() -> Result<()> {
    let f = Fixture::new()?;
    let mut input = draft_input();
    for task in input["plan"]["tasks"].as_array_mut().unwrap() {
        task["requirement"] = json!("required");
        task["pendingPlanning"] = json!("");
    }
    input["plan"]["tasks"][1]["requirement"] = json!("optional");
    input["plan"]["tasks"][1]["pendingPlanning"] = json!("Lighting remains unplanned");
    let saved = call(&f.router, "objectTask.saveDraft", &input)?;
    assert_eq!(saved["plan"]["tasks"][1]["requirement"], "optional");
    assert_eq!(
        saved["plan"]["tasks"][1]["pendingPlanning"],
        "Lighting remains unplanned"
    );
    call(&f.router, "objectTask.commit", &commit_input())?;
    let target = json!({"projectId":"p","taskId":"medium"});
    let before = call(&f.router, "objectTask.get", &target)?;
    assert_eq!(before["requirement"], "optional");
    assert_eq!(before["pendingPlanning"], "Lighting remains unplanned");
    let request = json!({
        "projectId":"p","taskId":"medium","requestId":"planning-metadata",
        "expectedTaskRevision":0,"expectedPlanRevision":1,
        "definition":{
            "title":"Hero","prompt":"Make hero","acceptance":"Moves",
            "requirement":"required","pendingPlanning":"Lighting and export remain unplanned",
            "dependsOn":[]
        },
        "reason":"The hero is necessary; record the outstanding planning scope"
    });
    let receipt = call(&f.router, "objectTask.revisePlanned", &request)?;
    assert_eq!(receipt["before"]["requirement"], "optional");
    assert_eq!(
        receipt["after"]["pendingPlanning"],
        request["definition"]["pendingPlanning"]
    );
    assert_eq!(receipt["adoptedBy"], "owner");
    let after = call(&f.router, "objectTask.get", &target)?;
    // Required remains the backwards-compatible omitted serialization default.
    assert!(after.get("requirement").is_none());
    assert_eq!(
        after["pendingPlanning"],
        request["definition"]["pendingPlanning"]
    );
    assert_eq!(after["status"], "planned");
    let Fixture {
        router,
        host,
        temp: _temp,
    } = f;
    drop(router);
    let reopened = ProjectStorageRouter::new(host.clone());
    drop(reopened.open_registered("p")?);
    assert_eq!(call(&reopened, "objectTask.get", &target)?, after);
    assert_eq!(
        call(&reopened, "objectTask.revisePlanned", &request)?,
        receipt
    );
    assert_eq!(
        call(&reopened, "objectTask.revisions", &target)?,
        json!([receipt])
    );
    let mut conflict = request;
    conflict["definition"]["pendingPlanning"] = json!("");
    assert!(call(&reopened, "objectTask.revisePlanned", &conflict)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_TASK_REQUEST_ID_CONFLICT"));
    assert!(host
        .lock()
        .unwrap()
        .list::<Value>("object_task")?
        .is_empty());
    Ok(())
}

#[test]
fn desktop_rejects_invalid_planning_metadata_without_writing() -> Result<()> {
    let f = Fixture::new()?;
    for (index, field, value) in [
        (1, "requirement", json!("recommended")),
        (1, "requirement", json!(null)),
        (
            2,
            "pendingPlanning",
            json!("Fine tasks cannot have unexpanded work"),
        ),
        (1, "pendingPlanning", json!("界".repeat(1_334))),
        (1, "pendingPlanning", json!(false)),
    ] {
        let mut input = draft_input();
        input["plan"]["tasks"][index][field] = value;
        assert!(call(&f.router, "objectTask.saveDraft", &input).is_err());
    }
    assert!(call(
        &f.router,
        "objectTask.getDraft",
        &json!({"projectId":"p","draftId":"draft"})
    )?
    .is_null());
    call(&f.router, "objectTask.saveDraft", &draft_input())?;
    call(&f.router, "objectTask.commit", &commit_input())?;
    let before = call(&f.router, "objectTask.snapshot", &json!({"projectId":"p"}))?;
    for (task_id, field, value) in [
        ("medium", "requirement", json!("recommended")),
        ("fine", "pendingPlanning", json!("Nested work")),
        ("medium", "pendingPlanning", json!("界".repeat(1_334))),
    ] {
        let request = json!({
            "projectId":"p","taskId":task_id,"requestId":"invalid-metadata",
            "expectedTaskRevision":0,"expectedPlanRevision":1,
            "definition":{"title":"Changed","prompt":"Changed","acceptance":"Check","dependsOn":[],field:value},
            "reason":"Invalid definition"
        });
        assert!(call(&f.router, "objectTask.revisePlanned", &request).is_err());
    }
    assert_eq!(
        call(&f.router, "objectTask.snapshot", &json!({"projectId":"p"}))?,
        before
    );
    Ok(())
}

#[test]
fn discovery_advertises_metadata_on_both_write_contracts() {
    let tools = crate::object_task_catalog::tools();
    let schema =
        |method: &str| &tools.iter().find(|tool| tool["name"] == method).unwrap()["inputSchema"];
    let draft =
        &schema("objectTask.saveDraft")["properties"]["plan"]["properties"]["tasks"]["items"];
    let definition = &schema("objectTask.revisePlanned")["properties"]["definition"];
    for item in [draft, definition] {
        assert_eq!(item["additionalProperties"], false);
        let properties = &item["properties"];
        assert_eq!(
            properties["requirement"]["enum"],
            json!(["required", "optional"])
        );
        assert_eq!(properties["requirement"]["default"], "required");
        assert_eq!(properties["pendingPlanning"]["type"], "string");
        assert_eq!(properties["pendingPlanning"]["default"], "");
        assert_eq!(properties["pendingPlanning"]["maxLength"], 4_000);
        assert!(properties["pendingPlanning"]["description"]
            .as_str()
            .unwrap()
            .contains("UTF-8 byte"));
        // Legacy requests without either field remain valid.
        let required = item["required"].as_array().unwrap();
        assert!(!required.contains(&json!("requirement")));
        assert!(!required.contains(&json!("pendingPlanning")));
    }
}
