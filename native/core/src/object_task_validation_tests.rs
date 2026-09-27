use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{
        Granularity, ObjectProposal, PlanProposal, SaveDraftRequest, TaskProposal,
    },
    object_tasks,
};
use anyhow::Result;

fn task(
    id: &str,
    granularity: Granularity,
    object_id: Option<&str>,
    parent_task_id: Option<&str>,
    depends_on: &[&str],
    stage_id: Option<&str>,
) -> TaskProposal {
    TaskProposal {
        id: id.into(),
        position: 0,
        granularity,
        title: id.into(),
        prompt: format!("Work on {id}"),
        acceptance: String::new(),
        requirement: Default::default(),
        pending_planning: String::new(),
        object_id: object_id.map(str::to_owned),
        parent_task_id: parent_task_id.map(str::to_owned),
        depends_on: depends_on.iter().map(|id| (*id).into()).collect(),
        stage_id: stage_id.map(str::to_owned),
        baseline: None,
    }
}

fn save(fixture: &Fixture, id: &str, plan: PlanProposal) -> anyhow::Result<()> {
    object_tasks::save_draft(
        &fixture.runtime,
        &SaveDraftRequest {
            project_id: "project-1".into(),
            draft_id: id.into(),
            expected_revision: 0,
            expected_plan_revision: 0,
            plan,
        },
    )?;
    Ok(())
}

#[test]
fn draft_rejects_dependency_cycles_and_fine_tasks_on_another_object() -> Result<()> {
    let fixture = Fixture::new()?;
    let cycle = PlanProposal {
        objects: vec![],
        tasks: vec![
            task("a", Granularity::Coarse, None, None, &["b"], None),
            task("b", Granularity::Coarse, None, None, &["a"], None),
        ],
        assumptions: vec![],
    };
    let error = save(&fixture, "cycle", cycle).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_DEPENDENCY_CYCLE"));

    let cross_object = PlanProposal {
        objects: vec![
            ObjectProposal {
                id: "one".into(),
                name: "One".into(),
                category: "other".into(),
            },
            ObjectProposal {
                id: "two".into(),
                name: "Two".into(),
                category: "other".into(),
            },
        ],
        tasks: vec![
            task("medium", Granularity::Medium, Some("one"), None, &[], None),
            task(
                "fine",
                Granularity::Fine,
                Some("two"),
                Some("medium"),
                &[],
                Some("stage"),
            ),
        ],
        assumptions: vec![],
    };
    let error = save(&fixture, "cross-object", cross_object).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_FINE_PARENT_MISMATCH"));
    assert_eq!(fixture.count("object")?, 0);
    assert_eq!(fixture.count("object_task")?, 0);
    Ok(())
}
