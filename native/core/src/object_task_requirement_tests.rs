use super::{
    commit::{commit, save, task},
    definition_fixture::{fixture, rejected, request},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_types::{Granularity, PlanProposal, TaskProposal, WorkRequirement},
    object_tasks::{self, TaskDefinition},
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn old_definitions_default_to_required_without_changing_frozen_json() -> Result<()> {
    let proposal = task("coarse", Granularity::Coarse, None, None, None);
    let original = serde_json::to_value(&proposal)?;
    assert!(original.get("requirement").is_none());
    let loaded: TaskProposal = serde_json::from_value(original.clone())?;
    assert_eq!(loaded.requirement, WorkRequirement::Required);
    assert_eq!(serde_json::to_value(&loaded)?, original);

    let f = fixture()?;
    let record = object_tasks::get_task(&f.runtime, "project-1", "fine")?.unwrap();
    let original = serde_json::to_value(&record)?;
    assert!(original.get("requirement").is_none());
    let loaded: crate::object_task_types::TaskRecord = serde_json::from_value(original.clone())?;
    assert_eq!(loaded.requirement, WorkRequirement::Required);
    assert_eq!(serde_json::to_value(&loaded)?, original);
    let definition = serde_json::to_value(TaskDefinition::from_task(&record))?;
    let loaded: TaskDefinition = serde_json::from_value(definition.clone())?;
    assert_eq!(loaded.requirement, WorkRequirement::Required);
    assert_eq!(serde_json::to_value(&loaded)?, definition);
    let mut invalid = original;
    invalid["requirement"] = "ignored".into();
    assert!(serde_json::from_value::<crate::object_task_types::TaskRecord>(invalid).is_err());
    Ok(())
}

#[test]
fn optional_draft_commits_reopens_and_cannot_be_overwritten_by_duplicate_proposal() -> Result<()> {
    let f = Fixture::new()?;
    let mut proposal = task("coarse", Granularity::Coarse, None, None, None);
    proposal.requirement = WorkRequirement::Optional;
    let plan = PlanProposal {
        tasks: vec![proposal.clone()],
        ..Default::default()
    };
    save(&f, "optional", 0, plan)?;
    commit(&f, "optional", "optional", 0)?;
    proposal.requirement = WorkRequirement::Required;
    save(
        &f,
        "conflict",
        1,
        PlanProposal {
            tasks: vec![proposal],
            ..Default::default()
        },
    )?;
    let error = commit(&f, "conflict", "conflict", 1).unwrap_err();
    assert!(format!("{error:#}").contains("OBJECT_TASK_ID_CONFLICT"));
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let snapshot = object_tasks::snapshot(&runtime, "project-1")?;
    assert_eq!(snapshot.plan_revision, 1);
    assert_eq!(snapshot.tasks[0].requirement, WorkRequirement::Optional);
    assert_eq!(
        serde_json::to_value(&snapshot.tasks[0])?["requirement"],
        "optional"
    );
    Ok(())
}

#[test]
fn classification_only_revision_has_audited_history_and_stable_retry_after_reopen() -> Result<()> {
    let f = fixture()?;
    let mut change = request(&f, "fine", "scope-change")?;
    let original = object_tasks::get_task(&f.runtime, "project-1", "fine")?.unwrap();
    change.definition = TaskDefinition::from_task(&original);
    change.definition.requirement = WorkRequirement::Optional;
    change.reason = "Owner removed this stage from the required scope".into();
    let receipt = object_tasks::revise_planned(&f.runtime, &change)?;
    assert_eq!(receipt.before.requirement, WorkRequirement::Required);
    assert_eq!(receipt.after.requirement, WorkRequirement::Optional);
    assert_eq!(receipt.reason, change.reason);
    let mut altered_retry = change.clone();
    altered_retry.definition.requirement = WorkRequirement::Required;
    rejected(&f, &altered_retry, "OBJECT_TASK_REQUEST_ID_CONFLICT")?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(object_tasks::revise_planned(&runtime, &change)?, receipt);
    assert_eq!(
        object_tasks::revisions(&runtime, "project-1", "fine")?,
        [receipt]
    );
    let task = object_tasks::get_task(&runtime, "project-1", "fine")?.unwrap();
    assert_eq!(task.requirement, WorkRequirement::Optional);
    assert_eq!(task.identity, original.identity);
    assert_eq!(task.status, "planned");
    Ok(())
}
