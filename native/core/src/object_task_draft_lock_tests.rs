use super::commit::{commit, save, task};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_planning_store,
    object_task_planning_types::StartRequest,
    object_tasks::{self, Granularity, PlanProposal, SaveDraftRequest, UnlockDraftRequest},
    project_storage::ProjectStore,
};
use anyhow::Result;

fn unlock_request() -> UnlockDraftRequest {
    UnlockDraftRequest {
        project_id: "project-1".into(),
        request_id: "next".into(),
        draft_id: "draft".into(),
        expected_revision: 2,
        expected_plan_revision: 1,
    }
}

#[test]
fn committed_draft_survives_reopen_and_unlock_replay_never_overwrites_later_edits() -> Result<()> {
    let fixture = Fixture::new()?;
    let plan = PlanProposal {
        tasks: vec![task("coarse", Granularity::Coarse, None, None, None)],
        ..Default::default()
    };
    save(&fixture, "draft", 0, plan.clone())?;
    let receipt = commit(&fixture, "commit", "draft", 0)?;
    assert_eq!(commit(&fixture, "commit", "draft", 0)?, receipt);
    let locked = object_tasks::get_draft(&fixture.runtime, "project-1", "draft")?.unwrap();
    assert_eq!(locked.revision, 2);
    assert_eq!(locked.committed_request_id.as_deref(), Some("commit"));
    assert_eq!(locked.plan, plan);
    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let save_request = SaveDraftRequest {
        project_id: "project-1".into(),
        draft_id: "draft".into(),
        expected_revision: 2,
        expected_plan_revision: 1,
        plan: PlanProposal::default(),
    };
    let error = object_tasks::save_draft(&fixture.runtime, &save_request).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_DRAFT_LOCKED"));
    let input = StartRequest {
        project_id: "project-1".into(),
        draft_id: "draft".into(),
        request_id: "planning".into(),
        expected_draft_revision: 2,
        expected_plan_revision: 1,
        goal: "New work".into(),
        acceptance: "Ready".into(),
        ask_ratio: 30,
    };
    let error = object_task_planning_store::transact(&fixture.runtime, "project-1", |db| {
        object_task_planning_store::scope(db, &input)
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_DRAFT_LOCKED"));
    let Fixture { runtime, temp } = fixture;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        object_tasks::get_draft(&reopened, "project-1", "draft")?,
        Some(locked)
    );
    let request = unlock_request();
    let mut stale = request.clone();
    stale.expected_plan_revision = 0;
    assert!(object_tasks::unlock_draft(&reopened, &stale).is_err());
    let next = object_tasks::unlock_draft(&reopened, &request)?;
    assert_eq!(next.revision, 3);
    assert_eq!(next.plan_revision, 1);
    assert_eq!(next.plan, PlanProposal::default());
    assert_eq!(next.committed_request_id, None);
    let edited = object_tasks::save_draft(
        &reopened,
        &SaveDraftRequest {
            expected_revision: 3,
            plan: PlanProposal {
                tasks: vec![task("next-coarse", Granularity::Coarse, None, None, None)],
                ..Default::default()
            },
            ..save_request
        },
    )?;
    drop(reopened);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(object_tasks::unlock_draft(&reopened, &request)?, next);
    assert_eq!(
        object_tasks::get_draft(&reopened, "project-1", "draft")?,
        Some(edited)
    );
    assert_eq!(object_tasks::snapshot(&reopened, "project-1")?, snapshot);
    let mut reused = request;
    reused.expected_revision = 4;
    let error = object_tasks::unlock_draft(&reopened, &reused).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_REQUEST_ID_CONFLICT"));
    Ok(())
}

#[test]
fn unlock_requires_committed_draft_and_commit_failure_does_not_lock() -> Result<()> {
    let fixture = Fixture::new()?;
    save(&fixture, "draft", 0, PlanProposal::default())?;
    assert!(commit(&fixture, "empty", "draft", 0).is_err());
    let draft = object_tasks::get_draft(&fixture.runtime, "project-1", "draft")?.unwrap();
    assert_eq!(draft.revision, 1);
    assert_eq!(draft.committed_request_id, None);
    let request = UnlockDraftRequest {
        expected_revision: 1,
        expected_plan_revision: 0,
        ..unlock_request()
    };
    let error = object_tasks::unlock_draft(&fixture.runtime, &request).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_DRAFT_NOT_LOCKED"));
    assert_eq!(fixture.count("object_task_draft_unlock_receipt")?, 0);
    assert_eq!(fixture.count("object_task_commit_receipt")?, 0);
    Ok(())
}
