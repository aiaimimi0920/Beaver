use super::*;
use beaver_core::{object_task_planning_declaration as declaration, object_tasks};

fn tasks(f: &Fixture, method: &str, input: Value) -> Result<Value> {
    crate::data_dispatch::call_object_tasks_for_test(&f.router, method, input)
        .map_err(anyhow::Error::msg)
}

fn state<'a>(snapshot: &'a Value, task: &str) -> &'a Value {
    snapshot["planningStates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["taskId"] == task)
        .unwrap()
}

fn request(snapshot: &Value, project: &str, task: &str, id: &str) -> Value {
    json!({"projectId":project,"taskId":task,"requestId":id,
        "expectedPlanRevision":snapshot["planRevision"],"expectedScopeHash":state(snapshot,task)["scopeHash"],
        "reason":"Owner reviewed the entire copied planning scope"})
}

#[test]
fn migration_derivation_declaration_api_views_replays_reconfirms_and_reopens_without_acceptance(
) -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let source = ProjectStore::open(&f.source, "original")?.into_runtime();
    object_tasks::save_draft(
        &source,
        &serde_json::from_value(json!({
            "projectId":"original","draftId":"plan","expectedRevision":0,"expectedPlanRevision":0,
            "plan":{"objects":[{"id":"object","name":"Planned object"}],"tasks":[
                {"id":"root","granularity":"coarse","title":"Root","prompt":"Root goal","acceptance":""},
                {"id":"build","granularity":"medium","title":"Build","prompt":"Build goal","acceptance":"",
                    "objectId":"object","parentTaskId":"root"},
                {"id":"shape","granularity":"fine","title":"Shape","prompt":"Shape goal","acceptance":"",
                    "objectId":"object","parentTaskId":"build","stageId":"model"}
            ]}
        }))?,
    )?;
    object_tasks::commit(
        &source,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"commit","draftId":"plan","expectedDraftRevision":1,"expectedPlanRevision":0
        }))?,
    )?;
    let initial = serde_json::to_value(object_tasks::snapshot(&source, "original")?)?;
    let older = declaration::declare(
        &source,
        &serde_json::from_value(request(&initial, "original", "root", "old-root"))?,
    )?;
    declaration::declare(
        &source,
        &serde_json::from_value(request(&initial, "original", "build", "old-build"))?,
    )?;
    object_tasks::revise_planned(
        &source,
        &serde_json::from_value(json!({
            "projectId":"original","taskId":"shape","requestId":"revise-source","expectedTaskRevision":0,"expectedPlanRevision":1,
            "definition":{"title":"Shape","prompt":"Revised shape goal","acceptance":"","dependsOn":[]},"reason":"More scope"
        }))?,
    )?;
    let revised = serde_json::to_value(object_tasks::snapshot(&source, "original")?)?;
    let latest = declaration::declare(
        &source,
        &serde_json::from_value(request(&revised, "original", "root", "latest-root"))?,
    )?;
    object_tasks::save_draft(
        &source,
        &serde_json::from_value(json!({
            "projectId":"original","draftId":"unrelated","expectedRevision":0,"expectedPlanRevision":2,
            "plan":{"tasks":[{"id":"unrelated","granularity":"coarse","title":"Unrelated","prompt":"Other scope","acceptance":""}]}
        }))?,
    )?;
    object_tasks::commit(
        &source,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"unrelated","draftId":"unrelated","expectedDraftRevision":1,"expectedPlanRevision":2
        }))?,
    )?;
    let original = serde_json::to_value(object_tasks::snapshot(&source, "original")?)?;
    assert_eq!(state(&original, "root")["current"], true);
    assert_eq!(state(&original, "build")["current"], false);
    drop(source);
    let before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let project = inspection["request"]["targetProjectId"].as_str().unwrap();
    let mut input = inspection["request"].clone();
    input["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", input)?;
    let prepared = beaver_core::project_derivation_copy::inspect(&f.preparation)?;
    let mapped = |kind: &str, id: &str| -> String {
        let entry = prepared
            .identities
            .entities
            .iter()
            .find(|e| e.source.kind == kind && e.source.id == id)
            .unwrap();
        match &entry.target {
            beaver_core::project_derivation_identity::Target::Remap { key } => key.id.clone(),
            _ => panic!("unexpected archived declaration"),
        }
    };
    let root = mapped("object_task", "root");
    let build = mapped("object_task", "build");
    let shape = mapped("object_task", "shape");
    let prepared_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    f.api("migration.registerAssembly", f.paths())?;
    let snapshot = tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?;
    assert_eq!(snapshot["planRevision"], 3);
    assert_eq!(state(&snapshot, &root)["current"], true);
    assert_eq!(state(&snapshot, &build)["current"], false);
    let current_declaration = &state(&snapshot, &root)["declaration"];
    assert_eq!(
        current_declaration["request"]["reason"],
        latest.request.reason
    );
    assert_eq!(current_declaration["createdAt"], latest.created_at);
    assert_eq!(current_declaration["declaredBy"], "owner");
    assert_eq!(current_declaration["request"]["expectedPlanRevision"], 2);
    let mut expected = vec![root.clone(), build.clone(), shape.clone()];
    expected.sort();
    assert_eq!(current_declaration["taskIds"], json!(expected));
    assert_ne!(
        current_declaration["request"]["expectedScopeHash"],
        latest.request.expected_scope_hash
    );
    for id in ["old-root", "old-build", "latest-root"] {
        let runtime = f.router.runtime_for_project(project)?;
        let receipt = runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>(
                "object_task_planning_declaration_receipt",
                &mapped("object_task_planning_declaration_receipt", id),
            )?
            .unwrap();
        if id == "old-root" {
            assert_eq!(receipt["request"]["reason"], older.request.reason);
        }
        let replay = tasks(
            &f,
            "objectTask.declarePlanningComplete",
            receipt["request"].clone(),
        )?;
        assert_eq!(replay, receipt);
        let mut conflict = receipt["request"].clone();
        conflict["reason"] = json!("different copied request");
        assert!(tasks(&f, "objectTask.declarePlanningComplete", conflict)
            .unwrap_err()
            .to_string()
            .contains("REQUEST_CONFLICT"));
    }
    assert_eq!(
        tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?,
        snapshot
    );
    f.router.close(project)?;
    f.router.open_registered(project)?;
    assert_eq!(
        tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?,
        snapshot
    );
    // Explicit planning edits invalidate the copied confirmation; ordinary UI confirmation restores it.
    tasks(
        &f,
        "objectTask.revisePlanned",
        json!({
            "projectId":project,"taskId":shape,"requestId":"revise-copy","expectedTaskRevision":1,"expectedPlanRevision":3,
            "definition":{"title":"Shape","prompt":"Copy-only change","acceptance":"","dependsOn":[]},"reason":"Independent scope"
        }),
    )?;
    let changed = tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?;
    assert_eq!(state(&changed, &root)["current"], false);
    let confirm = request(&changed, project, &root, "confirm-copy");
    let receipt = tasks(&f, "objectTask.declarePlanningComplete", confirm.clone())?;
    assert_eq!(
        tasks(&f, "objectTask.declarePlanningComplete", confirm)?,
        receipt
    );
    let after = tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?;
    assert_eq!(state(&after, &root)["current"], true);
    assert_eq!(state(&after, &build)["current"], false);
    for field in ["tasks", "runs", "planRevision", "assumptions"] {
        assert_eq!(after[field], changed[field]);
    }
    assert!(after["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["status"] == "planned"));
    assert_eq!(
        tasks(&f, "objectTask.queue", json!({"projectId":project}))?,
        json!([])
    );
    assert_eq!(
        tasks(
            &f,
            "objectTask.claim",
            json!({"projectId":project,"owner":"declaration-migration-test"})
        )?,
        Value::Null
    );
    f.router.close(project)?;
    f.router.open_registered(project)?;
    f.api("migration.registerAssembly", f.paths())?;
    assert_eq!(
        tasks(&f, "objectTask.snapshot", json!({"projectId":project}))?,
        after
    );
    assert_eq!(data_backup::inventory(&f.source)?, before);
    assert_eq!(data_backup::inventory(&f.preparation)?, prepared_before);
    assert!(f
        .store
        .lock()
        .unwrap()
        .list::<Value>("object_task_planning_declaration")?
        .is_empty());
    Ok(())
}
