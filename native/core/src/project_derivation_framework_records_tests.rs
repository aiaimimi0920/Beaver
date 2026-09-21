use super::*;
use crate::{
    framework_checks, framework_evidence, framework_operations, project_derivation_asset_records,
    project_derivation_copy::Request, project_derivation_identity, store::Store,
};
use serde_json::json;

#[test]
fn converted_framework_gates_and_operation_readers_preserve_evidence() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(&temp.path().join("source"))?;
    source.put("project", "source", &json!({"id":"source"}))?;
    source.put("task", "task", &json!({"id":"task","projectId":"source"}))?;
    let configuration: Configuration = serde_json::from_value(json!({
        "revision":3,"plugins":[],"semanticRequired":true,
        "rules":[{"id":"rule","version":1,"checker":{"executable":temp.path().join("checker"),
            "sha256":"a".repeat(64),"args":[],"timeoutSeconds":1}}]
    }))?;
    source.put("framework-configuration", "task", &configuration)?;
    let candidate: Candidate = serde_json::from_value(json!({
        "id":"candidate","taskId":"task","stageId":"stage","templateId":"template",
        "templateVersion":1,"inputCandidates":[],"attemptIds":[],"files":{},
        "summary":"source task","threadId":"thread","turnId":"turn","sessionId":null,"createdAt":"then"
    }))?;
    source.put("asset-delivery/task", "candidate", &candidate)?;
    let report = json!({"candidateId":"candidate","candidateSha256":digest(&candidate)?,
        "configurationRevision":3,"configurationSha256":digest(&configuration)?,"passed":true,
        "source":"owner","verdict":"pass","note":"source task","visual":{}});
    for (kind, id) in [
        ("framework-check/task", "candidate"),
        ("framework-judgment/task", "owner-candidate"),
        ("framework-judgment-history/task", "history"),
    ] {
        source.put(kind, id, &report)?;
    }
    let operation = Operation {
        id: "operation".into(),
        task_id: "task".into(),
        request_id: "request".into(),
        job: Job::Check {
            candidate_id: "candidate".into(),
        },
        source: "owner".into(),
        thread_id: None,
        turn_id: None,
        status: "succeeded".into(),
        created_at: "then".into(),
        ended_at: Some("later".into()),
        error: None,
        result: Some(json!({"context":{"taskId":"task","workspace":"source/path"},"value":report})),
    };
    source.put("framework-operation", "operation", &operation)?;
    framework_evidence::start(
        &source,
        "task",
        "trace",
        "method",
        &json!({"id":"item","taskId":"opaque"}),
        None,
        false,
    )?;
    framework_evidence::finish(
        &source,
        "task",
        "trace",
        "succeeded",
        &json!({"taskId":"opaque output"}),
    )?;
    framework_evidence::recovery(&source, "task", "source task", Some("operation"))?;
    let reference: crate::asset_task::Reference = serde_json::from_value(json!({
        "id":"reference","taskId":"task","projectId":"source","imagePath":"missing.png","sha256":"image",
        "used":false,"frame":{"id":"frame","sessionId":"session","generation":"generation",
            "sceneRevision":1,"viewRevision":1,"capturedAt":0,"width":1,"height":1,
            "viewMatrix":vec![0.0;16],"projectionMatrix":vec![0.0;16]},"pick":null
    }))?;
    source.put("asset-reference", "reference", &reference)?;
    framework_evidence::observation(&source, "task", &reference)?;
    framework_checks::gate(&source, "task", &candidate)?;
    let map = project_derivation_identity::build(
        &source.connection,
        &Request {
            request_id: "derive".into(),
            source: temp.path().join("source"),
            source_project_id: "source".into(),
            target_project_id: "target".into(),
        },
    )?;
    let target = Store::open(&temp.path().join("target"))?;
    for kind in [
        "framework-configuration",
        "framework-operation",
        "framework-trace/task",
        "framework-observation/task",
        "framework-recovery/task",
        "framework-check/task",
        "framework-judgment/task",
        "framework-judgment-history/task",
    ] {
        for (id, value) in source.list_with_ids::<Value>(kind)? {
            let (key, converted) = rewrite(&source.connection, &map, kind, &id, &value)?;
            target.put(&key.kind, &key.id, &converted)?;
        }
    }
    let task = Rewrite(&map).key("task", "task")?.id;
    let (key, converted) = project_derivation_asset_records::rewrite(
        &map,
        "asset-delivery/task",
        "candidate",
        &json!(candidate),
    )?;
    target.put(&key.kind, &key.id, &converted)?;
    let converted: Candidate = serde_json::from_value(converted)?;
    framework_checks::gate(&target, &task, &converted)?;
    let op = framework_operations::get(
        &target,
        &task,
        &Rewrite(&map).key("framework-operation", "operation")?.id,
    )?;
    assert_eq!(op.result.as_ref().unwrap()["context"]["taskId"], task);
    assert_eq!(
        op.result.as_ref().unwrap()["value"]["candidateSha256"],
        digest(&converted)?
    );
    let recoveries: Vec<Value> = target.list(&format!("framework-recovery/{task}"))?;
    assert_eq!(recoveries[0]["operationId"], op.id);
    let observation: Value = target
        .get(
            &format!("framework-observation/{task}"),
            &Rewrite(&map).key("asset-reference", "reference")?.id,
        )?
        .unwrap();
    assert_eq!(observation["context"]["taskId"], task);
    assert_eq!(observation["frame"], json!(reference.frame));
    let trace: Value = target
        .get(
            &format!("framework-trace/{task}"),
            &Rewrite(&map).key("framework-trace/task", "trace")?.id,
        )?
        .unwrap();
    let original_trace: Value = source.get("framework-trace/task", "trace")?.unwrap();
    assert_eq!(trace["input"], original_trace["input"]);
    assert_eq!(trace["output"], original_trace["output"]);
    let mut stale = report.clone();
    stale["configurationRevision"] = json!(2);
    stale["configurationSha256"] = json!("old-configuration");
    let (key, converted_stale) = rewrite(
        &source.connection,
        &map,
        "framework-judgment/task",
        "owner-candidate",
        &stale,
    )?;
    target.put(&key.kind, &key.id, &converted_stale)?;
    assert!(framework_checks::gate(&target, &task, &converted).is_err());
    let mut corrupt = report.clone();
    corrupt["candidateSha256"] = json!("corrupt");
    assert!(rewrite(
        &source.connection,
        &map,
        "framework-check/task",
        "candidate",
        &corrupt
    )
    .is_err());
    framework_checks::gate(&source, "task", &candidate)?;
    Ok(())
}
