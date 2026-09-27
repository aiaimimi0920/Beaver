use super::publication_fixture::*;
use crate::{
    object_attempt_view,
    object_catalog_test_fixture::capture_request,
    object_run_preparation, object_run_recovery as recovery, object_tasks,
    object_version_acceptance::{self, AcceptanceRequest},
    object_version_capture,
};
use anyhow::Result;
use serde_json::json;

#[test]
fn publication_commits_acceptance_and_releases_next_task_to_the_new_baseline() -> Result<()> {
    let f = fixture()?;
    mutate(
        &f,
        "object_task",
        "next",
        "/identity/baseline",
        json!({"basePolicy":"latestAccepted"}),
    )?;
    let request = ready(&f)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let preview = publication::preview(&f.runtime, &review(&request))?;
    assert_eq!(preview.files[0].path, "hero.gd");
    assert_eq!(preview.files[0].role, "resource");
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "next")?.is_none());

    let published = publication::publish(&f.runtime, &request)?;
    assert_eq!(published.state, State::Published);
    assert_eq!(
        std::fs::read_to_string(f.temp.path().join("hero.gd"))?,
        "extends Node\nvar speed = 3\n"
    );
    assert_eq!(task_record(&f, "head")?.status, "accepted");
    assert_eq!(task_record(&f, "fine-a")?.status, "accepted");
    assert_eq!(task_record(&f, "fine-b")?.status, "accepted");
    let snapshot = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(snapshot.plan_revision, before.plan_revision + 1);
    let queue = object_tasks::queue(&f.runtime, "project-1")?;
    let head = queue.iter().find(|entry| entry.task_id == "head").unwrap();
    assert_eq!(head.state, "accepted");
    assert!(head.owner.is_none() && head.claim_token.is_none());
    assert!(recovery::get(&f.runtime, "project-1", "head")?.is_none());
    assert_eq!(
        object_attempt_view::list_with_details(&f.runtime, &request.target.run_id)?.len(),
        2
    );
    let object = f.object("hero")?;
    assert_eq!(object.files, preview.files);
    assert_eq!(
        object.versions.last().unwrap().version_id,
        published.version_id
    );
    assert_eq!(
        object.versions.last().unwrap().manifest["status"],
        "accepted"
    );
    let accepted = object_version_acceptance::accept(
        &f.runtime,
        &AcceptanceRequest {
            project_id: "project-1".into(),
            request_id: request.request_id.clone(),
            object_id: "hero".into(),
            version_id: published.version_id.clone(),
            expected_revision: preview.object_revision,
        },
    )?;
    assert_eq!(accepted.object, object);
    assert_eq!(publication::publish(&f.runtime, &request)?, published);
    let next = object_run_preparation::claim_next(&f.runtime, "project-1", "next")?.unwrap();
    assert_eq!(next.record().medium.id, "next");
    assert_eq!(
        next.record()
            .baseline
            .as_ref()
            .unwrap()
            .resolved_version_id
            .as_ref(),
        Some(&published.version_id)
    );
    assert_eq!(
        publication::list(&f.runtime, "project-1", "head")?,
        vec![published.clone()]
    );
    assert_eq!(publication::publish(&f.runtime, &request)?, published);
    Ok(())
}

#[test]
fn publication_rejects_changed_requests_and_receipt_collisions_before_writing() -> Result<()> {
    let f = fixture()?;
    let request = ready(&f)?;
    let pending = fail_at(&f, &request, "prepared")?;
    let mut changed = request.clone();
    changed.acceptance_note = "different authorization".into();
    assert!(publication::publish(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    assert_eq!(
        publication::list(&f.runtime, "project-1", "head")?,
        vec![pending]
    );
    assert!(!f.temp.path().join("hero.gd").exists());

    let f = fixture()?;
    object_version_capture::capture(&f.runtime, &capture_request(&f.object("rival")?, "publish"))?;
    let request = ready(&f)?;
    assert!(publication::publish(&f.runtime, &request)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REQUEST_CONFLICT"));
    assert_eq!(f.count("object_publication")?, 0);
    assert!(!f.temp.path().join("hero.gd").exists());
    Ok(())
}

#[test]
fn publication_pending_excludes_capture_and_recovery_but_allows_completed_capture_replay(
) -> Result<()> {
    let f = fixture()?;
    let capture = capture_request(&f.object("rival")?, "capture-rival");
    let captured = object_version_capture::capture(&f.runtime, &capture)?;
    let request = ready(&f)?;
    let verify = super::object_recovery_verification::request(&f, "during-publication")?;
    fail_at(&f, &request, "prepared")?;
    let new_capture = capture_request(&f.object("rival")?, "capture-again");
    assert!(object_version_capture::capture(&f.runtime, &new_capture)
        .unwrap_err()
        .to_string()
        .contains("PUBLICATION_PENDING"));
    assert!(recovery::verify(&f.runtime, &verify, false)
        .unwrap_err()
        .to_string()
        .contains("PUBLICATION_PENDING"));
    let view = recovery::get(&f.runtime, "project-1", "head")?.unwrap();
    assert!(!view.can_resume && !view.can_dispose);
    let _guard = f.runtime.materialization.lock().unwrap();
    assert_eq!(
        object_version_capture::capture(&f.runtime, &capture)?,
        captured
    );
    Ok(())
}

#[test]
fn publication_rejects_corrupt_write_journal_without_claiming_files() -> Result<()> {
    let f = fixture()?;
    let request = multiple_files(&f)?;
    fail_at(&f, &request, "prepared")?;
    let key = crate::framework_checks::digest(&("project-1", "publish"))?;
    mutate(&f, "object_publication", &key, "/writes", json!(["z.txt"]))?;
    assert!(publication::list(&f.runtime, "project-1", "head").is_err());
    assert!(publication::publish(&f.runtime, &request).is_err());
    assert!(publication::abort(&f.runtime, "project-1", "publish").is_err());
    assert!(!f.temp.path().join("z.txt").exists());
    Ok(())
}
