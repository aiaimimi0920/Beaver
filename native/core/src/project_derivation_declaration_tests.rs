use super::*;
use crate::object_task_planning_declaration::{self as declaration, Declaration};

#[path = "project_derivation_declaration_rejection_tests.rs"]
mod rejection;

fn state<'a>(snapshot: &'a object_tasks::Snapshot, task: &str) -> &'a declaration::State {
    snapshot
        .planning_states
        .iter()
        .find(|s| s.task_id == task)
        .unwrap()
}

fn receipt(runtime: &crate::project_runtime::ProjectRuntime, id: &str) -> Result<Declaration> {
    Ok(runtime
        .store()
        .lock()
        .unwrap()
        .get::<Declaration>("object_task_planning_declaration_receipt", id)?
        .unwrap())
}

#[test]
fn project_derivation_declaration_preserves_current_stale_cancelled_and_replay_after_reopen(
) -> Result<()> {
    let f = Fixture::with_declarations()?;
    let source = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let original = object_tasks::snapshot(&source, "original")?;
    let originals = [
        "early-root",
        "early-root-again",
        "revised-root",
        "build-confirmed",
        "discard-confirmed",
    ]
    .into_iter()
    .map(|id| receipt(&source, id))
    .collect::<Result<Vec<_>>>()?;
    assert!(!state(&original, "root").current);
    assert!(state(&original, "build").current);
    assert!(!state(&original, "discard").current);
    drop(source);
    let before = data_backup::inventory(&f.base.source)?;
    let preparation = f.base.temp.path().join("declarations");
    let prepared = copy::prepare(f.base.request(), &preparation)?;
    let mapped = |kind: &str, id: &str| -> Result<String> {
        Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
    };
    let root = mapped("object_task", "root")?;
    let build = mapped("object_task", "build")?;
    let discard = mapped("object_task", "discard")?;
    let prepared_before = data_backup::inventory(&preparation)?;
    let target = f.base.temp.path().join("declaration-target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let data = f.base.temp.path().join("declaration-host");
    let router = ProjectStorageRouter::new(Arc::new(Mutex::new(Store::open(&data)?)));
    router.register_assembly(&preparation, &target, &data)?;
    let runtime = router.open_registered("derived")?;
    let snapshot = object_tasks::snapshot(&runtime, "derived")?;
    for source_state in &original.planning_states {
        let id = mapped("object_task", &source_state.task_id)?;
        let derived_state = state(&snapshot, &id);
        assert_eq!(derived_state.current, source_state.current);
        assert_ne!(derived_state.scope_hash, source_state.scope_hash);
    }
    assert!(state(&snapshot, &discard)
        .blockers
        .iter()
        .any(|s| s.contains("已撤销")));
    assert_eq!(snapshot.plan_revision, original.plan_revision);
    for original in &originals {
        let derived = receipt(
            &runtime,
            &mapped(
                "object_task_planning_declaration_receipt",
                &original.request.request_id,
            )?,
        )?;
        assert_eq!(derived.request.project_id, "derived");
        assert_eq!(
            derived.request.task_id,
            mapped("object_task", &original.request.task_id)?
        );
        assert_eq!(
            derived.request.expected_plan_revision,
            original.request.expected_plan_revision
        );
        assert_eq!(derived.request.reason, original.request.reason);
        assert_eq!(derived.created_at, original.created_at);
        assert_eq!(derived.declared_by, original.declared_by);
        assert_ne!(
            derived.request.expected_scope_hash,
            original.request.expected_scope_hash
        );
        let mut ids = original
            .task_ids
            .iter()
            .map(|id| mapped("object_task", id))
            .collect::<Result<Vec<_>>>()?;
        ids.sort();
        assert_eq!(derived.task_ids, ids);
        assert_eq!(declaration::declare(&runtime, &derived.request)?, derived);
    }
    // Replaying older receipts cannot replace the latest head, accept tasks, or revise the plan.
    assert_eq!(object_tasks::snapshot(&runtime, "derived")?, snapshot);
    let mut conflict = receipt(
        &runtime,
        &mapped("object_task_planning_declaration_receipt", "early-root")?,
    )?
    .request;
    conflict.reason.push_str(" changed");
    assert!(declaration::declare(&runtime, &conflict)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    assert!(object_tasks::queue(&runtime, "derived")?.is_empty());
    assert!(object_tasks::claim_next(&runtime, "derived", "declaration-test")?.is_none());
    drop(runtime);
    router.close("derived")?;
    let runtime = router.open_registered("derived")?;
    assert_eq!(object_tasks::snapshot(&runtime, "derived")?, snapshot);
    let confirmed = fixture::declare(&runtime, &root, "confirm-copy")?;
    let current = object_tasks::snapshot(&runtime, "derived")?;
    assert!(state(&current, &root).current);
    assert!(state(&current, &build).current);
    assert!(!state(&current, &discard).current);
    assert_eq!(confirmed.task_ids.len(), 5); // Includes the cancelled descendant subtree.
    assert_eq!(current.tasks, snapshot.tasks);
    assert_eq!(current.runs, snapshot.runs);
    assert_eq!(current.plan_revision, snapshot.plan_revision);
    assert_eq!(
        declaration::declare(&runtime, &confirmed.request)?,
        confirmed
    );
    let next_source = runtime.project_root().to_path_buf();
    drop(runtime);
    router.close("derived")?;
    copy::prepare(
        copy::Request {
            request_id: "derive-declarations-again".into(),
            source: next_source,
            source_project_id: "derived".into(),
            target_project_id: "third".into(),
        },
        &f.base.temp.path().join("third-declarations"),
    )?;
    assert_eq!(data_backup::inventory(&f.base.source)?, before);
    assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
    Ok(())
}

#[test]
fn project_derivation_declaration_excludes_later_members_and_preserves_restored_scope() -> Result<()>
{
    let f = Fixture::with_declarations()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let shape = object_tasks::get_task(&runtime, "original", "shape")?.unwrap();
    let original_definition = object_tasks::TaskDefinition::from_task(&shape);
    let mut changed = original_definition.clone();
    changed.prompt.push_str(" revised");
    for (definition, request, revision) in [
        (changed, "change-shape", 0),
        (original_definition, "restore-shape", 1),
    ] {
        let plan_revision = object_tasks::snapshot(&runtime, "original")?.plan_revision;
        object_tasks::revise_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":request,"taskId":"shape","expectedTaskRevision":revision,
                "expectedPlanRevision":plan_revision,"definition":definition,"reason":"Explicit edit"
            }))?,
        )?;
    }
    assert!(state(&object_tasks::snapshot(&runtime, "original")?, "build").current);
    drop(runtime);
    // Equal scope is current even though the global plan revision and execution-independent task revision increased.
    let restored = copy::prepare(f.base.request(), &f.base.temp.path().join("restored"))?;
    let database = crate::project_derivation_database::compose(
        &ProjectStore::open(&f.base.source, "original")?
            .store()
            .connection,
        &restored.request,
        &restored.identities,
    )?;
    let tasks = crate::object_task_storage::all_tasks(&database.connection)?;
    let states = declaration::snapshot_in(&database.connection, &tasks)?;
    let build = generated(&restored.request, "object_task", "build")?;
    assert!(states.iter().find(|s| s.task_id == build).unwrap().current);
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    save(
        &runtime,
        "later",
        0,
        json!({"tasks":[{
            "id":"later-child","granularity":"fine","title":"Later","prompt":"New member","acceptance":"",
            "objectId":f.base.parent.id,"parentTaskId":"build","stageId":"audio"
        }]}),
    )?;
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"later-commit","draftId":"later",
            "expectedDraftRevision":1,"expectedPlanRevision":6
        }))?,
    )?;
    fixture::declare(&runtime, "root", "expanded-root")?;
    assert!(!state(&object_tasks::snapshot(&runtime, "original")?, "build").current);
    drop(runtime);
    let prepared = copy::prepare(f.base.request(), &f.base.temp.path().join("expanded"))?;
    let staged = crate::project_derivation_database::compose(
        &ProjectStore::open(&f.base.source, "original")?
            .store()
            .connection,
        &prepared.request,
        &prepared.identities,
    )?;
    let tasks = crate::object_task_storage::all_tasks(&staged.connection)?;
    let states = declaration::snapshot_in(&staged.connection, &tasks)?;
    let root = generated(&prepared.request, "object_task", "root")?;
    assert!(states.iter().find(|s| s.task_id == root).unwrap().current);
    assert!(!states.iter().find(|s| s.task_id == build).unwrap().current);
    let old: Declaration = crate::object_task_storage::read(
        &staged.connection,
        "object_task_planning_declaration_receipt",
        &generated(&prepared.request, "object_task_request", "early-root")?,
    )?
    .unwrap();
    assert_eq!(old.task_ids.len(), 5);
    assert!(!old
        .task_ids
        .contains(&generated(&prepared.request, "object_task", "later-child")?));
    Ok(())
}
