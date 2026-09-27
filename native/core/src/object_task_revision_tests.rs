use super::commit::{commit, save, task};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_tasks::{
        self, CommitRequest, Granularity, ObjectProposal, PlanProposal, SaveDraftRequest,
    },
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn duplicate_proposals_keep_ids_and_request_replays_survive_draft_edits() -> Result<()> {
    let fixture = Fixture::new()?;
    let plan = PlanProposal {
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
    };
    save(&fixture, "first", 0, plan.clone())?;
    let original = commit(&fixture, "request", "first", 0)?;
    save(&fixture, "duplicate", 1, plan.clone())?;
    let duplicate = commit(&fixture, "duplicate", "duplicate", 1)?;
    assert_eq!(duplicate.plan_revision, 1);
    assert_eq!(duplicate.task_ids, original.task_ids);
    assert_eq!(duplicate.runs, original.runs);
    assert_eq!(fixture.count("object")?, 1);
    assert_eq!(fixture.count("object_task")?, 1);
    assert_eq!(fixture.count("object_run")?, 1);

    let conflict = commit(&fixture, "request", "duplicate", 1).unwrap_err();
    assert!(format!("{conflict:#}").contains("OBJECT_TASK_REQUEST_ID_CONFLICT"));
    let mut edited = plan;
    edited.tasks[0].prompt = "Revised requirements".into();
    object_tasks::unlock_draft(
        &fixture.runtime,
        &object_tasks::UnlockDraftRequest {
            project_id: "project-1".into(),
            request_id: "next".into(),
            draft_id: "first".into(),
            expected_revision: 2,
            expected_plan_revision: 1,
        },
    )?;
    object_tasks::save_draft(
        &fixture.runtime,
        &SaveDraftRequest {
            project_id: "project-1".into(),
            draft_id: "first".into(),
            expected_revision: 3,
            expected_plan_revision: 1,
            plan: edited,
        },
    )?;
    assert_eq!(commit(&fixture, "request", "first", 0)?, original);
    let conflict = object_tasks::commit(
        &fixture.runtime,
        &CommitRequest {
            project_id: "project-1".into(),
            request_id: "edited".into(),
            draft_id: "first".into(),
            expected_draft_revision: 4,
            expected_plan_revision: 1,
        },
    )
    .unwrap_err();
    assert!(format!("{conflict:#}").contains("OBJECT_TASK_ID_CONFLICT"));
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision,
        1
    );
    Ok(())
}

#[test]
fn adding_a_fine_task_to_a_persisted_medium_returns_its_existing_run() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.accepted("hero", "v1")?;
    save(
        &fixture,
        "medium",
        0,
        PlanProposal {
            objects: vec![],
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
    let medium = commit(&fixture, "medium", "medium", 0)?;
    save(
        &fixture,
        "fine",
        1,
        PlanProposal {
            objects: vec![],
            tasks: vec![task(
                "fine",
                Granularity::Fine,
                Some("hero"),
                Some("medium"),
                Some("mesh"),
            )],
            assumptions: vec![],
        },
    )?;
    let fine = commit(&fixture, "fine", "fine", 1)?;
    assert_eq!(fine.runs, medium.runs);
    assert_eq!(fine.plan_revision, 2);
    assert_eq!(fixture.count("object_run")?, 1);
    assert_eq!(
        object_tasks::get_run(&fixture.runtime, "project-1", &fine.runs[0].id)?,
        Some(fine.runs[0].clone())
    );
    let Fixture { runtime, temp } = fixture;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let snapshot = object_tasks::snapshot(&reopened, "project-1")?;
    assert_eq!(snapshot.plan_revision, 2);
    assert_eq!(snapshot.tasks.len(), 2);
    assert_eq!(snapshot.runs, medium.runs);
    Ok(())
}

#[test]
fn stale_plan_and_draft_revisions_do_not_commit_partial_records() -> Result<()> {
    let fixture = Fixture::new()?;
    for id in ["first", "stale"] {
        save(
            &fixture,
            id,
            0,
            PlanProposal {
                objects: vec![],
                tasks: vec![task(id, Granularity::Coarse, None, None, None)],
                assumptions: vec![],
            },
        )?;
    }
    let stale_draft = object_tasks::commit(
        &fixture.runtime,
        &CommitRequest {
            project_id: "project-1".into(),
            request_id: "wrong-draft".into(),
            draft_id: "first".into(),
            expected_draft_revision: 0,
            expected_plan_revision: 0,
        },
    )
    .unwrap_err();
    assert!(format!("{stale_draft:#}").contains("OBJECT_TASK_DRAFT_REVISION_CONFLICT"));
    commit(&fixture, "first", "first", 0)?;
    let stale = commit(&fixture, "stale", "stale", 0).unwrap_err();
    assert!(format!("{stale:#}").contains("OBJECT_TASK_PLAN_REVISION_CONFLICT"));
    let stale = commit(&fixture, "stale", "stale", 1).unwrap_err();
    assert!(format!("{stale:#}").contains("OBJECT_TASK_DRAFT_PLAN_REVISION_CONFLICT"));
    assert_eq!(fixture.count("object_task")?, 1);
    assert_eq!(fixture.count("object_task_commit_receipt")?, 1);
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision,
        1
    );
    Ok(())
}
