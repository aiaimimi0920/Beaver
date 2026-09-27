use super::{
    cancel::{committed_hierarchy, request},
    commit::{commit, save, task},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{Granularity, ObjectProposal, PlanProposal},
    object_tasks,
};
use anyhow::Result;

#[test]
fn cancelled_references_cannot_be_hidden_by_reproposing_the_committed_task() -> Result<()> {
    let fixture = Fixture::new()?;
    committed_hierarchy(&fixture)?;
    object_tasks::cancel_planned(&fixture.runtime, &request("medium", "cancel", 0, 1))?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    for include_parent in [false, true] {
        let mut parent_plan = PlanProposal::default();
        if include_parent {
            parent_plan.tasks.push(task(
                "medium",
                Granularity::Medium,
                Some("hero"),
                None,
                None,
            ));
        }
        let mut child_plan = parent_plan.clone();
        child_plan.tasks.push(task(
            "new-fine",
            Granularity::Fine,
            Some("hero"),
            Some("medium"),
            Some("rig"),
        ));
        let error = save(&fixture, "new-child", 2, child_plan).unwrap_err();
        assert!(format!("{error:#}").contains("OBJECT_TASK_PARENT_CANCELLED"));

        let mut dependent = task("new-coarse", Granularity::Coarse, None, None, None);
        dependent.depends_on.push("medium".into());
        parent_plan.tasks.push(dependent);
        let error = save(&fixture, "new-dependent", 2, parent_plan).unwrap_err();
        assert!(format!("{error:#}").contains("OBJECT_TASK_DEPENDENCY_CANCELLED"));
    }
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        before
    );
    assert_eq!(fixture.count("object_task_draft")?, 1);
    Ok(())
}

#[test]
fn new_medium_cannot_join_a_cancelled_coarse_task() -> Result<()> {
    let fixture = Fixture::new()?;
    save(
        &fixture,
        "initial",
        0,
        PlanProposal {
            tasks: vec![task("coarse", Granularity::Coarse, None, None, None)],
            ..Default::default()
        },
    )?;
    commit(&fixture, "initial", "initial", 0)?;
    object_tasks::cancel_planned(&fixture.runtime, &request("coarse", "cancel", 0, 1))?;
    let error = save(
        &fixture,
        "new-medium",
        2,
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
                Some("coarse"),
                None,
            )],
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_PARENT_CANCELLED"));
    assert_eq!(fixture.count("object_task")?, 1);
    Ok(())
}

#[test]
fn existing_cancelled_definitions_can_be_reused_without_recreating_records() -> Result<()> {
    let fixture = Fixture::new()?;
    committed_hierarchy(&fixture)?;
    object_tasks::cancel_planned(&fixture.runtime, &request("medium", "cancel", 0, 1))?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    save(
        &fixture,
        "history",
        2,
        PlanProposal {
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
            ..Default::default()
        },
    )?;
    let receipt = commit(&fixture, "history", "history", 2)?;
    assert_eq!(receipt.plan_revision, before.plan_revision);
    let after = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(after.tasks, before.tasks);
    assert_eq!(after.runs, before.runs);
    assert_eq!(receipt.runs, after.runs);
    Ok(())
}
