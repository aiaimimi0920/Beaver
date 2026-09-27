use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_storage::{DRAFT_KIND, STATE_KIND},
    object_task_types::{Draft, PlanProposal, SaveDraftRequest},
    object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::json;

fn save_request(expected_revision: u64, expected_plan_revision: u64) -> SaveDraftRequest {
    SaveDraftRequest {
        project_id: "project-1".into(),
        draft_id: "draft-1".into(),
        expected_revision,
        expected_plan_revision,
        plan: PlanProposal::default(),
    }
}

#[test]
fn empty_draft_revisions_conflict_and_survive_project_reopen() -> Result<()> {
    let fixture = Fixture::new()?;
    let draft = object_tasks::save_draft(&fixture.runtime, &save_request(0, 0))?;
    assert_eq!(draft.revision, 1);
    assert_eq!(
        object_tasks::get_draft(&fixture.runtime, "project-1", "draft-1")?,
        Some(draft.clone())
    );

    let stale_draft = object_tasks::save_draft(&fixture.runtime, &save_request(0, 0)).unwrap_err();
    assert!(format!("{stale_draft:#}").contains("OBJECT_TASK_DRAFT_REVISION_CONFLICT"));
    let stale_plan = object_tasks::save_draft(&fixture.runtime, &save_request(1, 1)).unwrap_err();
    assert!(format!("{stale_plan:#}").contains("OBJECT_TASK_PLAN_REVISION_CONFLICT"));

    let crate::object_catalog_test_fixture::Fixture { runtime, temp } = fixture;
    drop(runtime);
    let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        object_tasks::get_draft(&reopened, "project-1", "draft-1")?,
        Some(draft)
    );
    Ok(())
}

#[test]
fn draft_lookup_rejects_payload_id_that_does_not_match_storage_key() -> Result<()> {
    let fixture = Fixture::new()?;
    let malformed = Draft {
        project_id: "project-1".into(),
        id: "other-id".into(),
        revision: 1,
        plan_revision: 0,
        plan: PlanProposal::default(),
        committed_request_id: None,
    };
    fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .put(DRAFT_KIND, "draft-1", &malformed)?;
    let error = object_tasks::get_draft(&fixture.runtime, "project-1", "draft-1").unwrap_err();
    assert!(error
        .to_string()
        .contains("OBJECT_TASK_DRAFT_IDENTITY_MISMATCH"));
    Ok(())
}

#[test]
fn snapshots_read_plan_state_written_before_assumption_history_existed() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.runtime.store().lock().unwrap().put(
        STATE_KIND,
        "project-1",
        &json!({"projectId":"project-1","revision":4}),
    )?;

    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(snapshot.plan_revision, 4);
    assert!(snapshot.assumptions.is_empty());
    Ok(())
}
