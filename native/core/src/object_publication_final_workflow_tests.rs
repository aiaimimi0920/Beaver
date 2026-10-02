use super::*;
use crate::{object_attempt_checks as checks, object_run_recovery::candidate};
use std::sync::atomic::AtomicBool;

#[test]
fn publication_final_relocation_published_numbered_followup_requires_complete_new_output_mapping(
) -> Result<()> {
    let f = fixture()?;
    let original = publication::publish(&f.runtime, &ready(&f)?)?;
    let source = archive(&f, &original)?;
    let feedback = attach(&original, &source);
    let receipt = followup::create(&f.runtime, &feedback)?;
    let lease = start_followup(&f, &receipt)?;
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 0, 255, 255]))
        .save(workspace(&f, lease.record())?.join("preview.png"))?;
    let target = crate::object_attempt_view::Target::from_record(lease.record());
    object_attempt::finish(
        &f.runtime,
        lease,
        object_attempt::State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-followup".into(),
            target: target.clone(),
        },
    )?;
    let mut publish = super::super::super::publication_fixture::request(
        &f,
        &candidate::Request {
            project_id: "project-1".into(),
            request_id: "review-followup".into(),
            check_request_id: "check-followup".into(),
            target,
        },
    )?;
    publish.request_id = "publish-followup".into();
    // The frame fixture is shared by deferred and published feedback workflows.
    super::super::super::object_publication_deferred::approve_final_frames(&f, &mut publish)?;
    let preview = publication::preview(
        &f.runtime,
        &super::super::super::publication_fixture::review(&publish),
    )?;
    assert_eq!(preview.feedback.len(), 1);
    let inherited = &preview.feedback[0];
    assert_eq!(inherited.preview_frame, feedback.preview_frame);
    let origin = inherited.origin.as_ref().unwrap();
    assert_eq!(origin.publication_request_id, original.request.request_id);
    assert_eq!(origin.version_id, original.version_id);
    assert_eq!(origin.run_id, original.request.target.run_id);
    assert!(origin.published_frame);
    assert_eq!(
        inherited
            .relocation_requirement
            .as_ref()
            .unwrap()
            .region_count,
        2
    );
    assert!(inherited.relocation.is_none());
    assert!(inherited.later.is_none());
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    for mutation in 0..4 {
        let mut bad = publish.clone();
        let mapping = bad.feedback[0].final_relocation.as_mut().unwrap();
        match mutation {
            0 => {
                mapping.regions.pop();
            }
            1 => mapping.regions[1] = mapping.regions[0].clone(),
            2 => mapping.target_frame = feedback.preview_frame.clone().unwrap(),
            _ => bad.feedback[0].final_relocation = None,
        }
        assert!(
            publication::publish(&f.runtime, &bad).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
        assert_eq!(f.count("object_publication")?, 1);
        assert_eq!(f.object("hero")?.versions.len(), 1);
    }
    let completed = publication::publish(&f.runtime, &publish)?;
    assert_eq!(completed.state, State::Published);
    assert_eq!(completed.preview.feedback, preview.feedback);
    assert_eq!(publication::publish(&f.runtime, &publish)?, completed);
    assert_eq!(f.object("hero")?.versions.len(), 2);
    Ok(())
}
