use super::*;
use crate::{
    project_derivation_copy::Request,
    project_derivation_identity,
    store::Store,
    validation::{
        flow::Definition,
        model::{Flow, Run},
        requests,
    },
};
use serde_json::json;

fn fixture(store: &mut Store) -> Result<Release> {
    store.put("project", "source", &json!({"id":"source"}))?;
    store.put("task", "task", &json!({"id":"task","projectId":"source"}))?;
    let definition: Definition = serde_json::from_value(json!({
        "key":"game", "name":"Game", "category":"feature", "purpose":"source task",
        "taskIds":["task"], "steps":[{"id":"capture","kind":"capture"}]
    }))?;
    let flow = repository::save_flow(store, "source", definition, 0)?;
    let mut release = Release {
        id: "release".into(),
        project_id: "source".into(),
        snapshot: Default::default(),
        snapshot_id: repository::digest(&json!({}))?,
        preset: "Windows".into(),
        visual_required: true,
        policy_version: "historical-policy".into(),
        flow_ids: vec![flow.id.clone()],
        flows: vec![flow.clone()],
        scope_id: String::new(),
        excluded: vec![],
        missing: vec![],
        run_ids: vec![],
        created_at: "then".into(),
        exports: vec![],
    };
    for flow in [None, Some(flow)] {
        let mut run = repository::build_run(
            store,
            "source",
            Default::default(),
            flow,
            Some("task".into()),
            Some(release.id.clone()),
        )?;
        run.log = "source task release".into();
        run.confirmations.push(json!({"runId":run.id,"snapshotId":run.snapshot_id,"requestId":"keep-request","evidenceIds":["local-evidence"]}));
        store.put("validationRun", &run.id, &run)?;
        release.run_ids.push(run.id);
    }
    release.scope_id = release::scope(&release)?;
    store.put("validationRelease", &release.id, &release)?;
    let receipt =
        requests::Request::new("test", &json!({"projectId":"source","requestId":"receipt"}))?;
    requests::commit(store, vec![receipt.record(&json!({"id":release.id}))])?;
    Ok(release)
}

fn mapping(store: &Store) -> Result<IdentityMap> {
    project_derivation_identity::build(
        &store.connection,
        &Request {
            request_id: "derive".into(),
            source: std::env::temp_dir().join("source"),
            source_project_id: "source".into(),
            target_project_id: "target".into(),
        },
    )
}

#[test]
fn converted_release_passes_real_inspection_and_flow_revision_lookup() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut source = Store::open(&temp.path().join("source"))?;
    let original = fixture(&mut source)?;
    let map = mapping(&source)?;
    let target = Store::open(&temp.path().join("target"))?;
    target.put("project", "target", &json!({"id":"target"}))?;
    let task = Rewrite(&map).key("task", "task")?;
    target.put(
        "task",
        &task.id,
        &json!({"id":task.id,"projectId":"target"}),
    )?;
    for kind in [
        "validationFlow",
        "validationFlowRevision",
        "validationRun",
        "validationRelease",
    ] {
        for value in source.list::<Value>(kind)? {
            let id = if kind == "validationFlowRevision" {
                format!("{}:{}", value["id"].as_str().unwrap(), value["revision"])
            } else {
                value["id"].as_str().unwrap().to_owned()
            };
            let (key, converted) = rewrite(&map, kind, &id, &value)?;
            target.put(&key.kind, &key.id, &converted)?;
        }
    }
    let converted: Release = target.list("validationRelease")?.pop().unwrap();
    assert_ne!(converted.scope_id, original.scope_id);
    assert_eq!(converted.snapshot_id, original.snapshot_id);
    let report = release::inspect(&target, &converted)?;
    assert_eq!(report["ready"], false);
    assert_eq!(converted.flows[0].definition.task_ids, vec![task.id]);
    assert_eq!(converted.flows[0].definition.purpose, "source task");
    assert_eq!(
        converted.flows[0].definition.signature()?,
        original.flows[0].definition.signature()?
    );
    for run in target.list::<Run>("validationRun")? {
        assert_eq!(run.confirmations[0]["runId"], run.id);
        assert_eq!(run.confirmations[0]["requestId"], "keep-request");
        assert_eq!(run.confirmations[0]["evidenceIds"][0], "local-evidence");
        assert_eq!(run.log, "source task release");
    }
    let next: Flow = repository::prepare_flow(
        &target,
        "target",
        converted.flows[0].definition.clone(),
        1,
        "next",
    )?;
    assert_eq!(next.revision, 2);
    assert_eq!(next.id, converted.flows[0].id);
    let unchanged: Release = source.get("validationRelease", "release")?.unwrap();
    assert_eq!(
        serde_json::to_value(unchanged)?,
        serde_json::to_value(original)?
    );
    Ok(())
}

#[test]
fn refuses_corrupt_snapshots_scopes_missing_mappings_and_replay_receipts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let release = fixture(&mut store)?;
    let map = mapping(&store)?;
    let receipt = map
        .entities
        .iter()
        .find(|e| e.source.kind == "validationRequest")
        .unwrap();
    let value: Value = store.get("validationRequest", &receipt.source.id)?.unwrap();
    assert!(
        rewrite(&map, "validationRequest", &receipt.source.id, &value)
            .unwrap_err()
            .to_string()
            .contains("replay state")
    );
    let mut value = serde_json::to_value(&release)?;
    value["scopeId"] = json!("tampered");
    assert!(rewrite(&map, "validationRelease", "release", &value)
        .unwrap_err()
        .to_string()
        .contains("integrity"));
    let mut run: Value = store.get("validationRun", &release.run_ids[0])?.unwrap();
    run["id"] = json!(release.run_ids[1]);
    assert!(rewrite(&map, "validationRun", &release.run_ids[0], &run)
        .unwrap_err()
        .to_string()
        .contains("identity mismatch"));
    run["id"] = json!(release.run_ids[0]);
    run["snapshotId"] = json!("tampered");
    assert!(rewrite(&map, "validationRun", &release.run_ids[0], &run)
        .unwrap_err()
        .to_string()
        .contains("digest"));
    run["snapshotId"] = json!(release.snapshot_id);
    run["confirmations"][0]["runId"] = json!("missing");
    assert!(rewrite(&map, "validationRun", &release.run_ids[0], &run)
        .unwrap_err()
        .to_string()
        .contains("mapping missing"));
    Ok(())
}

#[test]
fn rewrites_baseline_judgments_and_invalidates_coverage_cache() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let release = fixture(&mut store)?;
    let run = &release.run_ids[1];
    store.put("validationBaseline", "baseline", &json!({"id":"baseline","projectId":"source","flowId":release.flow_ids[0],"runId":run,"signature":"same-content","evidenceIds":["local"]}))?;
    store.put("validationCoverage", "task", &json!({"id":"task","taskId":"task","projectId":"source","codeRunId":release.run_ids[0],"flowIds":release.flow_ids,"runIds":release.run_ids,"watchSignature":"source-cache","title":"source task"}))?;
    let map = mapping(&store)?;
    let mut value: Value = store.get("validationRun", run)?.unwrap();
    value["judgments"] = json!([{"runId":run,"baselineId":"baseline","reason":"source task"}]);
    let (_, converted) = rewrite(&map, "validationRun", run, &value)?;
    assert_eq!(
        converted["judgments"][0]["baselineId"],
        Rewrite(&map).key("validationBaseline", "baseline")?.id
    );
    let value: Value = store.get("validationBaseline", "baseline")?.unwrap();
    let (_, converted) = rewrite(&map, "validationBaseline", "baseline", &value)?;
    assert_eq!(
        converted["runId"],
        Rewrite(&map).key("validationRun", run)?.id
    );
    assert_eq!(converted["signature"], "same-content");
    assert_eq!(converted["evidenceIds"][0], "local");
    let value: Value = store.get("validationCoverage", "task")?.unwrap();
    let (key, converted) = rewrite(&map, "validationCoverage", "task", &value)?;
    assert_eq!(converted["id"], key.id);
    assert_eq!(converted["taskId"], key.id);
    assert_ne!(converted["id"], "task");
    assert_eq!(
        converted["codeRunId"],
        Rewrite(&map).key("validationRun", &release.run_ids[0])?.id
    );
    assert!(converted.get("watchSignature").is_none());
    assert_eq!(converted["title"], "source task");
    Ok(())
}
