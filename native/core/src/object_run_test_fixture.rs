use super::{
    commit::{commit, save, task},
    queue_fixture,
};
use crate::{
    object_catalog::{ObjectFile, ObjectReference},
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
    object_framework::Baseline,
    object_registration,
    object_run_preparation::{self, PreparationClaim},
    object_task_types::{Granularity, PlanProposal},
    object_tasks,
    object_version_acceptance::{self, AcceptanceRequest},
    object_version_capture,
};
use anyhow::Result;
use std::fs;

pub(super) fn fixture() -> Result<Fixture> {
    queue_fixture::fixture(false)
}

pub(super) fn capture(
    fixture: &Fixture,
    object_id: &str,
    label: &str,
    content: &str,
    references: Vec<ObjectReference>,
) -> Result<String> {
    let path = format!("{object_id}.tscn");
    fs::write(fixture.temp.path().join(&path), content)?;
    let object = fixture.object(object_id)?;
    let mut request = update_request(&object, &format!("update-{label}"));
    request.files = vec![ObjectFile {
        path,
        role: "scene".into(),
    }];
    request.references = references;
    let object = object_registration::update(&fixture.runtime, &request)?.object;
    Ok(object_version_capture::capture(
        &fixture.runtime,
        &capture_request(&object, &format!("capture-{label}")),
    )?
    .version_id
    .unwrap())
}

pub(super) fn accept(fixture: &Fixture, object_id: &str, version_id: &str) -> Result<()> {
    let object = fixture.object(object_id)?;
    object_version_acceptance::accept(
        &fixture.runtime,
        &AcceptanceRequest {
            project_id: "project-1".into(),
            request_id: format!("accept-{version_id}"),
            object_id: object_id.into(),
            version_id: version_id.into(),
            expected_revision: object.revision,
        },
    )?;
    Ok(())
}

pub(super) fn claim(fixture: &Fixture, baseline: Option<Baseline>) -> Result<PreparationClaim> {
    let revision = object_tasks::snapshot(&fixture.runtime, "project-1")?.plan_revision;
    let mut medium = task("work", Granularity::Medium, Some("hero"), None, None);
    medium.baseline = baseline;
    save(
        fixture,
        "run-plan",
        revision,
        PlanProposal {
            tasks: vec![medium],
            ..PlanProposal::default()
        },
    )?;
    commit(fixture, "commit-run", "run-plan", revision)?;
    queue_fixture::enqueue(fixture, &["work"])?;
    Ok(object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap())
}
