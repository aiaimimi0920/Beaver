use super::{attempt_fixture::*, object_candidate_review::final_output};
use crate::{
    object_attempt_checks as checks,
    object_catalog_test_fixture::Fixture,
    object_run_recovery::candidate,
    object_tasks,
    project_storage::ProjectStore,
    validation::object_report::{self, Request},
};
use anyhow::Result;

#[test]
fn exact_validation_source_tracks_only_explicit_candidate_references_after_reopen() -> Result<()> {
    let f = fixture()?;
    let candidate_request = final_output(&f)?;
    let request = Request {
        project_id: "project-1".into(),
        attempt_id: candidate_request.target.attempt_id.clone(),
        request_id: candidate_request.check_request_id.clone(),
    };
    let original = object_report::get(&f.runtime, &request)?;
    assert!(original.source.candidate_reviews.is_empty());
    assert_eq!(original.source.target, candidate_request.target);
    let stage = attempts(&f, "head")?
        .into_iter()
        .find(|attempt| attempt.id == request.attempt_id)
        .unwrap()
        .fine
        .stage_id
        .unwrap();
    assert_eq!(original.source.stage_id, stage);
    let reviewed = candidate::prepare(&f.runtime, &candidate_request)?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: request.project_id.clone(),
            request_id: "newer-check".into(),
            target: candidate_request.target.clone(),
        },
    )?;
    candidate::prepare(
        &f.runtime,
        &candidate::Request {
            request_id: "other-review".into(),
            check_request_id: "newer-check".into(),
            ..candidate_request
        },
    )?;
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let before = serde_json::to_value(object_tasks::snapshot(&runtime, "project-1")?)?;
    let queue = serde_json::to_value(object_tasks::queue(&runtime, "project-1")?)?;
    let evidence = object_report::get(&runtime, &request)?;
    assert_eq!(evidence.report, original.report);
    assert_eq!(evidence.source.stage_id, stage);
    assert_eq!(evidence.source.candidate_reviews.len(), 1);
    let reference = &evidence.source.candidate_reviews[0];
    assert_eq!(reference.request_id, reviewed.request.request_id);
    assert_eq!(reference.source_digest, reviewed.source_digest);
    assert_eq!(reference.output_digest, reviewed.output_digest);
    assert_eq!(
        serde_json::to_value(object_tasks::snapshot(&runtime, "project-1")?)?,
        before
    );
    assert_eq!(
        serde_json::to_value(object_tasks::queue(&runtime, "project-1")?)?,
        queue
    );
    assert_eq!(
        checks::list(&runtime, "project-1", &request.attempt_id)?.len(),
        2
    );
    for invalid in [
        Request {
            project_id: "other".into(),
            ..request.clone()
        },
        Request {
            request_id: "missing".into(),
            ..request.clone()
        },
        Request {
            attempt_id: "missing".into(),
            ..request.clone()
        },
    ] {
        assert!(object_report::get(&runtime, &invalid).is_err());
    }
    Ok(())
}
