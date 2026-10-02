use crate::{
    data_backup, object_tasks, project_derivation_assembly as assembly,
    project_derivation_copy as copy, project_derivation_identity::generated,
    project_derivation_validation_records::Rewrite, project_storage::ProjectStore,
    project_storage_router::ProjectStorageRouter, store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[path = "project_derivation_declaration_tests.rs"]
mod declarations;
#[path = "project_derivation_plan_fixture.rs"]
mod fixture;
#[path = "project_derivation_object_fixture.rs"]
mod objects;
#[path = "project_derivation_queue_tests.rs"]
mod queue;
#[path = "project_derivation_plan_rejection_tests.rs"]
mod rejection;
use fixture::{save, Fixture};

#[test]
fn project_derivation_plan_preserves_history_and_continues_manual_work_after_reopen() -> Result<()>
{
    let f = Fixture::new()?;
    let before = data_backup::inventory(&f.base.source)?;
    copy::inspect_source(&f.base.source)?;
    let preparation = f.base.temp.path().join("prepared-plan");
    let prepared = copy::prepare(f.base.request(), &preparation)?;
    let mapped = |kind: &str, id: &str| -> Result<String> {
        Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
    };
    let root = mapped("object_task", "root")?;
    let build = mapped("object_task", "build")?;
    let discard = mapped("object_task", "discard")?;
    assert_ne!(root, "root");
    assert_eq!(
        mapped("object_task_draft", "object-task-plan")?,
        "object-task-plan"
    );
    let prepared_before = data_backup::inventory(&preparation)?;
    let target = f.base.temp.path().join("assembled-plan");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let data = f.base.temp.path().join("plan-host");
    let router = ProjectStorageRouter::new(Arc::new(Mutex::new(Store::open(&data)?)));
    router.register_assembly(&preparation, &target, &data)?;
    let runtime = router.open_registered("derived")?;
    let snapshot = object_tasks::snapshot(&runtime, "derived")?;
    assert_eq!(snapshot.plan_revision, 4);
    assert_eq!(snapshot.tasks.len(), 6);
    assert_eq!(snapshot.runs.len(), 2);
    assert_eq!(snapshot.assumptions[0].id, "local-assumption");
    assert_eq!(snapshot.assumptions[0].statement, "Keep original text root");
    let medium = object_tasks::get_task(&runtime, "derived", &build)?.unwrap();
    assert_eq!(medium.parent_task_id.as_deref(), Some(root.as_str()));
    assert_eq!(
        serde_json::to_value(&medium.identity)?["baseline"]["selectedVersionId"],
        mapped("object_version", &f.base.first.version_id)?
    );
    let fine =
        object_tasks::get_task(&runtime, "derived", &mapped("object_task", "shape")?)?.unwrap();
    assert_eq!(fine.stage_id.as_deref(), Some("model"));
    assert_eq!(fine.run_id, medium.run_id);
    let cancelled = object_tasks::get_task(&runtime, "derived", &discard)?.unwrap();
    assert_eq!(cancelled.depends_on, [build.clone()]);
    assert_eq!(cancelled.status, "cancelled");
    assert_eq!(
        object_tasks::get_task(&runtime, "derived", &mapped("object_task", "discard-fine")?)?
            .unwrap()
            .status,
        "cancelled"
    );
    assert!(object_tasks::queue(&runtime, "derived")?.is_empty());
    assert!(object_tasks::claim_next(&runtime, "derived", "derivation-test")?.is_none());
    let receipt = object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":mapped("object_task_commit_receipt", "commit")?,"draftId":"historical",
            "expectedDraftRevision":f.commit.draft_revision,"expectedPlanRevision":f.commit.previous_plan_revision
        }))?,
    )?;
    assert_eq!(receipt.task_ids.len(), f.commit.task_ids.len());
    assert!(receipt.runs.iter().all(|r| r.status == "planned"));
    let revision = object_tasks::revise_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":mapped("object_task_definition_revision", "revise")?,"taskId":root,
            "expectedTaskRevision":f.revision.previous_task_revision,"expectedPlanRevision":f.revision.previous_plan_revision,
            "definition":f.revision.after,"reason":f.revision.reason
        }))?,
    )?;
    assert_eq!(revision.before, f.revision.before);
    assert_eq!(
        revision.affected_task_ids.len(),
        f.revision.affected_task_ids.len()
    );
    let cancel = object_tasks::cancel_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":mapped("object_task_cancel_receipt", "cancel")?,"taskId":discard,
            "expectedTaskRevision":f.cancel.previous_task_revision,"expectedPlanRevision":f.cancel.previous_plan_revision
        }))?,
    )?;
    assert_eq!(cancel.plan_revision, f.cancel.plan_revision);
    let unlocked = object_tasks::unlock_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":mapped("object_task_draft_unlock_receipt", "unlock")?,"draftId":"historical",
            "expectedRevision":2,"expectedPlanRevision":3
        }))?,
    )?;
    assert_eq!(unlocked.plan, f.unlocked.plan);
    assert_eq!(unlocked.project_id, "derived");
    let stale = object_tasks::get_draft(&runtime, "derived", "stale")?.unwrap();
    let error = object_tasks::commit(&runtime, &serde_json::from_value(json!({
        "projectId":"derived","requestId":"stale-commit","draftId":"stale","expectedDraftRevision":stale.revision,"expectedPlanRevision":4
    }))?).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_DRAFT_PLAN_REVISION_CONFLICT"));
    let mut draft = object_tasks::get_draft(&runtime, "derived", "object-task-plan")?.unwrap();
    assert_eq!(
        draft.plan.tasks[0].id,
        generated(&prepared.request, "object_task", "future-task")?
    );
    assert_eq!(
        draft.plan.tasks[0].object_id,
        Some(draft.plan.objects[0].id.clone())
    );
    assert_eq!(draft.plan.tasks[0].depends_on, [build.clone()]);
    assert_eq!(draft.plan.tasks[0].prompt, "future original root");
    draft.plan.tasks[0].prompt.push_str(" edited in copy");
    save(
        &runtime,
        &draft.id,
        draft.revision,
        serde_json::to_value(&draft.plan)?,
    )?;
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":"new-commit","draftId":draft.id,"expectedDraftRevision":2,"expectedPlanRevision":4
        }))?,
    )?;
    let future = object_tasks::get_task(&runtime, "derived", &draft.plan.tasks[0].id)?.unwrap();
    let mut definition = object_tasks::TaskDefinition::from_task(&future);
    definition.title = "Edited after derivation".into();
    object_tasks::revise_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":"new-revision","taskId":future.id,"expectedTaskRevision":0,"expectedPlanRevision":5,
            "definition":definition,"reason":"Continue independent work"
        }))?,
    )?;
    object_tasks::unlock_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":"new-unlock","draftId":draft.id,"expectedRevision":3,"expectedPlanRevision":6
        }))?,
    )?;
    object_tasks::cancel_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"derived","requestId":"new-cancel","taskId":root,"expectedTaskRevision":1,"expectedPlanRevision":6
        }))?,
    )?;
    let current = object_tasks::snapshot(&runtime, "derived")?;
    assert_eq!(current.plan_revision, 7);
    drop(runtime);
    router.close("derived")?;
    let reopened = router.open_registered("derived")?;
    assert_eq!(object_tasks::snapshot(&reopened, "derived")?, current);
    assert_eq!(
        object_tasks::get_draft(&reopened, "derived", &draft.id)?
            .unwrap()
            .revision,
        4
    );
    assert!(object_tasks::claim_next(&reopened, "derived", "reopened-test")?.is_none());
    let next_source = reopened.project_root().to_path_buf();
    drop(reopened);
    router.close("derived")?;
    copy::prepare(
        copy::Request {
            request_id: "derive-again".into(),
            source: next_source,
            source_project_id: "derived".into(),
            target_project_id: "third".into(),
        },
        &f.base.temp.path().join("third-prepared"),
    )?;
    assert_eq!(data_backup::inventory(&f.base.source)?, before);
    assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    Ok(())
}

#[test]
fn project_derivation_plan_accepts_draft_only_and_noop_commits() -> Result<()> {
    let f = objects::Fixture::new()?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    let plan = json!({"tasks":[{
        "id":"manual","granularity":"coarse","title":"Manual","prompt":"No dispatch","acceptance":""
    }]});
    save(&runtime, "object-task-plan", 0, plan.clone())?;
    drop(runtime);
    copy::prepare(f.request(), &f.temp.path().join("draft-only"))?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"first","draftId":"object-task-plan",
            "expectedDraftRevision":1,"expectedPlanRevision":0
        }))?,
    )?;
    save(&runtime, "same-plan", 0, plan)?;
    let receipt = object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"noop","draftId":"same-plan",
            "expectedDraftRevision":1,"expectedPlanRevision":1
        }))?,
    )?;
    assert_eq!(receipt.previous_plan_revision, receipt.plan_revision);
    drop(runtime);
    copy::prepare(f.request(), &f.temp.path().join("noop-history"))?;
    Ok(())
}
