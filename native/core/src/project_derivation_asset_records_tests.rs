use super::*;
use crate::{
    asset_delivery_review, asset_feedback, asset_reference, asset_task,
    project_derivation_copy::Request, project_derivation_identity, store::Store,
};
use serde_json::json;

fn fixture(store: &Store) -> Result<IdentityMap> {
    store.put("project", "source", &json!({"id":"source"}))?;
    let task = json!({"id":"task","projectId":"source"});
    store.put("task", "task", &task)?;
    let mut state = asset_task::initial(&task)?;
    let reference: Reference = serde_json::from_value(json!({
        "id":"reference","taskId":"task","projectId":"source",
        "imagePath":"source/reference.png","sha256":"content","used":false,
        "frame":{"id":"frame","sessionId":"session","generation":"generation",
            "sceneRevision":1,"viewRevision":2,"capturedAt":3,"width":100,"height":100,
            "viewMatrix":vec![0.0;16],"projectionMatrix":vec![0.0;16]},
        "pick":null
    }))?;
    store.put("asset-reference", &reference.id, &reference)?;
    state.last_frame = Some(reference.clone());
    let input = Submission {
        id: "task".into(),
        feedback_id: "feedback".into(),
        timing: "now".into(),
        text: "Keep task reference source text".into(),
        reference_id: "reference".into(),
        annotations: vec![],
    };
    let feedback = asset_feedback::build(&mut state, input, reference)?;
    let followup = json!({
        "id":"followup","projectId":"source","assetFeedbackSeed":feedback,
        "assetRestore":".beaver/workspaces/.codex/task/asset-checkpoints/saved.blend"
    });
    store.put("task", "followup", &followup)?;
    store.put(
        "operation",
        "followup-op",
        &json!({
            "id":"followup-op","taskId":"followup","projectId":"source",
            "taskAfter":followup
        }),
    )?;
    state.feedback.push(feedback);
    state.delivery = Some(serde_json::from_value(json!({
        "templateId":"template","templateVersion":1,"approved":[],"pending":"candidate","history":["candidate"]
    }))?);
    asset_task::save(store, &state)?;
    store.put("asset-delivery/task", "candidate", &json!({
        "id":"candidate","taskId":"task","stageId":"stage","templateId":"template",
        "templateVersion":1,"inputCandidates":[],"attemptIds":["attempt"],"files":{},
        "summary":"task source reference","threadId":"thread","turnId":"turn","sessionId":"session","createdAt":"then"
    }))?;
    let decision = Decision {
        id: "task".into(),
        request_id: "request".into(),
        expected_revision: 1,
        candidate_id: "candidate".into(),
        decision: "approve".into(),
        note: "task".into(),
    };
    store.put("asset-delivery-decisions/task", "request", &json!({
        "request":decision,"response":{"candidateId":"candidate","status":"queued"},"source":"owner"
    }))?;
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
fn asset_readers_and_feedback_replay_use_converted_global_identities() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(&temp.path().join("source"))?;
    let map = fixture(&source)?;
    let target = Store::open(&temp.path().join("target"))?;
    for kind in [
        "asset-reference",
        "asset-task",
        "asset-delivery/task",
        "asset-delivery-decisions/task",
    ] {
        for (id, value) in source.list_with_ids::<Value>(kind)? {
            let (key, converted) = rewrite(&map, kind, &id, &value)?;
            target.put(&key.kind, &key.id, &converted)?;
        }
    }
    let task = Rewrite(&map).key("task", "task")?.id;
    let reference_id = Rewrite(&map).key("asset-reference", "reference")?.id;
    assert_ne!(reference_id, "reference");
    let reference = asset_reference::get(&target, &task, &reference_id)?;
    assert_eq!(reference.project_id, "target");
    assert_eq!(reference.frame.id, "frame");
    let state = asset_task::get(&target, &task)?;
    assert_eq!(state.last_frame.as_ref().unwrap().id, reference.id);
    let feedback = &state.feedback[0];
    assert_eq!(feedback.reference.id, reference.id);
    assert_eq!(feedback.task_id, task);
    assert_eq!(feedback.source_task_id, task);
    let input = submission(feedback);
    assert!(asset_feedback::duplicate(&state, &input)?.is_some());
    let old = asset_task::get(&source, "task")?;
    assert_ne!(feedback.fingerprint, old.feedback[0].fingerprint);
    assert_eq!(feedback.text, old.feedback[0].text);
    assert_eq!(feedback.id, old.feedback[0].id);
    assert_eq!(reference.image_path, old.last_frame.unwrap().image_path);
    let inspected = asset_delivery_review::inspect(&target, &task)?;
    let candidate = &inspected["candidates"][0];
    assert_eq!(candidate["taskId"], task);
    assert_eq!(candidate["id"], "candidate");
    assert_eq!(candidate["attemptIds"], json!(["attempt"]));
    assert_eq!(candidate["threadId"], "thread");
    let decision: Decision = serde_json::from_value(inspected["decisions"][0]["request"].clone())?;
    assert_eq!(decision.id, task);
    assert_eq!(
        asset_delivery_review::duplicate(&target, &decision)?.unwrap()["candidateId"],
        "candidate"
    );
    assert_eq!(
        asset_task::get(&source, "task")?.feedback[0].reference.id,
        "reference"
    );
    Ok(())
}

#[test]
fn rejects_corrupt_feedback_fingerprints_and_wrong_namespaced_owners() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(temp.path())?;
    let map = fixture(&source)?;
    let mut state: Value = source.get("asset-task", "task")?.unwrap();
    state["feedback"][0]["fingerprint"] = json!("corrupt");
    assert!(rewrite(&map, "asset-task", "task", &state)
        .unwrap_err()
        .to_string()
        .contains("fingerprint"));
    let mut candidate: Value = source.get("asset-delivery/task", "candidate")?.unwrap();
    candidate["taskId"] = json!("other");
    assert!(rewrite(&map, "asset-delivery/task", "candidate", &candidate).is_err());
    let mut decision: Value = source
        .get("asset-delivery-decisions/task", "request")?
        .unwrap();
    decision["request"]["requestId"] = json!("other");
    assert!(rewrite(&map, "asset-delivery-decisions/task", "request", &decision).is_err());
    Ok(())
}

#[test]
fn derived_followup_seed_supports_initialization_and_feedback_replay() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = Store::open(temp.path())?;
    let map = fixture(&source)?;
    let original: Value = source.get("task", "followup")?.unwrap();
    let (_, converted) =
        crate::project_derivation_task_records::rewrite(&map, "task", "followup", &original)?;
    let state = asset_task::initial(&converted)?;
    let feedback = &state.feedback[0];
    assert_eq!(feedback.task_id, Rewrite(&map).key("task", "followup")?.id);
    assert_eq!(
        feedback.source_task_id,
        Rewrite(&map).key("task", "task")?.id
    );
    assert_eq!(feedback.reference.task_id, feedback.source_task_id);
    assert_eq!(feedback.reference.project_id, "target");
    assert_eq!(
        feedback.reference.id,
        Rewrite(&map).key("asset-reference", "reference")?.id
    );
    assert!(asset_feedback::duplicate(&state, &submission(feedback))?.is_some());
    assert_ne!(
        converted["assetFeedbackSeed"]["fingerprint"],
        original["assetFeedbackSeed"]["fingerprint"]
    );
    for field in ["text", "history", "checkpoint", "id"] {
        assert_eq!(
            converted["assetFeedbackSeed"][field],
            original["assetFeedbackSeed"][field]
        );
    }
    assert_eq!(
        converted["assetFeedbackSeed"]["reference"]["frame"],
        original["assetFeedbackSeed"]["reference"]["frame"]
    );
    let operation: Value = source.get("operation", "followup-op")?.unwrap();
    let (_, journal) = crate::project_derivation_task_records::rewrite(
        &map,
        "operation",
        "followup-op",
        &operation,
    )?;
    assert_eq!(journal["taskAfter"], converted);
    assert_eq!(source.get::<Value>("task", "followup")?.unwrap(), original);
    let mut corrupt = original.clone();
    corrupt["assetFeedbackSeed"]["fingerprint"] = json!("corrupt");
    assert!(
        crate::project_derivation_task_records::rewrite(&map, "task", "followup", &corrupt)
            .is_err()
    );
    let mut missing = original;
    missing["assetFeedbackSeed"]["taskId"] = json!("missing");
    assert!(
        crate::project_derivation_task_records::rewrite(&map, "task", "followup", &missing)
            .is_err()
    );
    Ok(())
}
