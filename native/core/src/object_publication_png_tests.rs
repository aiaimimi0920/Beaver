use super::super::super::{delivery, frames};
use super::*;

#[path = "object_publication_png_recovery_tests.rs"]
mod recovery_tests;

fn ready_png() -> Result<(Fixture, Request, deferred::Request)> {
    let f = fixture()?;
    let (original, mut feedback) = image_feedback(&f)?;
    feedback
        .image
        .as_mut()
        .unwrap()
        .regions
        .push(resume::rework::image::Region {
            x: 0.7,
            y: 0.1,
            width: 0.2,
            height: 0.2,
            prompt: "Remove old badge".into(),
        });
    deferred::create(&f.runtime, &feedback)?;
    let rework = rework(&f, &feedback, "change-png", false)?;
    let (_, lease) = resume::execute(&f.runtime, &rework, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 255, 0, 255]))
        .save(workspace(&f, lease.record())?.join("preview.png"))?;
    let mut publish = finish_candidate(&f, lease, "final-png")?;
    assert_ne!(original.target.attempt_id, publish.target.attempt_id);
    frames::approve(&f, &mut publish)?;
    Ok((f, publish, feedback))
}

#[test]
fn publication_png_requires_complete_final_mapping_and_rejects_old_output_without_mutation(
) -> Result<()> {
    let (f, publish, source) = ready_png()?;
    let preview = publication::preview(&f.runtime, &review(&publish))?;
    let item = preview.feedback.iter().find(|v| v.image.is_some()).unwrap();
    assert_eq!(item.image, source.image);
    assert_eq!(item.attempt_id, source.review.target.attempt_id);
    assert!(item.preview_frame.is_none());
    assert_eq!(
        item.relocation_requirement.as_ref().unwrap().region_count,
        2
    );
    let mut archived = source.clone();
    let old = frames::attach_named(&f, &mut archived, "old-png")?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    for mutation in 0..8 {
        let mut bad = publish.clone();
        match mutation {
            0 => bad
                .feedback
                .iter_mut()
                .for_each(|d| d.final_relocation = None),
            1 => mapping(&mut bad).confirmed = false,
            2 => mapping(&mut bad).source_digest = "f".repeat(64),
            3 => {
                mapping(&mut bad).regions.pop();
            }
            4 => mapping(&mut bad).regions.reverse(),
            5 => {
                mapping(&mut bad).regions[1] = relocation::Region::Absent {
                    source_region: 1,
                    note: " ".into(),
                }
            }
            6 => {
                mapping(&mut bad).target_frame = followup::frames::Reference {
                    run_id: "old-png".into(),
                    frame_id: old["id"].as_str().unwrap().into(),
                }
            }
            _ => mapping(&mut bad).target_frame.run_id = "foreign".into(),
        }
        assert!(
            publication::publish(&f.runtime, &bad).is_err(),
            "mutation {mutation}"
        );
        assert_unchanged(&f, &publish, &before)?;
    }
    let op = publication::publish(&f.runtime, &publish)?;
    assert_eq!(op.state, State::Published);
    assert_eq!(op.preview.feedback, preview.feedback);
    assert_eq!(op.request.feedback, publish.feedback);
    Ok(())
}

fn save_feedback(f: &Fixture, feedback: &deferred::Request) -> Result<()> {
    let key = crate::framework_checks::digest(&(&feedback.project_id, &feedback.request_id))?;
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_publication_deferred", &key, feedback)?;
    Ok(())
}

fn assert_source_blocked(f: &Fixture, publish: &Request) -> Result<()> {
    assert!(publication::preview(&f.runtime, &review(publish))
        .unwrap_err()
        .to_string()
        .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
    assert!(publication::publish(&f.runtime, publish)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
    Ok(())
}

#[test]
fn publication_png_revalidates_missing_corrupt_dimensions_and_identity_before_journal() -> Result<()>
{
    let (f, publish, source) = ready_png()?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let blob = f
        .runtime
        .files()
        .blob(&source.image.as_ref().unwrap().sha256)?;
    let bytes = std::fs::read(&blob)?;
    let backup = blob.with_extension("png-backup");
    std::fs::rename(&blob, &backup)?;
    assert_source_blocked(&f, &publish)?;
    assert_unchanged(&f, &publish, &before)?;
    std::fs::rename(&backup, &blob)?;
    std::fs::write(&blob, b"broken PNG")?;
    assert_source_blocked(&f, &publish)?;
    assert_unchanged(&f, &publish, &before)?;
    std::fs::write(&blob, bytes)?;
    for mutation in 0..5 {
        let mut broken = source.clone();
        match mutation {
            0 => broken.image.as_mut().unwrap().width = 21,
            1 => broken.image.as_mut().unwrap().height = 11,
            2 => broken.image.as_mut().unwrap().path = "other.png".into(),
            3 => broken.image.as_mut().unwrap().sha256 = "f".repeat(64),
            _ => broken.review.target.attempt_id = "foreign".into(),
        }
        save_feedback(&f, &broken)?;
        assert_source_blocked(&f, &publish)?;
        assert_unchanged(&f, &publish, &before)?;
    }
    save_feedback(&f, &source)?;
    let f = reopen(f)?;
    assert_eq!(
        publication::preview(&f.runtime, &review(&publish))?.digest,
        publish.preview_digest
    );
    assert_eq!(
        publication::publish(&f.runtime, &publish)?.state,
        State::Published
    );
    Ok(())
}

#[test]
fn publication_png_deferred_followup_preserves_original_image_and_requires_new_authorization(
) -> Result<()> {
    let (f, publish, source) = ready_png()?;
    let original = publication::publish(&f.runtime, &publish)?;
    let receipt = followup::list(&f.runtime, "project-1", "publish")?.remove(0);
    let f = reopen(f)?;
    let lease = delivery::start_followup(&f, &receipt)?;
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 0, 255, 255]))
        .save(workspace(&f, lease.record())?.join("preview.png"))?;
    let mut next = finish_candidate(&f, lease, "followup-png")?;
    next.request_id = "publish-followup-png".into();
    let preview = publication::preview(&f.runtime, &review(&next))?;
    assert_eq!(preview.feedback.len(), 1);
    let inherited = &preview.feedback[0];
    assert_eq!(inherited.image, source.image);
    assert_eq!(inherited.attempt_id, source.review.target.attempt_id);
    assert!(inherited.preview_frame.is_none());
    assert!(inherited.relocation.is_none());
    assert!(inherited.later.is_none());
    let origin = inherited.origin.as_ref().unwrap();
    assert_eq!(origin.run_id, original.request.target.run_id);
    assert_eq!(origin.publication_request_id, original.request.request_id);
    assert_eq!(origin.version_id, original.version_id);
    assert!(!origin.published_frame);
    let old = original
        .preview
        .feedback
        .iter()
        .find(|v| v.image.is_some())
        .unwrap();
    assert_ne!(inherited.relocation_requirement, old.relocation_requirement);
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert!(publication::publish(&f.runtime, &next).is_err());
    frames::approve(&f, &mut next)?;
    let mut stale = next.clone();
    mapping(&mut stale).source_digest = old
        .relocation_requirement
        .as_ref()
        .unwrap()
        .source_digest
        .clone();
    assert!(publication::publish(&f.runtime, &stale).is_err());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert_eq!(f.object("hero")?.versions.len(), 1);
    let completed = publication::publish(&f.runtime, &next)?;
    assert_eq!(completed.state, State::Published);
    assert_eq!(completed.preview.feedback, preview.feedback);
    assert_eq!(f.object("hero")?.versions.len(), 2);
    Ok(())
}
