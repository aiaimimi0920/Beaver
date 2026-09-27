use super::{
    cancel,
    definition_fixture::{fixture, rejected, request},
};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_tasks::{self, DefinitionAdopter, TaskDefinition},
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn revisions_preserve_identity_runs_and_unrelated_records_at_every_granularity() -> Result<()> {
    let f = fixture()?;
    let object = f.object("hero")?;
    for id in ["coarse", "medium", "fine"] {
        let before = object_tasks::snapshot(&f.runtime, "project-1")?;
        let request = request(&f, id, &format!("revise-{id}"))?;
        let receipt = object_tasks::revise_planned(&f.runtime, &request)?;
        let after = object_tasks::snapshot(&f.runtime, "project-1")?;
        assert_eq!(after.plan_revision, before.plan_revision + 1);
        assert_eq!(after.runs, before.runs);
        assert_eq!(after.assumptions, before.assumptions);
        assert_eq!(f.object("hero")?, object);
        for task in &before.tasks {
            let saved = after
                .tasks
                .iter()
                .find(|saved| saved.id == task.id)
                .unwrap();
            if task.id != id {
                assert_eq!(saved, task);
                continue;
            }
            let mut expected = task.clone();
            expected.title = request.definition.title.clone();
            expected.prompt = request.definition.prompt.clone();
            expected.acceptance = request.definition.acceptance.clone();
            expected.depends_on = request.definition.depends_on.clone();
            expected.revision += 1;
            assert_eq!(saved, &expected);
            assert_eq!(receipt.before, TaskDefinition::from_task(task));
        }
        assert_eq!(receipt.after, request.definition);
        assert_eq!(receipt.reason, request.reason);
        assert_eq!(receipt.adopted_by, DefinitionAdopter::Owner);
        assert!(chrono::DateTime::parse_from_rfc3339(&receipt.created_at).is_ok());
        let mut impact = vec!["dependent", "dependent-child", "fine", "transitive"];
        if id != "fine" {
            impact.push("medium");
        }
        if id == "coarse" {
            impact.push("coarse");
        }
        impact.sort();
        assert_eq!(receipt.affected_task_ids, impact);
        assert_eq!(
            object_tasks::revisions(&f.runtime, "project-1", id)?,
            [receipt]
        );
    }
    assert_eq!(f.count("object_task_definition_revision")?, 3);
    assert_eq!(f.count("task")?, 0);
    Ok(())
}

#[test]
fn history_and_original_retry_survive_later_revision_cancellation_and_reopen() -> Result<()> {
    let f = fixture()?;
    let original = request(&f, "medium", "z-first")?;
    let first = object_tasks::revise_planned(&f.runtime, &original)?;
    let next = request(&f, "medium", "a-second")?;
    let second = object_tasks::revise_planned(&f.runtime, &next)?;
    object_tasks::cancel_planned(&f.runtime, &cancel::request("coarse", "cancel", 0, 3))?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        object_tasks::revisions(&runtime, "project-1", "medium")?,
        [first.clone(), second]
    );
    assert_eq!(object_tasks::revise_planned(&runtime, &original)?, first);
    assert_eq!(
        object_tasks::snapshot(&runtime, "project-1")?.plan_revision,
        4
    );
    assert_eq!(
        object_tasks::get_task(&runtime, "project-1", "medium")?
            .unwrap()
            .status,
        "cancelled"
    );
    Ok(())
}

#[test]
fn request_id_cannot_be_reused_for_changed_definition_reason_identity_or_versions() -> Result<()> {
    let f = fixture()?;
    let original = request(&f, "fine", "stable")?;
    object_tasks::revise_planned(&f.runtime, &original)?;
    let mut changed = vec![original.clone(); 6];
    changed[0].definition.prompt.push_str(" different");
    changed[1].reason.push_str(" different");
    changed[2].task_id = "medium".into();
    changed[3].expected_task_revision += 1;
    changed[4].expected_plan_revision += 1;
    changed[5].definition.depends_on.clear();
    for request in changed {
        rejected(&f, &request, "OBJECT_TASK_REQUEST_ID_CONFLICT")?;
    }
    Ok(())
}

#[test]
fn impact_traverses_cancelled_dependents_without_listing_them() -> Result<()> {
    let f = fixture()?;
    object_tasks::cancel_planned(&f.runtime, &cancel::request("dependent", "cancel", 0, 1))?;
    let receipt = object_tasks::revise_planned(&f.runtime, &request(&f, "fine", "revise")?)?;
    assert_eq!(receipt.affected_task_ids, ["fine", "transitive"]);
    Ok(())
}
