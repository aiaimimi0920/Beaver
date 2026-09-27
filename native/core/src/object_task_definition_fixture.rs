use super::commit::{commit, save, task};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_tasks::{
        self, Granularity, ObjectProposal, PlanProposal, RevisePlannedRequest, TaskDefinition,
    },
};
use anyhow::Result;

pub(super) fn fixture() -> Result<Fixture> {
    let fixture = Fixture::new()?;
    let mut dependent = task("dependent", Granularity::Coarse, None, None, None);
    dependent.depends_on = vec!["fine".into()];
    let mut transitive = task("transitive", Granularity::Coarse, None, None, None);
    transitive.depends_on = vec!["dependent-child".into()];
    save(
        &fixture,
        "initial",
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
                task(
                    "fine",
                    Granularity::Fine,
                    Some("hero"),
                    Some("medium"),
                    Some("mesh"),
                ),
                task("other", Granularity::Coarse, None, None, None),
                task(
                    "shared-object",
                    Granularity::Medium,
                    Some("hero"),
                    None,
                    None,
                ),
                dependent,
                task(
                    "dependent-child",
                    Granularity::Medium,
                    Some("hero"),
                    Some("dependent"),
                    None,
                ),
                transitive,
            ],
            assumptions: vec![],
        },
    )?;
    commit(&fixture, "initial-commit", "initial", 0)?;
    Ok(fixture)
}

pub(super) fn request(
    fixture: &Fixture,
    task_id: &str,
    request_id: &str,
) -> Result<RevisePlannedRequest> {
    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .unwrap();
    let mut definition = TaskDefinition::from_task(task);
    definition.title = format!("Revised {request_id}");
    definition.prompt = "Implement the clarified goal".into();
    definition.acceptance = "Demonstrate the clarified result".into();
    definition.depends_on = vec!["other".into()];
    Ok(RevisePlannedRequest {
        project_id: "project-1".into(),
        task_id: task_id.into(),
        request_id: request_id.into(),
        expected_task_revision: task.revision,
        expected_plan_revision: snapshot.plan_revision,
        definition,
        reason: "Clarify the requested behavior".into(),
    })
}

pub(super) fn rejected(
    fixture: &Fixture,
    request: &RevisePlannedRequest,
    code: &str,
) -> Result<()> {
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let count = fixture.count("object_task_definition_revision")?;
    let error = object_tasks::revise_planned(&fixture.runtime, request).unwrap_err();
    assert!(
        format!("{error:#}").contains(code),
        "expected {code}, got {error:#}"
    );
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        before
    );
    assert_eq!(fixture.count("object_task_definition_revision")?, count);
    Ok(())
}
