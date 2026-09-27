use super::super::object_candidate_review::final_output_with;
use super::*;
use crate::object_run_recovery::resume::rework::image::{Feedback, Region};

#[path = "object_rework_image_delivery_tests.rs"]
mod delivery;

fn image_request(f: &Fixture, valid_png: bool) -> Result<resume::Request> {
    let review = final_output_with(f, |root| {
        if valid_png {
            ::image::RgbaImage::from_pixel(20, 10, ::image::Rgba([255, 0, 0, 255]))
                .save(root.join("preview.png"))?;
        } else {
            std::fs::write(root.join("preview.png"), b"not a PNG")?;
        }
        Ok(())
    })?;
    let mut request = authorize_review(f, review)?;
    let original = attempts(f, "head")?
        .into_iter()
        .find(|a| a.fine.id == "fine-b")
        .unwrap();
    request.rework.as_mut().unwrap().image = Some(Feedback {
        path: "preview.png".into(),
        sha256: original.output.unwrap()["preview.png"].clone(),
        width: 20,
        height: 10,
        regions: vec![Region {
            x: 0.1,
            y: 0.2,
            width: 0.5,
            height: 0.5,
            prompt: "Brighten this area".into(),
        }],
    });
    Ok(request)
}

#[test]
fn frozen_image_rework_survives_pending_restart_and_preserves_publication_evidence() -> Result<()> {
    let f = fixture()?;
    let request = image_request(&f, true)?;
    assert!(resume::execute_with(
        &f.runtime,
        &request,
        &AtomicBool::new(false),
        || anyhow::bail!("restart")
    )
    .is_err());
    let mut changed = request.clone();
    changed
        .rework
        .as_mut()
        .unwrap()
        .image
        .as_mut()
        .unwrap()
        .regions[0]
        .x = 0.2;
    assert_eq!(
        resume::execute(&f.runtime, &changed, &AtomicBool::new(false))
            .err()
            .unwrap()
            .to_string(),
        "OBJECT_RECOVERY_RESUME_REQUEST_CONFLICT"
    );
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let (receipt, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    let image = request.rework.as_ref().unwrap().image.as_ref().unwrap();
    assert_eq!(lease.record().input.get(&image.path), Some(&image.sha256));
    assert!(lease
        .record()
        .fine
        .prompt
        .contains(&serde_json::to_string(image)?));
    object_attempt::finish(
        &runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    let store = runtime.store();
    let feedback =
        resume::rework::feedback(&store.lock().unwrap().connection, "project-1", "head")?;
    assert_eq!(feedback.len(), 1);
    assert_eq!(feedback[0].image.as_ref(), Some(image));
    let (replay, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
    assert_eq!(receipt, replay);
    assert!(lease.is_none());
    Ok(())
}

#[test]
fn image_source_and_geometry_reject_before_journaling() -> Result<()> {
    let f = fixture()?;
    let request = image_request(&f, true)?;
    for reason in ["hash", "path", "bounds", "count"] {
        let mut invalid = request.clone();
        let image = invalid.rework.as_mut().unwrap().image.as_mut().unwrap();
        match reason {
            "hash" => image.sha256 = "f".repeat(64),
            "path" => image.path = "other.png".into(),
            "bounds" => image.regions[0].width = 1.0,
            _ => image.regions.clear(),
        }
        assert!(resume::execute(&f.runtime, &invalid, &AtomicBool::new(false)).is_err());
        assert!(recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .resume
            .is_some_and(|op| op.request.request_id == "advance-a"));
    }
    assert!(
        resume::execute(&f.runtime, &request, &AtomicBool::new(false))?
            .1
            .is_some()
    );
    Ok(())
}

#[test]
fn wrong_dimensions_or_invalid_png_block_without_launch_and_replay_terminal_receipt() -> Result<()>
{
    for valid_png in [true, false] {
        let f = fixture()?;
        let mut request = image_request(&f, valid_png)?;
        if valid_png {
            request
                .rework
                .as_mut()
                .unwrap()
                .image
                .as_mut()
                .unwrap()
                .width = 21;
        }
        let (receipt, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
        assert!(lease.is_none());
        let Some(resume::Outcome::Blocked { report }) = &receipt.result else {
            panic!("must block")
        };
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.starts_with("OBJECT_REWORK_IMAGE_")));
        let (replay, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
        assert_eq!(receipt, replay);
        assert!(lease.is_none());
    }
    Ok(())
}
