use super::{
    cancel::{committed_hierarchy, request},
    commit::{commit, save, task},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{
        AssumptionSource, Granularity, ObjectProposal, PlanAssumption, PlanProposal,
    },
    object_tasks,
};
use anyhow::Result;

#[test]
fn cancellation_follows_ownership_without_cancelling_shared_objects_or_dependencies() -> Result<()>
{
    let fixture = Fixture::new()?;
    let mut fine = task(
        "fine",
        Granularity::Fine,
        Some("hero"),
        Some("medium"),
        Some("mesh"),
    );
    fine.depends_on.push("independent".into());
    let mut dependent = task("dependent", Granularity::Coarse, None, None, None);
    dependent.depends_on.push("fine".into());
    save(
        &fixture,
        "hierarchy",
        0,
        PlanProposal {
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
                fine,
                task("independent", Granularity::Medium, Some("hero"), None, None),
                dependent,
            ],
            assumptions: vec![PlanAssumption {
                id: "camera".into(),
                statement: "Fixed camera".into(),
                basis: "User request".into(),
                source: AssumptionSource::User,
                source_detail: None,
            }],
        },
    )?;
    let original = commit(&fixture, "initial-commit", "hierarchy", 0)?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    object_tasks::cancel_planned(&fixture.runtime, &request("coarse", "cancel-coarse", 0, 1))?;
    let after = object_tasks::snapshot(&fixture.runtime, "project-1")?;

    assert_eq!(after.plan_revision, 2);
    assert_eq!(after.tasks.len(), before.tasks.len());
    assert_eq!(after.assumptions, before.assumptions);
    for id in ["coarse", "medium", "fine"] {
        let record = after.tasks.iter().find(|task| task.id == id).unwrap();
        assert_eq!((record.status.as_str(), record.revision), ("cancelled", 1));
    }
    for id in ["independent", "dependent"] {
        assert_eq!(
            after.tasks.iter().find(|task| task.id == id),
            before.tasks.iter().find(|task| task.id == id),
        );
    }
    assert_eq!(
        after
            .tasks
            .iter()
            .find(|task| task.id == "fine")
            .unwrap()
            .depends_on,
        ["independent"]
    );
    for run in &after.runs {
        if run.medium_task_id == "medium" {
            assert_eq!((run.status.as_str(), run.revision), ("cancelled", 1));
        } else {
            assert_eq!(
                Some(run),
                before.runs.iter().find(|previous| previous.id == run.id)
            );
        }
    }
    assert_eq!(
        commit(&fixture, "initial-commit", "hierarchy", 0)?,
        original
    );
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        after
    );
    Ok(())
}

#[test]
fn fine_cancellation_keeps_its_owner_open_and_later_owner_cancellation_keeps_child_revision(
) -> Result<()> {
    let fixture = Fixture::new()?;
    committed_hierarchy(&fixture)?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let fine_request = request("fine", "cancel-fine", 0, 1);
    let fine_receipt = object_tasks::cancel_planned(&fixture.runtime, &fine_request)?;
    let fine_cancelled = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(fine_cancelled.runs, before.runs);
    assert_eq!(
        fine_cancelled.tasks.iter().find(|task| task.id == "medium"),
        before.tasks.iter().find(|task| task.id == "medium"),
    );

    object_tasks::cancel_planned(&fixture.runtime, &request("medium", "cancel-medium", 0, 2))?;
    let after = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(after.plan_revision, 3);
    assert_eq!(after.runs[0].status, "cancelled");
    assert_eq!(after.runs[0].revision, 1);
    assert_eq!(
        after.tasks.iter().find(|task| task.id == "fine"),
        fine_cancelled.tasks.iter().find(|task| task.id == "fine"),
    );
    assert_eq!(
        object_tasks::cancel_planned(&fixture.runtime, &fine_request)?,
        fine_receipt
    );
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        after
    );
    Ok(())
}

#[test]
fn started_owned_tasks_or_runs_reject_the_entire_cancellation() -> Result<()> {
    for (kind, expected_error) in [
        ("object_task", "OBJECT_TASK_NOT_PLANNED"),
        ("object_run", "OBJECT_TASK_RUN_NOT_PLANNED"),
    ] {
        let fixture = Fixture::new()?;
        committed_hierarchy(&fixture)?;
        let run = object_tasks::get_task(&fixture.runtime, "project-1", "medium")?
            .unwrap()
            .run_id
            .unwrap();
        let id = if kind == "object_task" { "fine" } else { &run };
        fixture.runtime.store().lock().unwrap().connection.execute(
            "UPDATE entities SET value=json_set(value,'$.status','running') WHERE kind=? AND id=?",
            [kind, id],
        )?;
        let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
        let error =
            object_tasks::cancel_planned(&fixture.runtime, &request("medium", "cancel", 0, 1))
                .unwrap_err();
        assert!(format!("{error:#}").contains(expected_error));
        assert_eq!(
            object_tasks::snapshot(&fixture.runtime, "project-1")?,
            before
        );
        assert_eq!(fixture.count("object_task_cancel_receipt")?, 0);
    }
    Ok(())
}

#[test]
fn exhausted_owned_revisions_reject_the_entire_cancellation() -> Result<()> {
    for kind in ["object_task", "object_run"] {
        let fixture = Fixture::new()?;
        committed_hierarchy(&fixture)?;
        let run = object_tasks::get_task(&fixture.runtime, "project-1", "medium")?
            .unwrap()
            .run_id
            .unwrap();
        let id = if kind == "object_task" { "fine" } else { &run };
        fixture.runtime.store().lock().unwrap().connection.execute(
            "UPDATE entities SET value=json_set(value,'$.revision',9223372036854775807) WHERE kind=? AND id=?",
            [kind, id],
        )?;
        let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
        let error =
            object_tasks::cancel_planned(&fixture.runtime, &request("medium", "cancel", 0, 1))
                .unwrap_err();
        assert!(format!("{error:#}").contains("OBJECT_TASK_REVISION_EXHAUSTED"));
        assert_eq!(
            object_tasks::snapshot(&fixture.runtime, "project-1")?,
            before
        );
        assert_eq!(fixture.count("object_task_cancel_receipt")?, 0);
    }
    Ok(())
}
