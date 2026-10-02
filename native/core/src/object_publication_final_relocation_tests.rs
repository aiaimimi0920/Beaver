use super::*;

#[path = "object_publication_png_tests.rs"]
mod png_tests;
#[path = "object_publication_final_recovery_tests.rs"]
mod recovery_tests;

#[test]
fn publication_final_relocation_deferred_followup_inherits_original_authority_and_publishes(
) -> Result<()> {
    let (f, publish, _) = ready_final()?;
    let original = publication::publish(&f.runtime, &publish)?;
    let receipt = followup::list(&f.runtime, "project-1", "publish")?.remove(0);
    let f = reopen(f)?;
    let lease = super::super::delivery::start_followup(&f, &receipt)?;
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 0, 255, 255]))
        .save(workspace(&f, lease.record())?.join("preview.png"))?;
    let mut next = finish_candidate(&f, lease, "deferred-final")?;
    next.request_id = "publish-followup".into();
    let preview = publication::preview(&f.runtime, &review(&next))?;
    assert_eq!(preview.feedback.len(), 1);
    let inherited = &preview.feedback[0];
    let old = original
        .preview
        .feedback
        .iter()
        .find(|item| item.later.is_some())
        .unwrap();
    assert_eq!(inherited.attempt_id, old.attempt_id);
    assert_eq!(inherited.preview_frame, old.preview_frame);
    assert_eq!(inherited.relocation, old.relocation);
    let origin = inherited.origin.as_ref().unwrap();
    assert_eq!(origin.publication_request_id, original.request.request_id);
    assert_eq!(origin.version_id, original.version_id);
    assert_eq!(origin.run_id, original.request.target.run_id);
    assert!(!origin.published_frame);
    assert!(inherited.later.is_none());
    assert_ne!(inherited.relocation_requirement, old.relocation_requirement);
    assert!(publication::publish(&f.runtime, &next).is_err());
    approve(&f, &mut next)?;
    let completed = publication::publish(&f.runtime, &next)?;
    assert_eq!(completed.state, State::Published);
    assert_eq!(completed.preview.feedback, preview.feedback);
    assert_eq!(f.object("hero")?.versions.len(), 2);
    Ok(())
}

fn ready_final() -> Result<(Fixture, Request, Value)> {
    let f = fixture()?;
    let (mut publish, feedback, _, _) = historical(&f)?;
    deferred::create(&f.runtime, &feedback)?;
    approve(&f, &mut publish)?;
    let target = super::super::frames::final_frame(&f, &publish)?;
    for decision in &mut publish.feedback {
        if let Some(confirmation) = &mut decision.final_relocation {
            confirmation.target_frame = followup::frames::Reference {
                run_id: "final-frame".into(),
                frame_id: target["id"].as_str().unwrap().into(),
            };
        }
    }
    Ok((f, publish, target))
}

fn mapping(request: &mut Request) -> &mut publication::relocation::Confirmation {
    request
        .feedback
        .iter_mut()
        .find_map(|d| d.final_relocation.as_mut())
        .unwrap()
}

fn assert_unchanged(
    f: &Fixture,
    request: &Request,
    before: &crate::object_task_types::Snapshot,
) -> Result<()> {
    assert_eq!(f.count("object_publication")?, 0);
    assert_eq!(&object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert_eq!(
        task_record(f, &request.target.task_id)?.status,
        "awaitingAcceptance"
    );
    assert!(f.object("hero")?.versions.is_empty());
    assert!(!f.temp.path().join("preview.png").exists());
    Ok(())
}

#[test]
fn publication_final_relocation_requires_new_owner_confirmation_without_mutation() -> Result<()> {
    let (f, publish, _) = ready_final()?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    for mutation in 0..8 {
        let mut bad = publish.clone();
        match mutation {
            0 => {
                for d in &mut bad.feedback {
                    d.final_relocation = None;
                }
            }
            1 => mapping(&mut bad).confirmed = false,
            2 => mapping(&mut bad).source_digest = "f".repeat(64),
            3 => mapping(&mut bad).regions.clear(),
            4 => {
                mapping(&mut bad).regions[0] = relocation::Region::Matched {
                    source_region: 1,
                    target_region: 0,
                }
            }
            5 => {
                mapping(&mut bad).regions[0] = relocation::Region::Matched {
                    source_region: 0,
                    target_region: 1,
                }
            }
            6 => {
                mapping(&mut bad).regions[0] = relocation::Region::Absent {
                    source_region: 0,
                    note: " ".into(),
                }
            }
            _ => {
                mapping(&mut bad).regions[0] = relocation::Region::Absent {
                    source_region: 0,
                    note: "字".repeat(334),
                }
            }
        }
        assert!(
            publication::publish(&f.runtime, &bad).is_err(),
            "mutation {mutation}"
        );
        assert_unchanged(&f, &publish, &before)?;
    }
    // Historical relocation is retained, but cannot authorize this final output.
    assert!(publication::preview(&f.runtime, &review(&publish))?
        .feedback
        .iter()
        .any(|d| d.relocation.is_some()));
    let op = publication::publish(&f.runtime, &publish)?;
    assert_eq!(op.state, State::Published);
    assert_eq!(op.request.feedback, publish.feedback);
    assert_eq!(publication::publish(&f.runtime, &publish)?, op);
    let mut changed = publish.clone();
    mapping(&mut changed).regions[0] = relocation::Region::Absent {
        source_region: 0,
        note: "Different approval".into(),
    };
    assert!(publication::publish(&f.runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    Ok(())
}

#[test]
fn publication_final_relocation_rejects_foreign_input_stale_output_and_corrupt_targets(
) -> Result<()> {
    let (f, publish, target) = ready_final()?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let old = publication::preview(&f.runtime, &review(&publish))?
        .feedback
        .iter()
        .find_map(|d| d.relocation.as_ref())
        .unwrap()
        .source_frame
        .clone();
    let mut bad = publish.clone();
    mapping(&mut bad).target_frame = old;
    assert!(publication::publish(&f.runtime, &bad).is_err());
    for (path, value) in [
        ("/source/target/projectId", json!("foreign")),
        ("/source/target/runId", json!("foreign")),
        ("/source/target/attemptId", json!("foreign")),
        ("/source/target/checkpoint", json!("input")),
        ("/source/target/sha256", json!("f".repeat(64))),
        ("/source/sourceDigest", json!("f".repeat(64))),
        ("/frame/dataUrl", json!("data:image/png;base64,broken")),
    ] {
        let mut broken = target.clone();
        *broken.pointer_mut(path).unwrap() = value;
        broken.as_object_mut().unwrap().remove("id");
        broken["id"] = json!(crate::framework_checks::digest(&broken)?);
        f.runtime.store().lock().unwrap().put(
            "objectPreviewFrames",
            "final-frame",
            &json!([broken.clone()]),
        )?;
        let mut bad = publish.clone();
        mapping(&mut bad).target_frame.frame_id = broken["id"].as_str().unwrap().into();
        assert!(publication::publish(&f.runtime, &bad).is_err(), "{path}");
        assert_unchanged(&f, &publish, &before)?;
    }
    f.runtime.store().lock().unwrap().put(
        "objectPreviewFrames",
        "final-frame",
        &json!([target]),
    )?;
    mapping(&mut bad).target_frame.run_id = "foreign".into();
    assert!(publication::publish(&f.runtime, &bad).is_err());
    assert_unchanged(&f, &publish, &before)?;
    Ok(())
}
