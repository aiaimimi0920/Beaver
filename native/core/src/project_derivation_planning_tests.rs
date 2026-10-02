use crate::{
    data_backup, object_task_planning as planning, object_task_planning_round as round,
    object_tasks, project_derivation_assembly as assembly, project_derivation_copy as copy,
    project_derivation_validation_records::Rewrite, project_storage::ProjectStore,
    project_storage_router::ProjectStorageRouter, store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "project_derivation_planning_boundary_tests.rs"]
mod boundaries;
#[path = "project_derivation_planning_continuation_tests.rs"]
mod continuation;
#[path = "project_derivation_planning_fixture.rs"]
mod fixture;
#[path = "project_derivation_object_fixture.rs"]
mod objects;
#[path = "project_derivation_planning_rejection_tests.rs"]
mod rejection;
use fixture::{current, Fixture};

#[test]
fn project_derivation_planning_preserves_history_and_explicitly_adopts_then_commits() -> Result<()>
{
    let f = Fixture::new()?;
    let before = data_backup::inventory(&f.base.source)?;
    copy::inspect_source(&f.base.source)?;
    let preparation = f.base.temp.path().join("prepared");
    let prepared = copy::prepare(f.base.request(), &preparation)?;
    let mapped = |kind: &str, id: &str| -> Result<String> {
        Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
    };
    let prepared_before = data_backup::inventory(&preparation)?;
    let target = f.base.temp.path().join("target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let data = f.base.temp.path().join("host");
    let router = ProjectStorageRouter::new(Arc::new(Mutex::new(Store::open(&data)?)));
    router.register_assembly(&preparation, &target, &data)?;
    let runtime = router.open_registered("derived")?;
    let adopted = current(&runtime, "adopted")?;
    assert_eq!(adopted.status, planning::Status::Adopted);
    assert_eq!(adopted.id, mapped("object_task_planning", &f.adopted.id)?);
    assert_eq!(adopted.input.request_id, adopted.id);
    assert_ne!(adopted.id, f.adopted.id);
    assert_ne!(adopted.round_id, f.adopted.round_id);
    assert_eq!(adopted.input.goal, f.adopted.input.goal);
    assert_eq!(adopted.decisions, f.adopted.decisions);
    assert_eq!(adopted.thread_id, f.adopted.thread_id);
    assert_eq!(adopted.turn_id, f.adopted.turn_id);
    let assumptions = &adopted.adopted_draft.as_ref().unwrap().plan.assumptions;
    assert!(assumptions.iter().all(|a| a
        .source_detail
        .as_ref()
        .unwrap()
        .starts_with(&format!("planning:{}", adopted.id))));
    let store = runtime.store();
    let record: Value = store
        .lock()
        .unwrap()
        .get("object_task_planning", &adopted.id)?
        .unwrap();
    let snapshot_task = record["context"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == mapped("object_task", "root").unwrap())
        .unwrap();
    assert_eq!(snapshot_task["prompt"], "Old root");
    assert_eq!(snapshot_task["revision"], 0);
    let build = record["context"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == mapped("object_task", "build").unwrap())
        .unwrap();
    assert_eq!(
        build["identity"]["baseline"]["selectedVersionId"],
        mapped("object_version", &f.base.first.version_id)?
    );
    assert_eq!(build["status"], "planned");
    assert!(record["context"]["runs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["projectId"] == "derived" && r["status"] == "planned"));
    for (draft, status) in [
        ("waiting", planning::Status::AwaitingInput),
        ("cancelled", planning::Status::Cancelled),
        ("failed", planning::Status::Failed),
        ("interrupted", planning::Status::Interrupted),
    ] {
        assert_eq!(current(&runtime, draft)?.status, status);
    }
    let cancel_value: Value = store
        .lock()
        .unwrap()
        .get(
            "object_task_planning_receipt",
            &mapped("object_task_planning_receipt", "cancel-twice")?,
        )?
        .unwrap();
    let cancel: planning::SessionRequest = serde_json::from_value(cancel_value["request"].clone())?;
    assert_eq!(
        planning::cancel(&runtime, &cancel)?.status,
        planning::Status::Cancelled
    );
    let mut answer = f.answer.clone();
    answer.project_id = "derived".into();
    answer.session_id = adopted.id.clone();
    answer.request_id = mapped("object_task_planning_receipt", &answer.request_id)?;
    assert!(!planning::answer(&runtime, &answer)?.launch);
    assert!(!planning::start(&runtime, &adopted.input)?.launch);
    let receipt: Value = store
        .lock()
        .unwrap()
        .get(
            "object_task_planning_receipt",
            &mapped("object_task_planning_receipt", "adopt-history")?,
        )?
        .unwrap();
    assert_eq!(
        planning::adopt(
            &runtime,
            &serde_json::from_value(receipt["request"].clone())?
        )?,
        adopted
    );
    let proposed = current(&runtime, "object-task-plan")?;
    assert_eq!(proposed.status, planning::Status::Proposed);
    assert!(proposed.conflict.is_none());
    assert_eq!(
        mapped("object_task_planning_head", "object-task-plan")?,
        "object-task-plan"
    );
    let adopted_now = planning::adopt(
        &runtime,
        &crate::object_task_planning_fixture::command(&proposed, "adopt-copy"),
    )?;
    let mut draft = adopted_now.adopted_draft.unwrap();
    assert_eq!(draft.plan.tasks[0].prompt, "original baseline");
    assert_eq!(
        draft.plan.tasks[1].prompt,
        "Do not replace original start-adopted in this text"
    );
    draft.plan.tasks[1].prompt.push_str(" edited in copy");
    object_tasks::save_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","draftId":draft.id,"expectedRevision":draft.revision,
            "expectedPlanRevision":draft.plan_revision,"plan":draft.plan
        }))?,
    )?;
    fixture::commit(&runtime, "object-task-plan", "commit-copy")?;
    let snapshot = object_tasks::snapshot(&runtime, "derived")?;
    assert!(snapshot
        .tasks
        .iter()
        .any(|t| t.id == draft.plan.tasks[1].id && t.prompt.ends_with("edited in copy")));
    assert!(object_tasks::queue(&runtime, "derived")?.is_empty());
    assert!(object_tasks::claim_next(&runtime, "derived", "copy-test")?.is_none());
    drop(store);
    drop(runtime);
    router.close("derived")?;
    let runtime = router.open_registered("derived")?;
    assert_eq!(object_tasks::snapshot(&runtime, "derived")?, snapshot);
    assert_eq!(current(&runtime, "adopted")?, adopted);
    let source = runtime.project_root().to_owned();
    drop(runtime);
    router.close("derived")?;
    copy::prepare(
        copy::Request {
            request_id: "next".into(),
            source,
            source_project_id: "derived".into(),
            target_project_id: "third".into(),
        },
        &f.base.temp.path().join("third"),
    )?;
    assert_eq!(data_backup::inventory(&f.base.source)?, before);
    assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    Ok(())
}
