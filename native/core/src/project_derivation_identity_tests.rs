use super::*;
use crate::{
    store::Store,
    validation::{flow::Definition, repository, requests},
};
use serde_json::{json, Value};

fn request() -> Request {
    Request {
        request_id: "copy-1".into(),
        source: std::env::temp_dir().join("original"),
        source_project_id: "p".into(),
        target_project_id: "derived".into(),
    }
}

fn setup(store: &mut Store) -> Result<String> {
    store.put("project", "p", &json!({"id":"p"}))?;
    store.put(
        "task",
        "task",
        &json!({"id":"task","projectId":"p","prompt":"p task run"}),
    )?;
    let definition: Definition = serde_json::from_value(json!({
        "key":"game", "name":"Game", "category":"feature", "purpose":"Check game",
        "taskIds":["task"], "steps":[{"id":"capture","kind":"capture"}]
    }))?;
    let flow = repository::save_flow(store, "p", definition, 0)?;
    store.put(
        "validationRun",
        "run",
        &json!({"id":"run","projectId":"p","taskId":"task","flow":flow}),
    )?;
    store.put(
        "validationRepairDecision",
        "run",
        &json!({"id":"run","projectId":"p","decision":{"result":{"taskId":"task"}}}),
    )?;
    store.put(
        "validationCoverage",
        "task",
        &json!({"taskId":"task","projectId":"p","flowIds":[flow.id],"runIds":["run"]}),
    )?;
    store.put(
        "asset-delivery/task",
        "candidate",
        &json!({"taskId":"task","projectId":"p","snapshotId":"immutable-content"}),
    )?;
    store.put(
        "feature",
        "p:semantic-key",
        &json!({"taskId":"task","projectId":"p"}),
    )?;
    store.connection.execute(
        "INSERT INTO calls(id,task,project,method,value) VALUES('call','task','p','test',?)",
        [json!({"id":"call","taskId":"task","projectId":"p","output":"task run p"}).to_string()],
    )?;
    let receipt = requests::Request::new("test", &json!({"projectId":"p","requestId":"same"}))?;
    requests::commit(store, vec![receipt.record(&json!({"runId":"run"}))])?;
    Ok(flow.id)
}

fn mapped<'a>(map: &'a IdentityMap, kind: &str, id: &str) -> &'a Key {
    let entry = map
        .entities
        .iter()
        .find(|entry| entry.source.kind == kind && entry.source.id == id)
        .unwrap();
    let Target::Remap { key } = &entry.target else {
        panic!("unexpected archive")
    };
    key
}

#[test]
fn stable_mapping_joins_related_keys_and_archives_unreplayable_receipts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let flow = setup(&mut store)?;
    let mut request = request();
    let map = build(&store.connection, &request)?;
    request.source = temp.path().join("source-offline");
    assert_eq!(map, build(&store.connection, &request)?);
    let task = &mapped(&map, "task", "task").id;
    let run = &mapped(&map, "validationRun", "run").id;
    assert!(uuid::Uuid::parse_str(task).is_ok());
    assert!(uuid::Uuid::parse_str(run).is_ok());
    assert!(uuid::Uuid::parse_str(&map.calls["call"]).is_ok());
    assert_ne!(task, run);
    assert_eq!(mapped(&map, "validationCoverage", "task").id, *task);
    assert_eq!(mapped(&map, "validationRepairDecision", "run").id, *run);
    let delivery = mapped(&map, "asset-delivery/task", "candidate");
    assert_eq!(delivery.kind, format!("asset-delivery/{task}"));
    assert_eq!(delivery.id, "candidate");
    assert_eq!(
        mapped(&map, "feature", "p:semantic-key").id,
        "derived:semantic-key"
    );
    let target_flow = &mapped(&map, "validationFlow", &flow).id;
    assert_eq!(
        mapped(&map, "validationFlowRevision", &format!("{flow}:1")).id,
        format!("{target_flow}:1")
    );
    assert!(map
        .entities
        .iter()
        .any(|entry| entry.source.kind == "validationRequest" && entry.target == Target::Archive));
    let old: Value = store.get("task", "task")?.unwrap();
    assert_eq!(old["prompt"], "p task run");
    request.request_id = "copy-2".into();
    assert_ne!(map, build(&store.connection, &request)?);
    request.target_project_id = "another".into();
    assert_ne!(
        mapped(
            &build(&store.connection, &request)?,
            "validationFlow",
            &flow
        )
        .id,
        *target_flow
    );
    Ok(())
}

#[test]
fn mapped_flow_key_supports_real_revision_lookup() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let flow_id = setup(&mut store)?;
    let map = build(&store.connection, &request())?;
    let mut flow: crate::validation::model::Flow = store.get("validationFlow", &flow_id)?.unwrap();
    flow.id = mapped(&map, "validationFlow", &flow_id).id.clone();
    flow.project_id = "derived".into();
    flow.definition.task_ids = vec![mapped(&map, "task", "task").id.clone()];
    let mut target = Store::open(&temp.path().join("target"))?;
    target.put("project", "derived", &json!({"id":"derived"}))?;
    requests::commit(&mut target, repository::flow_records(&flow)?)?;
    let next = repository::prepare_flow(&target, "derived", flow.definition, 1, "new revision")?;
    assert_eq!(next.id, flow.id);
    assert_eq!(next.revision, 2);
    Ok(())
}

#[test]
fn rejects_mismatched_primary_ids_and_flow_business_keys() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let flow = setup(&mut store)?;
    store.put(
        "validationRun",
        "run",
        &json!({"id":"wrong","projectId":"p"}),
    )?;
    assert!(build(&store.connection, &request())
        .unwrap_err()
        .to_string()
        .contains("entity identity mismatch"));
    store.put("validationRun", "run", &json!({"id":"run","projectId":"p"}))?;
    let mut value: Value = store.get("validationFlow", &flow)?.unwrap();
    value["definition"]["key"] = json!("different");
    store.put("validationFlow", &flow, &value)?;
    assert!(build(&store.connection, &request())
        .unwrap_err()
        .to_string()
        .contains("business key"));
    Ok(())
}
