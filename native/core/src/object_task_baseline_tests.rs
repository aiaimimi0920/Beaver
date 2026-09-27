use super::{
    commit::{commit, save, task},
    run_fixture::{accept, capture, fixture},
};
use crate::{
    object_framework::{Baseline, Identity},
    object_tasks::{self, Granularity, PlanProposal, RevisePlannedRequest, TaskDefinition},
};
use anyhow::Result;

#[test]
fn baseline_choices_survive_save_commit_replay_and_definition_revision() -> Result<()> {
    for policy in [
        None,
        Some(Baseline::LatestAccepted {}),
        Some(Baseline::Empty {}),
        Some(Baseline::PinnedVersion {
            selected_version_id: "accepted".into(),
        }),
    ] {
        let fixture = fixture()?;
        fixture.accepted("hero", "accepted")?;
        let mut proposed = task("work", Granularity::Medium, Some("hero"), None, None);
        proposed.baseline = policy.clone();
        let serialized = serde_json::to_value(&proposed)?;
        assert_eq!(serialized.get("baseline").is_some(), policy.is_some());
        save(
            &fixture,
            "baseline-plan",
            1,
            PlanProposal {
                tasks: vec![proposed],
                ..PlanProposal::default()
            },
        )?;
        let draft =
            object_tasks::get_draft(&fixture.runtime, "project-1", "baseline-plan")?.unwrap();
        assert_eq!(draft.plan.tasks[0].baseline, policy);
        let receipt = commit(&fixture, "commit-baseline", "baseline-plan", 1)?;
        assert!(receipt.runs[0].baseline_version_id.is_none());
        assert_eq!(
            commit(&fixture, "commit-baseline", "baseline-plan", 1)?,
            receipt
        );
        let saved = object_tasks::get_task(&fixture.runtime, "project-1", "work")?.unwrap();
        let expected = policy.unwrap_or(Baseline::LatestAccepted {});
        assert!(
            matches!(&saved.identity, Identity::Medium { baseline, .. } if baseline == &expected)
        );
        let mut definition = TaskDefinition::from_task(&saved);
        definition.title = "Clarified work".into();
        object_tasks::revise_planned(
            &fixture.runtime,
            &RevisePlannedRequest {
                project_id: "project-1".into(),
                task_id: "work".into(),
                request_id: "revise-work".into(),
                expected_task_revision: saved.revision,
                expected_plan_revision: receipt.plan_revision,
                definition,
                reason: "Clarify title".into(),
            },
        )?;
        let revised = object_tasks::get_task(&fixture.runtime, "project-1", "work")?.unwrap();
        assert_eq!(revised.title, "Clarified work");
        assert_eq!(revised.identity, saved.identity);
        assert_eq!(revised.run_id, saved.run_id);
    }
    Ok(())
}

#[test]
fn only_medium_tasks_may_select_a_baseline() -> Result<()> {
    let fixture = fixture()?;
    for mut proposed in [
        task("work", Granularity::Coarse, None, None, None),
        task(
            "work",
            Granularity::Fine,
            Some("hero"),
            Some("head"),
            Some("mesh"),
        ),
    ] {
        proposed.baseline = Some(Baseline::Empty {});
        let error = save(
            &fixture,
            "invalid-baseline",
            1,
            PlanProposal {
                tasks: vec![proposed],
                ..PlanProposal::default()
            },
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("INVALID_OBJECT_TASK_IDENTITY"));
        assert!(
            object_tasks::get_draft(&fixture.runtime, "project-1", "invalid-baseline")?.is_none()
        );
    }
    Ok(())
}

#[test]
fn pinned_baseline_must_name_an_accepted_version_on_its_object() -> Result<()> {
    let fixture = fixture()?;
    let candidate = capture(&fixture, "hero", "candidate", "candidate", vec![])?;
    let foreign = capture(&fixture, "rival", "foreign", "foreign", vec![])?;
    accept(&fixture, "rival", &foreign)?;
    for (version, expected) in [
        ("", "INVALID_OBJECT_TASK_BASELINE_VERSION"),
        ("../foreign", "INVALID_OBJECT_TASK_BASELINE_VERSION"),
        ("missing", "OBJECT_VERSION_NOT_FOUND"),
        (foreign.as_str(), "OBJECT_VERSION_NOT_FOUND"),
        (candidate.as_str(), "IMPORT_VERSION_NOT_ACCEPTED"),
    ] {
        let mut proposed = task("work", Granularity::Medium, Some("hero"), None, None);
        proposed.baseline = Some(Baseline::PinnedVersion {
            selected_version_id: version.into(),
        });
        let error = save(
            &fixture,
            "invalid-pin",
            1,
            PlanProposal {
                tasks: vec![proposed],
                ..PlanProposal::default()
            },
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        assert!(object_tasks::get_draft(&fixture.runtime, "project-1", "invalid-pin")?.is_none());
    }
    Ok(())
}
