use super::commit::{commit, save, task};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{CancelPlannedRequest, Granularity, ObjectProposal, PlanProposal},
    object_tasks,
};
use anyhow::Result;

pub(super) fn request(
    task_id: &str,
    request_id: &str,
    task_revision: u64,
    plan_revision: u64,
) -> CancelPlannedRequest {
    CancelPlannedRequest {
        project_id: "project-1".into(),
        task_id: task_id.into(),
        request_id: request_id.into(),
        expected_task_revision: task_revision,
        expected_plan_revision: plan_revision,
    }
}

pub(super) fn committed_hierarchy(fixture: &Fixture) -> Result<()> {
    save(
        fixture,
        "initial",
        0,
        PlanProposal {
            objects: vec![ObjectProposal {
                id: "hero".into(),
                name: "Hero".into(),
                category: "character".into(),
            }],
            tasks: vec![
                task("medium", Granularity::Medium, Some("hero"), None, None),
                task(
                    "fine",
                    Granularity::Fine,
                    Some("hero"),
                    Some("medium"),
                    Some("mesh"),
                ),
            ],
            assumptions: vec![],
        },
    )?;
    commit(fixture, "initial-commit", "initial", 0)?;
    Ok(())
}

#[test]
fn cancellation_preserves_history_and_replays_the_original_receipt() -> Result<()> {
    let fixture = Fixture::new()?;
    committed_hierarchy(&fixture)?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let cancel_request = request("medium", "withdraw-medium", 0, before.plan_revision);
    let receipt = object_tasks::cancel_planned(&fixture.runtime, &cancel_request)?;
    assert_eq!(receipt.previous_task_revision, 0);
    assert_eq!(receipt.task_revision, 1);
    assert_eq!(receipt.previous_plan_revision, 1);
    assert_eq!(receipt.plan_revision, 2);
    assert_eq!(
        object_tasks::cancel_planned(&fixture.runtime, &cancel_request)?,
        receipt
    );

    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(snapshot.plan_revision, 2);
    assert_eq!(snapshot.tasks.len(), 2);
    let medium = object_tasks::get_task(&fixture.runtime, "project-1", "medium")?.unwrap();
    let fine = object_tasks::get_task(&fixture.runtime, "project-1", "fine")?.unwrap();
    assert_eq!((medium.status.as_str(), medium.revision), ("cancelled", 1));
    assert_eq!((fine.status.as_str(), fine.revision), ("cancelled", 1));
    assert_eq!(fine.run_id, medium.run_id);
    assert_eq!(snapshot.runs[0].id, before.runs[0].id);
    assert_eq!(
        (snapshot.runs[0].status.as_str(), snapshot.runs[0].revision),
        ("cancelled", 1)
    );
    assert_eq!(fixture.count("object_task")?, 2);
    assert_eq!(fixture.count("object_run")?, 1);
    assert_eq!(fixture.count("object_task_cancel_receipt")?, 1);

    let second_request = request("medium", "withdraw-again", 1, 2);
    let error = object_tasks::cancel_planned(&fixture.runtime, &second_request).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_NOT_PLANNED"));
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision,
        2
    );
    Ok(())
}

#[test]
fn stale_revisions_and_reused_request_ids_are_rejected_without_extra_changes() -> Result<()> {
    let fixture = Fixture::new()?;
    save(
        &fixture,
        "initial",
        0,
        PlanProposal {
            objects: vec![],
            tasks: vec![task("coarse", Granularity::Coarse, None, None, None)],
            assumptions: vec![],
        },
    )?;
    commit(&fixture, "initial-commit", "initial", 0)?;

    let stale_plan = request("coarse", "stale-plan", 0, 0);
    let error = object_tasks::cancel_planned(&fixture.runtime, &stale_plan).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_PLAN_REVISION_CONFLICT"));
    let stale_task = request("coarse", "stale-task", 1, 1);
    let error = object_tasks::cancel_planned(&fixture.runtime, &stale_task).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_REVISION_CONFLICT"));
    assert_eq!(
        object_tasks::get_task(&fixture.runtime, "project-1", "coarse")?
            .unwrap()
            .status,
        "planned"
    );
    assert_eq!(fixture.count("object_task_cancel_receipt")?, 0);

    let original = request("coarse", "stable", 0, 1);
    object_tasks::cancel_planned(&fixture.runtime, &original)?;
    let conflict = request("coarse", "stable", 0, 0);
    let error = object_tasks::cancel_planned(&fixture.runtime, &conflict).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_REQUEST_ID_CONFLICT"));
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision,
        2
    );
    assert_eq!(fixture.count("object_task_cancel_receipt")?, 1);
    Ok(())
}

#[test]
fn receipt_write_failure_rolls_back_owned_tasks_runs_and_plan_revisions() -> Result<()> {
    let fixture = Fixture::new()?;
    committed_hierarchy(&fixture)?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let handle = fixture.runtime.store();
    handle.lock().unwrap().connection.execute_batch(
        "CREATE TRIGGER fail_object_task_cancel BEFORE INSERT ON entities
         WHEN NEW.kind = 'object_task_cancel_receipt'
         BEGIN SELECT RAISE(ABORT, 'forced cancellation failure'); END;",
    )?;

    let request = request("medium", "retry", 0, 1);
    let error = object_tasks::cancel_planned(&fixture.runtime, &request).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_CANCEL_FAILED"));
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        before
    );
    assert_eq!(fixture.count("object_task_cancel_receipt")?, 0);

    handle
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_object_task_cancel;")?;
    let receipt = object_tasks::cancel_planned(&fixture.runtime, &request)?;
    assert_eq!(receipt.plan_revision, 2);
    assert_eq!(fixture.count("object_task_cancel_receipt")?, 1);
    Ok(())
}
