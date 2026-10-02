use super::*;
use beaver_core::object_tasks;

impl Fixture {
    pub(super) fn task_api(&self, method: &str, input: Value) -> Result<Value> {
        crate::data_dispatch::call_object_tasks_for_test(&self.router, method, input)
            .map_err(anyhow::Error::msg)
    }
}

#[test]
fn migration_derivation_plan_api_continues_production_draft_and_reopens_without_dispatch(
) -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    object_tasks::save_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","draftId":"history","expectedRevision":0,"expectedPlanRevision":0,
            "plan":{"tasks":[{"id":"root","granularity":"coarse","title":"Root","prompt":"Manual root","acceptance":""}]}
        }))?,
    )?;
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"history","draftId":"history","expectedDraftRevision":1,"expectedPlanRevision":0
        }))?,
    )?;
    object_tasks::save_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","draftId":"object-task-plan","expectedRevision":0,"expectedPlanRevision":1,
            "plan":{
                "objects":[{"id":"object","name":"Manual object"}],
                "tasks":[
                    {"id":"medium","granularity":"medium","title":"Build","prompt":"Keep original text","acceptance":"",
                        "objectId":"object","parentTaskId":"root"},
                    {"id":"fine","granularity":"fine","title":"Model","prompt":"Model stage","acceptance":"",
                        "objectId":"object","parentTaskId":"medium","stageId":"model"}
                ]
            }
        }))?,
    )?;
    drop(runtime);
    let before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let id = inspection["request"]["targetProjectId"].as_str().unwrap();
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    let preparation_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let query = json!({"projectId":id,"draftId":"object-task-plan"});
    let mut draft = f.task_api("objectTask.getDraft", query.clone())?;
    assert_eq!(draft["projectId"], id);
    assert_eq!(draft["id"], "object-task-plan");
    assert_eq!(draft["plan"]["tasks"][0]["prompt"], "Keep original text");
    let medium = draft["plan"]["tasks"][0]["id"].clone();
    assert_ne!(medium, "medium");
    assert_eq!(draft["plan"]["tasks"][1]["parentTaskId"], medium);
    let snapshot = f.task_api("objectTask.snapshot", json!({"projectId":id}))?;
    assert_eq!(snapshot["planRevision"], 1);
    assert_eq!(
        draft["plan"]["tasks"][0]["parentTaskId"],
        snapshot["tasks"][0]["id"]
    );
    draft["plan"]["tasks"][0]["prompt"] = json!("Edited in derived project");
    let saved = f.task_api("objectTask.saveDraft", json!({
        "projectId":id,"draftId":"object-task-plan","expectedRevision":1,"expectedPlanRevision":1,"plan":draft["plan"]
    }))?;
    assert_eq!(saved["revision"], 2);
    let commit = json!({"projectId":id,"requestId":"continue","draftId":"object-task-plan","expectedDraftRevision":2,"expectedPlanRevision":1});
    let receipt = f.task_api("objectTask.commit", commit.clone())?;
    assert_eq!(receipt["planRevision"], 2);
    assert_eq!(f.task_api("objectTask.commit", commit)?, receipt);
    let mut definition = json!({"title":"Revised copy","prompt":"Edited in derived project","acceptance":"","dependsOn":[]});
    definition["requirement"] = json!("optional");
    f.task_api("objectTask.revisePlanned", json!({
        "projectId":id,"requestId":"revise-copy","taskId":medium,"expectedTaskRevision":0,"expectedPlanRevision":2,
        "definition":definition,"reason":"Continue manual work"
    }))?;
    let unlocked = f.task_api("objectTask.unlockDraft", json!({
        "projectId":id,"requestId":"unlock-copy","draftId":"object-task-plan","expectedRevision":3,"expectedPlanRevision":3
    }))?;
    assert_eq!(unlocked["revision"], 4);
    f.task_api("objectTask.cancelPlanned", json!({
        "projectId":id,"requestId":"cancel-copy","taskId":medium,"expectedTaskRevision":1,"expectedPlanRevision":3
    }))?;
    let current = f.task_api("objectTask.snapshot", json!({"projectId":id}))?;
    assert_eq!(current["planRevision"], 4);
    assert_eq!(
        current["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["status"] == "cancelled")
            .count(),
        2
    );
    let claim = json!({"projectId":id,"owner":"desktop-derivation-test"});
    assert_eq!(
        f.task_api("objectTask.queue", json!({"projectId":id}))?,
        json!([])
    );
    assert_eq!(f.task_api("objectTask.claim", claim.clone())?, Value::Null);
    f.router.close(id)?;
    f.router.open_registered(id)?;
    assert_eq!(
        f.task_api("objectTask.snapshot", json!({"projectId":id}))?,
        current
    );
    assert_eq!(f.task_api("objectTask.getDraft", query)?, unlocked);
    assert_eq!(
        f.task_api(
            "objectTask.revisions",
            json!({"projectId":id,"taskId":medium})
        )?
        .as_array()
        .unwrap()
        .len(),
        1
    );
    assert_eq!(f.task_api("objectTask.claim", claim)?, Value::Null);
    assert_eq!(data_backup::inventory(&f.source)?, before);
    assert_eq!(data_backup::inventory(&f.preparation)?, preparation_before);
    Ok(())
}
