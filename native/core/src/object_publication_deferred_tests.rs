use super::publication_fixture::*;
use crate::{
    object_framework::{Baseline, Identity},
    object_tasks,
};
use anyhow::Result;
use publication::{
    deferred::{self, Later},
    followup, FeedbackDecision, Resolution,
};

fn input(publish: &Request) -> deferred::Request {
    deferred::Request {
        project_id: publish.project_id.clone(),
        request_id: "later".into(),
        review: review(publish),
        feedback: "Improve contrast after this release".into(),
        later: Later {
            title: "Improve contrast".into(),
            acceptance: "Readable labels".into(),
        },
        image: None,
        preview_frame: None,
        relocation: None,
    }
}

#[test]
fn publication_deferred_requires_fresh_approval_and_commits_once_with_acceptance() -> Result<()> {
    let f = fixture()?;
    let mut publish = ready(&f)?;
    let request = input(&publish);
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let queue = object_tasks::queue(&f.runtime, "project-1")?;
    assert_eq!(deferred::create(&f.runtime, &request)?, request);
    assert_eq!(deferred::create(&f.runtime, &request)?, request);
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert_eq!(object_tasks::queue(&f.runtime, "project-1")?, queue);
    assert!(publication::publish(&f.runtime, &publish)
        .unwrap_err()
        .to_string()
        .contains("PREVIEW_CHANGED"));
    let preview = publication::preview(&f.runtime, &request.review)?;
    assert_eq!(preview.feedback.len(), 1);
    publish.preview_digest = preview.digest;
    publish.feedback = vec![FeedbackDecision {
        request_id: preview.feedback[0].request_id.clone(),
        resolution: Resolution::Resolved,
        note: "Do later".into(),
    }];
    assert!(publication::publish(&f.runtime, &publish)
        .unwrap_err()
        .to_string()
        .contains("ROUTE_MISMATCH"));
    publish.feedback[0].resolution = Resolution::Deferred;
    let pending = fail_at(&f, &publish, "beforeCommit")?;
    assert_eq!(pending.state, State::Applying);
    assert!(followup::list(&f.runtime, "project-1", "publish")?.is_empty());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    let mut late = request.clone();
    late.request_id = "racing-feedback".into();
    assert!(deferred::create(&f.runtime, &late).is_err());
    let op = publication::publish(&f.runtime, &publish)?;
    assert_eq!(op.state, State::Published);
    let receipts = followup::list(&f.runtime, "project-1", "publish")?;
    assert_eq!(receipts.len(), 1);
    let medium = task_record(&f, &receipts[0].medium_task_id)?;
    assert_eq!(medium.status, "planned");
    assert!(
        matches!(medium.identity, Identity::Medium { baseline: Baseline::PinnedVersion { selected_version_id }, .. } if selected_version_id == op.version_id)
    );
    assert_eq!(
        object_tasks::snapshot(&f.runtime, "project-1")?.tasks.len(),
        before.tasks.len() + 2
    );
    assert_eq!(publication::publish(&f.runtime, &publish)?, op);
    assert_eq!(deferred::create(&f.runtime, &request)?, request);
    assert!(deferred::create(&f.runtime, &late).is_err());
    let mut changed = request.clone();
    changed.later.title = "Changed intent".into();
    assert!(deferred::create(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_ID_CONFLICT"));
    let crate::object_catalog_test_fixture::Fixture { runtime, temp } = f;
    drop(runtime);
    let reopened =
        crate::project_storage::ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(followup::list(&reopened, "project-1", "publish")?, receipts);
    assert_eq!(deferred::create(&reopened, &request)?, request);
    Ok(())
}

#[path = "object_publication_image_delivery_tests.rs"]
mod delivery;
#[path = "object_deferred_frame_tests.rs"]
mod frames;
#[path = "object_feedback_relocation_tests.rs"]
mod relocation;
#[path = "object_rework_frame_tests.rs"]
mod rework_frames;

fn image_feedback(
    f: &crate::object_catalog_test_fixture::Fixture,
) -> Result<(Request, deferred::Request)> {
    use crate::object_run_recovery::resume::rework::image::{Feedback, Region};
    let candidate = super::object_candidate_review::final_output_with(f, |root| {
        image::RgbaImage::from_pixel(20, 10, image::Rgba([255, 0, 0, 255]))
            .save(root.join("preview.png"))?;
        Ok(())
    })?;
    let publish = request(f, &candidate)?;
    let mut deferred = input(&publish);
    let preview = publication::preview(&f.runtime, &deferred.review)?;
    deferred.image = Some(Feedback {
        path: "preview.png".into(),
        sha256: preview
            .paths
            .iter()
            .find(|p| p.path == "preview.png")
            .unwrap()
            .after
            .clone()
            .unwrap(),
        width: 20,
        height: 10,
        regions: vec![Region {
            x: 0.1,
            y: 0.2,
            width: 0.5,
            height: 0.5,
            prompt: "Brighten label".into(),
        }],
    });
    Ok((publish, deferred))
}

#[test]
fn publication_deferred_validates_png_and_retains_historical_regions() -> Result<()> {
    let f = fixture()?;
    let (mut publish, deferred) = image_feedback(&f)?;
    let mut bad = deferred.clone();
    bad.image.as_mut().unwrap().width = 21;
    assert!(deferred::create(&f.runtime, &bad)
        .unwrap_err()
        .to_string()
        .contains("DIMENSIONS_MISMATCH"));
    assert!(publication::preview(&f.runtime, &deferred.review)?
        .feedback
        .is_empty());
    deferred::create(&f.runtime, &deferred)?;
    let preview = publication::preview(&f.runtime, &deferred.review)?;
    assert_eq!(preview.feedback[0].image, deferred.image);
    publish.preview_digest = preview.digest;
    publish.feedback = vec![FeedbackDecision {
        request_id: preview.feedback[0].request_id.clone(),
        resolution: Resolution::Deferred,
        note: "Accept current appearance".into(),
    }];
    publication::publish(&f.runtime, &publish)?;
    let receipts = followup::list(&f.runtime, "project-1", "publish")?;
    let fine = task_record(&f, &receipts[0].fine_task_id)?;
    assert!(fine.prompt.contains("re-localize"));
    assert!(fine.prompt.contains("Brighten label"));
    assert!(fine.prompt.contains(&deferred.review.target.attempt_id));
    Ok(())
}
