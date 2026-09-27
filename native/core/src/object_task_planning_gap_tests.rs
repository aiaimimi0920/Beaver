use super::{
    commit::{commit, save, task},
    definition_fixture::{fixture, rejected, request},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{Granularity, PlanProposal},
    object_tasks::{self, TaskDefinition},
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn pending_requirements_survive_draft_commit_and_reopen() -> Result<()> {
    let f = Fixture::new()?;
    let mut proposal = task("coarse", Granularity::Coarse, None, None, None);
    proposal.pending_planning = "Plan scene integration after object prototypes".into();
    save(
        &f,
        "gaps",
        0,
        PlanProposal {
            tasks: vec![proposal.clone()],
            ..Default::default()
        },
    )?;
    commit(&f, "gaps", "gaps", 0)?;
    proposal.pending_planning.clear();
    save(
        &f,
        "conflict",
        1,
        PlanProposal {
            tasks: vec![proposal],
            ..Default::default()
        },
    )?;
    assert!(
        format!("{:#}", commit(&f, "conflict", "conflict", 1).unwrap_err())
            .contains("OBJECT_TASK_ID_CONFLICT")
    );
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let record = object_tasks::get_task(&runtime, "project-1", "coarse")?.unwrap();
    assert_eq!(
        record.pending_planning,
        "Plan scene integration after object prototypes"
    );
    assert_eq!(record.status, "planned");
    Ok(())
}

#[test]
fn planning_gap_revision_is_audited_and_rejects_invalid_scope_and_retry() -> Result<()> {
    let f = fixture()?;
    let original = object_tasks::get_task(&f.runtime, "project-1", "medium")?.unwrap();
    let mut change = request(&f, "medium", "planning-gap")?;
    change.definition = TaskDefinition::from_task(&original);
    change.definition.pending_planning = "Lighting and export remain unplanned".into();
    let receipt = object_tasks::revise_planned(&f.runtime, &change)?;
    assert!(receipt.before.pending_planning.is_empty());
    assert_eq!(
        receipt.after.pending_planning,
        change.definition.pending_planning
    );
    let mut retry = change.clone();
    retry.definition.pending_planning.clear();
    rejected(&f, &retry, "OBJECT_TASK_REQUEST_ID_CONFLICT")?;
    let mut invalid = request(&f, "fine", "invalid-fine")?;
    invalid.definition.pending_planning = "Nested stage".into();
    rejected(&f, &invalid, "INVALID_OBJECT_TASK: pending planning")?;
    let mut oversized = request(&f, "medium", "oversized-gap")?;
    oversized.definition.pending_planning = "x".repeat(4_001);
    rejected(&f, &oversized, "INVALID_OBJECT_TASK: pending planning")?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(object_tasks::revise_planned(&runtime, &change)?, receipt);
    assert_eq!(
        object_tasks::revisions(&runtime, "project-1", "medium")?,
        [receipt]
    );
    Ok(())
}
