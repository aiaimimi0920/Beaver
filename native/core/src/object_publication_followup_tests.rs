use super::{publication_fixture::*, run_fixture};
use crate::{
    object_framework::{Baseline, Identity},
    object_task_types::CancelPlannedRequest,
    object_tasks,
};
use anyhow::Result;
use publication::followup::{self, Request};

#[path = "object_publication_frame_tests.rs"]
mod frames;

fn input(op: &publication::Operation) -> Request {
    Request {
        project_id: "project-1".into(),
        request_id: "later-feedback".into(),
        publication_request_id: op.request.request_id.clone(),
        version_id: op.version_id.clone(),
        title: "Improve movement".into(),
        feedback: "Reduce speed near obstacles".into(),
        acceptance: "Slow down only when an obstacle is near".into(),
        preview_frame: None,
    }
}

#[test]
fn publication_followup_is_atomic_durable_and_pins_the_published_version() -> Result<()> {
    let f = fixture()?;
    let publish = ready(&f)?;
    let op = publication::publish(&f.runtime, &publish)?;
    let request = input(&op);
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let queue = object_tasks::queue(&f.runtime, "project-1")?;
    let receipt = followup::create(&f.runtime, &request)?;
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(after.tasks.len(), before.tasks.len() + 2);
    assert_eq!(after.runs.len(), before.runs.len() + 1);
    assert_eq!(after.plan_revision, before.plan_revision + 1);
    assert_eq!(object_tasks::queue(&f.runtime, "project-1")?, queue);
    let medium = task_record(&f, &receipt.medium_task_id)?;
    assert_eq!(medium.status, "planned");
    assert_eq!(medium.depends_on, vec!["head"]);
    assert!(
        matches!(medium.identity, Identity::Medium { baseline: Baseline::PinnedVersion { selected_version_id }, .. } if selected_version_id == op.version_id)
    );
    let fine = task_record(&f, &receipt.fine_task_id)?;
    assert_eq!(fine.parent_task_id.as_ref(), Some(&receipt.medium_task_id));
    assert_eq!(fine.run_id.as_ref(), Some(&receipt.run_id));
    assert!(fine.prompt.contains(&request.feedback));
    assert!(fine.prompt.contains(&publish.target.attempt_id));
    assert_eq!(receipt.source, publish.target);
    assert_eq!(followup::create(&f.runtime, &request)?, receipt);
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, after);
    // A newer accepted head must not silently retarget old feedback.
    let newer = run_fixture::capture(&f, "hero", "newer", "new head", vec![])?;
    run_fixture::accept(&f, "hero", &newer)?;
    object_tasks::cancel_planned(
        &f.runtime,
        &CancelPlannedRequest {
            project_id: "project-1".into(),
            task_id: "next".into(),
            request_id: "cancel-next".into(),
            expected_task_revision: task_record(&f, "next")?.revision,
            expected_plan_revision: object_tasks::snapshot(&f.runtime, "project-1")?.plan_revision,
        },
    )?;
    object_tasks::enqueue(&f.runtime, "project-1", &[receipt.medium_task_id.clone()])?;
    let claim =
        crate::object_run_preparation::claim_next(&f.runtime, "project-1", "followup-worker")?
            .unwrap();
    assert_eq!(claim.record().medium.id, receipt.medium_task_id);
    assert_eq!(
        claim
            .record()
            .baseline
            .as_ref()
            .unwrap()
            .resolved_version_id
            .as_ref(),
        Some(&op.version_id)
    );
    drop(claim);
    let crate::object_catalog_test_fixture::Fixture { runtime, temp } = f;
    drop(runtime);
    let reopened =
        crate::project_storage::ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        followup::list(&reopened, "project-1", "publish")?,
        vec![receipt.clone()]
    );
    assert_eq!(followup::create(&reopened, &request)?, receipt);
    Ok(())
}

#[test]
fn publication_followup_rejects_unpublished_wrong_version_and_changed_retry() -> Result<()> {
    let f = fixture()?;
    let publish = ready(&f)?;
    let pending = fail_at(&f, &publish, "prepared")?;
    let request = input(&pending);
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert!(followup::create(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("NOT_PUBLISHED"));
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    let op = publication::publish(&f.runtime, &publish)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut bad = input(&op);
    bad.version_id = "unrelated-version".into();
    assert!(followup::create(&f.runtime, &bad)
        .unwrap_err()
        .to_string()
        .contains("VERSION_MISMATCH"));
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    let receipt = followup::create(&f.runtime, &request)?;
    let after = object_tasks::snapshot(&f.runtime, "project-1")?;
    let mut changed = request.clone();
    changed.feedback = "Different instruction".into();
    assert!(followup::create(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_ID_CONFLICT"));
    changed.project_id = "foreign".into();
    assert!(followup::create(&f.runtime, &changed).is_err());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, after);
    assert_eq!(
        followup::list(&f.runtime, "project-1", "publish")?,
        vec![receipt]
    );
    Ok(())
}
