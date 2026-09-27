use super::commit::{commit, save, task};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_queue,
    object_task_types::{Granularity, ObjectProposal, PlanProposal},
};
use anyhow::Result;

pub(super) fn fixture(block_head: bool) -> Result<Fixture> {
    let fixture = Fixture::new()?;
    let mut head = task(
        "head",
        Granularity::Medium,
        Some("hero"),
        Some("root"),
        None,
    );
    if block_head {
        head.depends_on.push("gate".into());
    }
    let mut tasks = vec![
        task("root", Granularity::Coarse, None, None, None),
        head,
        task(
            "fine",
            Granularity::Fine,
            Some("hero"),
            Some("head"),
            Some("mesh"),
        ),
        task(
            "next",
            Granularity::Medium,
            Some("hero"),
            Some("root"),
            None,
        ),
        task(
            "independent",
            Granularity::Medium,
            Some("rival"),
            None,
            None,
        ),
        task("gate", Granularity::Coarse, None, None, None),
    ];
    for (position, task) in tasks.iter_mut().enumerate() {
        task.position = position as u64;
    }
    save(
        &fixture,
        "queue-plan",
        0,
        PlanProposal {
            objects: ["hero", "rival"]
                .into_iter()
                .map(|id| ObjectProposal {
                    id: id.into(),
                    name: id.into(),
                    category: "character".into(),
                })
                .collect(),
            tasks,
            assumptions: vec![],
        },
    )?;
    commit(&fixture, "commit-queue-plan", "queue-plan", 0)?;
    Ok(fixture)
}

pub(super) fn enqueue(fixture: &Fixture, ids: &[&str]) -> Result<()> {
    object_task_queue::enqueue(
        &fixture.runtime,
        "project-1",
        &ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>(),
    )?;
    Ok(())
}
