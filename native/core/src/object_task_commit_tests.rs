use crate::{
    object_catalog_test_fixture::Fixture,
    object_framework::{Baseline, Identity, VERSION},
    object_task_types::{
        AssumptionSource, CommitRequest, Granularity, ObjectProposal, PlanAssumption,
        PlanAssumptionRecord, PlanProposal, SaveDraftRequest, TaskProposal,
    },
    object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;

pub(super) fn task(
    id: &str,
    granularity: Granularity,
    object_id: Option<&str>,
    parent_task_id: Option<&str>,
    stage_id: Option<&str>,
) -> TaskProposal {
    TaskProposal {
        id: id.into(),
        position: 0,
        granularity,
        title: format!("Task {id}"),
        prompt: format!("Work on {id}"),
        acceptance: "Complete the requested work".into(),
        requirement: Default::default(),
        pending_planning: String::new(),
        object_id: object_id.map(str::to_owned),
        parent_task_id: parent_task_id.map(str::to_owned),
        depends_on: vec![],
        stage_id: stage_id.map(str::to_owned),
        baseline: None,
    }
}

pub(super) fn save(
    fixture: &Fixture,
    draft_id: &str,
    plan_revision: u64,
    plan: PlanProposal,
) -> Result<()> {
    object_tasks::save_draft(
        &fixture.runtime,
        &SaveDraftRequest {
            project_id: "project-1".into(),
            draft_id: draft_id.into(),
            expected_revision: 0,
            expected_plan_revision: plan_revision,
            plan,
        },
    )?;
    Ok(())
}

pub(super) fn commit(
    fixture: &Fixture,
    request_id: &str,
    draft_id: &str,
    plan_revision: u64,
) -> Result<crate::object_task_types::CommitReceipt> {
    object_tasks::commit(
        &fixture.runtime,
        &CommitRequest {
            project_id: "project-1".into(),
            request_id: request_id.into(),
            draft_id: draft_id.into(),
            expected_draft_revision: 1,
            expected_plan_revision: plan_revision,
        },
    )
}

#[test]
fn commits_and_replays_hierarchy_then_adds_medium_directly_to_object() -> Result<()> {
    let fixture = Fixture::new()?;
    let plan = PlanProposal {
        objects: vec![ObjectProposal {
            id: "hero".into(),
            name: "Hero".into(),
            category: "character".into(),
        }],
        tasks: vec![
            task("coarse", Granularity::Coarse, None, None, None),
            task(
                "medium",
                Granularity::Medium,
                Some("hero"),
                Some("coarse"),
                None,
            ),
            task(
                "fine",
                Granularity::Fine,
                Some("hero"),
                Some("medium"),
                Some("mesh"),
            ),
        ],
        assumptions: vec![],
    };
    save(&fixture, "draft-1", 0, plan)?;
    let first = commit(&fixture, "request-1", "draft-1", 0)?;
    assert_eq!(first.task_ids, ["coarse", "medium", "fine"]);
    assert_eq!(first.object_ids, ["hero"]);
    assert_eq!(first.plan_revision, 1);
    assert_eq!(first.runs.len(), 1);
    assert_eq!(commit(&fixture, "request-1", "draft-1", 0)?, first);

    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(snapshot.plan_revision, 1);
    assert_eq!(snapshot.tasks.len(), 3);
    assert_eq!(snapshot.runs, first.runs);
    let medium = object_tasks::get_task(&fixture.runtime, "project-1", "medium")?.unwrap();
    let fine = object_tasks::get_task(&fixture.runtime, "project-1", "fine")?.unwrap();
    assert_eq!(medium.run_id.as_deref(), Some(first.runs[0].id.as_str()));
    assert_eq!(
        medium.identity,
        Identity::Medium {
            schema_version: VERSION,
            object_id: "hero".into(),
            baseline: Baseline::LatestAccepted {},
        }
    );
    assert_eq!(fine.parent_task_id.as_deref(), Some("medium"));
    assert_eq!(fine.run_id, medium.run_id);
    assert_eq!(
        fine.identity,
        Identity::Fine {
            schema_version: VERSION,
            object_id: "hero".into(),
            medium_task_id: "medium".into(),
            run_id: first.runs[0].id.clone(),
            stage_id: "mesh".into(),
        }
    );
    assert_eq!(fixture.count("task")?, 0);
    assert_eq!(fixture.count("object")?, 1);
    assert_eq!(fixture.count("object_task")?, 3);
    assert_eq!(fixture.count("object_run")?, 1);

    save(
        &fixture,
        "draft-2",
        1,
        PlanProposal {
            objects: vec![],
            tasks: vec![task(
                "direct-medium",
                Granularity::Medium,
                Some("hero"),
                None,
                None,
            )],
            assumptions: vec![],
        },
    )?;
    let next = commit(&fixture, "request-2", "draft-2", 1)?;
    let direct = object_tasks::get_task(&fixture.runtime, "project-1", "direct-medium")?.unwrap();
    assert_eq!(direct.parent_task_id, None);
    assert_eq!(direct.object_id.as_deref(), Some("hero"));
    assert_eq!(next.plan_revision, 2);
    assert_eq!(fixture.count("object_task")?, 4);
    assert_eq!(fixture.count("object_run")?, 2);
    Ok(())
}

#[test]
fn snapshot_orders_tasks_by_explicit_position_then_id() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut first = task("z-task", Granularity::Coarse, None, None, None);
    first.position = 20;
    let mut second = task("a-task", Granularity::Coarse, None, None, None);
    second.position = 10;
    save(
        &fixture,
        "draft-position",
        0,
        PlanProposal {
            objects: vec![],
            tasks: vec![first, second],
            assumptions: vec![],
        },
    )?;
    commit(&fixture, "request-position", "draft-position", 0)?;

    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(
        snapshot
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        ["a-task", "z-task"]
    );
    Ok(())
}

#[test]
fn commit_rolls_back_object_run_task_revision_and_receipt_on_write_failure() -> Result<()> {
    let fixture = Fixture::new()?;
    save(
        &fixture,
        "draft-rollback",
        0,
        PlanProposal {
            objects: vec![ObjectProposal {
                id: "hero".into(),
                name: "Hero".into(),
                category: "character".into(),
            }],
            tasks: vec![task(
                "medium",
                Granularity::Medium,
                Some("hero"),
                None,
                None,
            )],
            assumptions: vec![],
        },
    )?;
    let handle = fixture.runtime.store();
    handle.lock().unwrap().connection.execute_batch(
        "CREATE TRIGGER fail_object_task BEFORE INSERT ON entities
         WHEN NEW.kind = 'object_task_commit_receipt'
         BEGIN SELECT RAISE(ABORT, 'forced object-task write failure'); END;",
    )?;

    let error = commit(&fixture, "request-fail", "draft-rollback", 0).unwrap_err();
    assert!(error.to_string().contains("OBJECT_TASK_COMMIT_FAILED"));
    assert_eq!(fixture.count("object")?, 0);
    assert_eq!(fixture.count("object_run")?, 0);
    assert_eq!(fixture.count("object_task")?, 0);
    assert_eq!(fixture.count("object_task_commit_receipt")?, 0);
    assert_eq!(fixture.count("object_task_plan_state")?, 0);
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision,
        0
    );
    handle
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_object_task;")?;
    let receipt = commit(&fixture, "request-fail", "draft-rollback", 0)?;
    assert_eq!(receipt.plan_revision, 1);
    assert_eq!(fixture.count("object_run")?, 1);
    assert_eq!(fixture.count("object_task")?, 1);
    Ok(())
}

#[test]
fn assumptions_are_deduplicated_versioned_and_survive_project_reopen() -> Result<()> {
    let fixture = Fixture::new()?;
    let original = PlanAssumption {
        id: "lighting".into(),
        statement: "Use warm lighting indoors.".into(),
        basis: "The brief requests a welcoming interior.".into(),
        source: AssumptionSource::Automatic,
        source_detail: Some("Plan synthesis".into()),
    };
    let proposal = |assumption: PlanAssumption| PlanProposal {
        objects: vec![],
        tasks: vec![task("task", Granularity::Coarse, None, None, None)],
        assumptions: vec![assumption],
    };
    save(&fixture, "assumptions-1", 0, proposal(original.clone()))?;
    assert_eq!(
        commit(&fixture, "assumptions-commit-1", "assumptions-1", 0)?.plan_revision,
        1
    );
    let first = PlanAssumptionRecord {
        id: original.id.clone(),
        statement: original.statement.clone(),
        basis: original.basis.clone(),
        source: original.source.clone(),
        source_detail: original.source_detail.clone(),
        plan_revision: 1,
    };
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.assumptions,
        [first.clone()]
    );

    save(
        &fixture,
        "assumptions-duplicate",
        1,
        proposal(original.clone()),
    )?;
    assert_eq!(
        commit(
            &fixture,
            "assumptions-commit-duplicate",
            "assumptions-duplicate",
            1
        )?
        .plan_revision,
        1
    );
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.assumptions,
        [first.clone()]
    );

    let revised = PlanAssumption {
        statement: "Use neutral lighting indoors.".into(),
        basis: "The owner clarified that materials should read accurately.".into(),
        ..original
    };
    save(&fixture, "assumptions-2", 1, proposal(revised.clone()))?;
    assert_eq!(
        commit(&fixture, "assumptions-commit-2", "assumptions-2", 1)?.plan_revision,
        2
    );
    let second = PlanAssumptionRecord {
        id: revised.id,
        statement: revised.statement,
        basis: revised.basis,
        source: revised.source,
        source_detail: revised.source_detail,
        plan_revision: 2,
    };
    let expected = vec![first, second];
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.assumptions,
        expected
    );

    let Fixture { runtime, temp } = fixture;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        object_tasks::snapshot(&reopened, "project-1")?.assumptions,
        expected
    );
    Ok(())
}
