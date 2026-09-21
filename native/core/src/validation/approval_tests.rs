use super::{
    comparison, confirmation, evidence, judgment,
    model::{Baseline, Run},
    repository,
    test_support::Fixture,
};
use anyhow::Result;
use std::fs;

fn confirm(fixture: &mut Fixture, run: &Run, ids: &[String], request: &str) -> Result<Run> {
    confirmation::confirm(
        &fixture.files,
        &mut fixture.store,
        &run.id,
        &run.snapshot_id,
        ids,
        request,
        "ui",
    )
}

#[test]
fn partial_approval_and_automatic_pass_never_advance_the_human_baseline() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let flow = fixture.flow()?;
    let original = fixture.visual(&flow)?;
    let first = vec![original.evidence[0].id.clone()];
    let partial = confirm(&mut fixture, &original, &first, "first")?;
    assert!(!judgment::green(&fixture.store, &partial)?);
    assert!(repository::baseline(&fixture.store, &flow)?.is_none());
    let replay = confirm(&mut fixture, &original, &first, "first")?;
    assert_eq!(replay.confirmations.len(), 1);
    let second = vec![original.evidence[1].id.clone()];
    assert!(confirm(&mut fixture, &original, &second, "first").is_err());
    let approved = confirm(&mut fixture, &original, &second, "second")?;
    assert!(judgment::green(&fixture.store, &approved)?);
    let baseline = repository::baseline(&fixture.store, &flow)?.unwrap();
    assert_eq!(baseline.run_id, original.id);

    fs::write(
        fixture.project.join("game.gd"),
        "extends Node\nvar value = 2\n",
    )?;
    let mut next = fixture.visual(&flow)?;
    assert_ne!(next.snapshot_id, original.snapshot_id);
    comparison::compare(&fixture.files, &fixture.store, &mut next)?;
    fixture.save(&next)?;
    assert_eq!(next.verdict, "autoPassed");
    assert!(judgment::green(&fixture.store, &next)?);
    assert_eq!(
        repository::baseline(&fixture.store, &flow)?.unwrap().id,
        baseline.id
    );
    assert_eq!(
        fixture.store.list::<Baseline>("validationBaseline")?.len(),
        1
    );

    // A small but substantial local change must not hide in a high whole-image score.
    let mut changed = fixture.visual(&flow)?;
    let image = &mut changed.evidence[0];
    let path = repository::run_dir(&fixture.files, &changed.id)?.join(&image.file);
    let mut pixels = image::open(&path)?.to_rgb8();
    for y in 0..16 {
        for x in 0..16 {
            pixels.put_pixel(x, y, image::Rgb([255, 255, 255]));
        }
    }
    pixels.save(&path)?;
    image.sha256 = crate::files::file_hash(&path)?.unwrap();
    comparison::compare(&fixture.files, &fixture.store, &mut changed)?;
    fixture.save(&changed)?;
    assert_eq!(changed.verdict, "needsReview");
    assert!(!judgment::green(&fixture.store, &changed)?);
    let ids = changed
        .evidence
        .iter()
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    confirm(&mut fixture, &changed, &ids, "approve-change")?;
    let latest = repository::baseline(&fixture.store, &flow)?.unwrap();
    assert_eq!(latest.run_id, changed.id);
    assert_eq!(latest.previous_id, Some(baseline.id));
    // A new approval never rewrites the previous comparison's chosen baseline.
    let recorded: Run = repository::get(&fixture.store, "validationRun", &next.id)?;
    assert_eq!(recorded.judgments, next.judgments);
    Ok(())
}

#[test]
fn incomplete_or_changed_evidence_cannot_be_confirmed() -> Result<()> {
    for failure in [
        "missing-point",
        "missing-file",
        "changed-file",
        "resolution",
        "steps",
        "video",
        "error",
        "cancelled",
    ] {
        let mut fixture = Fixture::new()?;
        let flow = fixture.flow()?;
        let mut run = fixture.visual(&flow)?;
        let path = repository::run_dir(&fixture.files, &run.id)?.join(&run.evidence[0].file);
        match failure {
            "missing-point" => {
                run.evidence.pop();
            }
            "missing-file" => fs::remove_file(&path)?,
            "changed-file" => fs::write(&path, b"truncated")?,
            "resolution" => {
                image::RgbImage::new(32, 32).save(&path)?;
                run.evidence[0].sha256 = crate::files::file_hash(&path)?.unwrap();
            }
            "steps" => run.completed_steps -= 1,
            "video" => run.flow.as_mut().unwrap().definition.video = true,
            "error" => run.error = Some("Capture was truncated".into()),
            "cancelled" => run.status = "cancelled".into(),
            _ => unreachable!(),
        }
        fixture.save(&run)?;
        let ids = run
            .evidence
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>();
        assert!(
            evidence::validate(&fixture.files, &run).is_err(),
            "{failure}"
        );
        assert!(
            confirm(&mut fixture, &run, &ids, "invalid").is_err(),
            "{failure}"
        );
        assert!(fixture
            .store
            .list::<Baseline>("validationBaseline")?
            .is_empty());
    }
    Ok(())
}

#[test]
fn confirmation_is_bound_to_user_run_snapshot_and_evidence() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let flow = fixture.flow()?;
    let run = fixture.visual(&flow)?;
    let ids = run
        .evidence
        .iter()
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    for (source, snapshot) in [
        ("mcp", run.snapshot_id.as_str()),
        ("automatic", run.snapshot_id.as_str()),
        ("ui", "stale-snapshot"),
    ] {
        assert!(confirmation::confirm(
            &fixture.files,
            &mut fixture.store,
            &run.id,
            snapshot,
            &ids,
            "rejected",
            source
        )
        .is_err());
    }
    assert!(confirm(&mut fixture, &run, &[repository::id()], "foreign-evidence").is_err());
    let mut code = fixture.run(None)?;
    fixture.passing_code(&mut code)?;
    assert!(confirm(&mut fixture, &code, &[], "code-approval").is_err());
    assert!(repository::baseline(&fixture.store, &flow)?.is_none());
    Ok(())
}

#[test]
fn restart_preserves_partial_evidence_and_does_not_resume_an_incomplete_run() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let flow = fixture.flow()?;
    let mut run = fixture.visual(&flow)?;
    run.status = "running".into();
    run.completed_steps = 1;
    fixture.save(&run)?;
    repository::recover(&fixture.store)?;
    let interrupted: Run = repository::get(&fixture.store, "validationRun", &run.id)?;
    assert_eq!(interrupted.status, "interrupted");
    assert_eq!(
        repository::digest(&interrupted.evidence)?,
        repository::digest(&run.evidence)?
    );
    assert_eq!(interrupted.snapshot, run.snapshot);
    assert!(interrupted.error.is_some());
    let media = evidence::media_path(&fixture.files, &interrupted, &run.evidence[0].id)?;
    assert!(media.is_file());
    repository::recover(&fixture.store)?;
    let again: Run = repository::get(&fixture.store, "validationRun", &run.id)?;
    assert_eq!(again.finished_at, interrupted.finished_at);
    Ok(())
}
