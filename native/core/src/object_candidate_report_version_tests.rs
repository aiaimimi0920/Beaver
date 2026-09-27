use super::{object_candidate_review::final_output, publication_fixture::*};
use crate::{
    object_catalog_test_fixture::Fixture, object_run_recovery::candidate,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::json;

fn legacy(f: &Fixture, report: &mut candidate::Report) -> Result<()> {
    report.schema_version = 1;
    report
        .blockers
        .retain(|item| item != "PUBLICATION_CONFIRMATION_REQUIRED");
    report.blockers.splice(
        1..1,
        [
            "FEEDBACK_REVIEW_UNAVAILABLE".into(),
            "PUBLICATION_NOT_IMPLEMENTED".into(),
        ],
    );
    mutate(
        f,
        "object_candidate_review",
        &report.request.request_id,
        "/report",
        serde_json::to_value(&*report)?,
    )
}

#[test]
fn candidate_versions_reopen_replay_and_publish_without_rewriting_frozen_evidence() -> Result<()> {
    for version in [1, 2] {
        let f = fixture()?;
        let review = final_output(&f)?;
        let mut report = candidate::prepare(&f.runtime, &review)?;
        assert_eq!(report.schema_version, 2);
        assert!(report
            .blockers
            .iter()
            .any(|item| item == "PUBLICATION_CONFIRMATION_REQUIRED"));
        assert!(!report
            .blockers
            .iter()
            .any(|item| item == "PUBLICATION_NOT_IMPLEMENTED"
                || item == "FEEDBACK_REVIEW_UNAVAILABLE"));
        if version == 1 {
            legacy(&f, &mut report)?;
        }
        let publish = request(&f, &review)?;
        let Fixture { runtime, temp } = f;
        drop(runtime);
        let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(candidate::prepare(&runtime, &review)?, report);
        assert_eq!(
            candidate::list(&runtime, "project-1", &review.target.attempt_id)?,
            vec![report.clone()]
        );
        let published = publication::publish(&runtime, &publish)?;
        assert_eq!(published.state, State::Published);
        assert_eq!(publication::publish(&runtime, &publish)?, published);
        assert_eq!(candidate::prepare(&runtime, &review)?, report);
        assert_eq!(
            std::fs::read_to_string(temp.path().join("hero.gd"))?,
            "extends Node\nvar speed = 3\n"
        );
    }
    Ok(())
}

#[test]
fn candidate_version_or_condition_tampering_cannot_authorize_publication() -> Result<()> {
    let f = fixture()?;
    let review = final_output(&f)?;
    let report = candidate::prepare(&f.runtime, &review)?;
    let publish = request(&f, &review)?;
    for bad in [
        {
            let mut value = serde_json::to_value(&report)?;
            value["schemaVersion"] = json!(3);
            value
        },
        {
            let mut value = serde_json::to_value(&report)?;
            value["schemaVersion"] = json!(1);
            value
        },
        {
            let mut value = serde_json::to_value(&report)?;
            value["blockers"] = json!([]);
            value
        },
    ] {
        mutate(
            &f,
            "object_candidate_review",
            &review.request_id,
            "/report",
            bad,
        )?;
        assert!(candidate::list(&f.runtime, "project-1", &review.target.attempt_id).is_err());
        assert!(publication::publish(&f.runtime, &publish).is_err());
        assert_eq!(f.count("object_publication")?, 0);
        assert!(!f.temp.path().join("hero.gd").exists());
    }
    Ok(())
}
