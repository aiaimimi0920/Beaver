use super::run_fixture::{accept, capture, claim, fixture};
use crate::{
    object_framework::Baseline,
    object_run_preparation::{self, PreparationState},
    object_task_queue,
};
use anyhow::Result;

#[test]
fn latest_uses_acceptance_order_and_freezes_before_later_acceptance() -> Result<()> {
    let fixture = fixture()?;
    let first = capture(&fixture, "hero", "first", "first", vec![])?;
    let second = capture(&fixture, "hero", "second", "second", vec![])?;
    accept(&fixture, "hero", &second)?;
    accept(&fixture, "hero", &first)?;
    let third = capture(&fixture, "hero", "third", "third", vec![])?;
    let claimed = claim(&fixture, None)?;
    let frozen = claimed.record().baseline.as_ref().unwrap().clone();
    assert_eq!(frozen.resolved_version_id.as_deref(), Some(first.as_str()));
    assert_eq!(
        frozen.accepted_version_id_at_claim,
        frozen.resolved_version_id
    );
    accept(&fixture, "hero", &third)?;
    let ready = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert_eq!(ready.state, PreparationState::Ready);
    assert_eq!(ready.baseline, Some(frozen));
    let workspace = fixture.runtime.files().workspace(&ready.id)?;
    assert_eq!(
        std::fs::read_to_string(workspace.join("hero.tscn"))?,
        "first"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.temp.path().join("hero.tscn"))?,
        "third"
    );
    Ok(())
}

#[test]
fn pinned_and_empty_preserve_the_independent_acceptance_comparison_point() -> Result<()> {
    for empty in [false, true] {
        let fixture = fixture()?;
        let first = capture(&fixture, "hero", "first", "first", vec![])?;
        accept(&fixture, "hero", &first)?;
        let second = capture(&fixture, "hero", "second", "second", vec![])?;
        accept(&fixture, "hero", &second)?;
        let policy = if empty {
            Baseline::Empty {}
        } else {
            Baseline::PinnedVersion {
                selected_version_id: first.clone(),
            }
        };
        let claimed = claim(&fixture, Some(policy.clone()))?;
        let baseline = claimed.record().baseline.as_ref().unwrap();
        assert_eq!(baseline.policy, policy);
        assert_eq!(
            baseline.accepted_version_id_at_claim.as_deref(),
            Some(second.as_str())
        );
        assert_eq!(baseline.resolved_version_id, (!empty).then_some(first));
        let prepared = object_run_preparation::prepare(&fixture.runtime, claimed)?;
        assert_eq!(prepared.state, PreparationState::Ready);
        let path = fixture
            .runtime
            .files()
            .workspace(&prepared.id)?
            .join("hero.tscn");
        if empty {
            assert!(!path.exists());
        } else {
            assert_eq!(std::fs::read_to_string(path)?, "first");
        }
    }
    Ok(())
}

#[test]
fn missing_latest_fails_without_files_and_keeps_the_writer() -> Result<()> {
    let fixture = fixture()?;
    let claimed = claim(&fixture, None)?;
    assert_eq!(claimed.record().state, PreparationState::Failed);
    assert!(claimed
        .record()
        .error
        .as_ref()
        .unwrap()
        .contains("OBJECT_BASELINE_NO_ACCEPTED_VERSION"));
    let prepared = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert!(!fixture.runtime.files().workspace(&prepared.id)?.exists());
    super::queue_fixture::enqueue(&fixture, &["head", "independent"])?;
    let next = object_task_queue::claim_next(&fixture.runtime, "project-1", "next")?.unwrap();
    assert_eq!(next.task.id, "independent");
    assert!(object_task_queue::claim_next(&fixture.runtime, "project-1", "other")?.is_none());
    let snapshot = crate::object_tasks::snapshot(&fixture.runtime, "project-1")?;
    assert_eq!(
        snapshot
            .tasks
            .iter()
            .find(|task| task.id == "work")
            .unwrap()
            .status,
        "failed"
    );
    assert_eq!(
        snapshot
            .runs
            .iter()
            .find(|run| run.id == prepared.id)
            .unwrap()
            .status,
        "failed"
    );
    Ok(())
}

#[test]
fn explicit_empty_is_ready_without_any_accepted_version() -> Result<()> {
    let fixture = fixture()?;
    let claimed = claim(&fixture, Some(Baseline::Empty {}))?;
    let prepared = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert_eq!(prepared.state, PreparationState::Ready);
    assert!(prepared.baseline.unwrap().versions.is_empty());
    assert_eq!(
        std::fs::read_dir(fixture.runtime.files().workspace(&prepared.id)?)?.count(),
        0
    );
    Ok(())
}

#[test]
fn a_single_legacy_acceptance_has_unambiguous_order() -> Result<()> {
    let fixture = fixture()?;
    fixture.accepted("hero", "legacy")?;
    let claimed = claim(&fixture, None)?;
    assert_eq!(
        claimed
            .record()
            .baseline
            .as_ref()
            .unwrap()
            .resolved_version_id
            .as_deref(),
        Some("legacy")
    );
    assert_eq!(
        object_run_preparation::prepare(&fixture.runtime, claimed)?.state,
        PreparationState::Ready
    );
    Ok(())
}
