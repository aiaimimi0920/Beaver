use super::fixture::{frame, png, Fixture};
use anyhow::Result;
use beaver_core::{
    asset_reference,
    asset_task::{Annotation, Pick, Reference},
};
use std::fs;

#[test]
fn png_integrity_and_frame_dimensions_are_checked_before_acceptance() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let mut wrong_size = frame();
    wrong_size.width += 1;
    assert!(asset_reference::capture(
        &fixture.store,
        &fixture.root,
        &fixture.id,
        wrong_size,
        &png()?
    )
    .is_err());
    assert!(asset_reference::capture(
        &fixture.store,
        &fixture.root,
        &fixture.id,
        frame(),
        b"not an image"
    )
    .is_err());
    let input = fixture.input("corrupt", "now")?;
    let reference = asset_reference::get(&fixture.store, &fixture.id, &input.reference_id)?;
    fs::write(&reference.image_path, b"replaced image")?;
    assert!(fixture.submit(input).is_err());
    assert!(fixture.state()?.feedback.is_empty());
    let accepted = fixture.input("accepted", "now")?;
    let receipt = fixture.submit(accepted.clone())?;
    fs::write(&receipt.reference.image_path, b"later corruption")?;
    assert_eq!(fixture.submit(accepted)?.id, receipt.id);
    assert!(asset_reference::read(&receipt.reference).is_err());
    Ok(())
}

#[test]
fn annotations_render_normalized_regions_without_mutating_the_frozen_original() -> Result<()> {
    let fixture = Fixture::new()?;
    let reference = fixture.reference()?;
    let before = asset_reference::read(&reference)?;
    let annotations = vec![Annotation {
        kind: "box".into(),
        points: vec![[0.25, 0.25], [0.75, 0.75]],
    }];
    let annotated = asset_reference::annotated(&reference, &annotations)?;
    let pixels = image::load_from_memory(&annotated)?.to_rgba8();
    assert_eq!(pixels.get_pixel(15, 11).0, [255, 195, 50, 255]);
    assert_eq!(pixels.get_pixel(32, 24).0, [255, 255, 255, 255]);
    assert_eq!(pixels.get_pixel(0, 0).0, [255, 255, 255, 255]);
    assert_eq!(asset_reference::read(&reference)?, before);
    for invalid in [
        Annotation {
            kind: "box".into(),
            points: vec![[0.0, 0.0], [1.1, 1.0]],
        },
        Annotation {
            kind: "arrow".into(),
            points: vec![[f64::NAN, 0.0], [1.0, 1.0]],
        },
        Annotation {
            kind: "brush".into(),
            points: vec![[0.0, 0.0]; 4097],
        },
    ] {
        assert!(asset_reference::annotated(&reference, &[invalid]).is_err());
    }
    Ok(())
}

#[test]
fn picking_is_task_and_frame_bound_immutable_after_submit_and_stale_after_geometry_change(
) -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("picked", "now")?;
    let mut pick = Pick {
        frame_id: "wrong".into(),
        object_id: "object-uuid".into(),
        object_name: "Face".into(),
        instance_id: "original".into(),
        local: [0.1, 0.2, 0.3],
        world: [0.1, 0.2, 0.3],
        normal: [0.0, 1.0, 0.0],
        face: 3,
        vertices: 120,
        polygons: 80,
        point: [0.5, 0.5],
    };
    assert!(asset_reference::set_pick(
        &fixture.store,
        &fixture.id,
        &input.reference_id,
        pick.clone()
    )
    .is_err());
    assert!(asset_reference::get(&fixture.store, "another-task", &input.reference_id).is_err());
    pick.frame_id = "frame".into();
    asset_reference::set_pick(
        &fixture.store,
        &fixture.id,
        &input.reference_id,
        pick.clone(),
    )?;
    fixture.submit(input.clone())?;
    assert!(
        asset_reference::set_pick(&fixture.store, &fixture.id, &input.reference_id, pick).is_err()
    );
    let mut camera_only = frame();
    camera_only.view_revision += 1;
    asset_reference::same_scene(&frame(), &camera_only)?;
    for field in ["session", "generation", "geometry"] {
        let mut changed = frame();
        match field {
            "session" => changed.session_id = "new".into(),
            "generation" => changed.generation = "reload".into(),
            _ => changed.scene_revision += 1,
        }
        assert!(asset_reference::same_scene(&frame(), &changed).is_err());
    }
    Ok(())
}

#[test]
fn unused_frames_are_bounded_while_accepted_references_remain_readable() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("pinned", "now")?;
    let pinned = fixture.submit(input)?.reference;
    let old_unused = fixture.reference()?;
    for index in 1..=12 {
        let mut next = frame();
        next.id = format!("frame-{index}");
        next.captured_at += index;
        asset_reference::capture(&fixture.store, &fixture.root, &fixture.id, next, &png()?)?;
    }
    let references = fixture.store.list::<Reference>("asset-reference")?;
    assert_eq!(references.iter().filter(|r| !r.used).count(), 8);
    assert_eq!(references.iter().filter(|r| r.used).count(), 1);
    assert!(asset_reference::get(&fixture.store, &fixture.id, &old_unused.id).is_err());
    assert!(!std::path::Path::new(&old_unused.image_path).exists());
    assert_eq!(asset_reference::read(&pinned)?, png()?);
    Ok(())
}
