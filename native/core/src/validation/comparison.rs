use super::{
    model::{Baseline, Run},
    repository,
};
use crate::{
    files::{safe_path, Files},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub use super::confirmation::confirm;
pub use super::evidence::validate as validate_evidence;

pub fn compare(files: &Files, store: &Store, run: &mut Run) -> Result<()> {
    let baseline = baseline_run(store, run)?;
    compare_with(files, run, baseline.as_ref())
}

pub fn baseline_run(store: &Store, run: &Run) -> Result<Option<(Baseline, Run)>> {
    run.baseline_id
        .as_ref()
        .map(|id| {
            let baseline: Baseline = repository::get(store, "validationBaseline", id)?;
            let previous = repository::get(store, "validationRun", &baseline.run_id)?;
            Ok((baseline, previous))
        })
        .transpose()
}

/// Image decoding and comparison run without holding the database lock.
pub fn compare_with(
    files: &Files,
    run: &mut Run,
    baseline: Option<&(Baseline, Run)>,
) -> Result<()> {
    validate_evidence(files, run)?;
    let Some((baseline, previous)) = baseline else {
        run.verdict = "missingBaseline".into();
        return Ok(());
    };
    let flow = run.flow.as_ref().context("Missing flow")?;
    super::judgment::baseline_valid(baseline, previous)?;
    if baseline.project_id != run.project_id
        || baseline.flow_id != flow.id
        || baseline.signature != flow.definition.signature()?
        || previous.engine_version != run.engine_version
        || previous.runner_version != run.runner_version
    {
        run.verdict = "stale".into();
        return Ok(());
    }
    validate_evidence(files, previous)?;
    let mut findings = vec![];
    let mut green = run.evidence.len() == previous.evidence.len();
    let current_dir = repository::run_dir(files, &run.id)?;
    let previous_dir = repository::run_dir(files, &previous.id)?;
    for current in &run.evidence {
        let old = previous
            .evidence
            .iter()
            .find(|old| old.point == current.point && old.kind == current.kind);
        let Some(old) = old else {
            green = false;
            findings.push(json!({"point":current.point,"reason":"No matching baseline point"}));
            continue;
        };
        let (score, worst_tile) = if current.kind == "image" {
            similarity(
                &safe_path(&current_dir, &current.file)?,
                &safe_path(&previous_dir, &old.file)?,
                &flow.definition.config.masks,
            )?
        } else {
            let current_events = event_steps(&current.state);
            let old_events = event_steps(&old.state);
            let valid = !current_events.is_empty()
                && current_events == old_events
                && (current.end - old.end).abs() <= old.end * 0.2 + 1.0;
            (if valid { 1.0 } else { 0.0 }, if valid { 1.0 } else { 0.0 })
        };
        let passed = score >= flow.definition.config.threshold
            && worst_tile >= flow.definition.config.threshold
            && (current.kind != "image" || current.state["scene"] == old.state["scene"]);
        green &= passed;
        findings.push(
            json!({"point":current.point,"start":current.start,"end":current.end,
            "score":score,"worstTile":worst_tile,"passed":passed}),
        );
    }
    run.verdict = if green { "autoPassed" } else { "needsReview" }.into();
    run.judgments.push(
        json!({"id":repository::id(),"at":repository::now(),"source":"automatic",
        "method":super::judgment::METHOD,"baselineId":baseline.id,
        "runId":run.id,"snapshotId":run.snapshot_id,"baselineSnapshotId":previous.snapshot_id,
        "evidenceDigest":repository::digest(&run.evidence)?,"baselineEvidenceDigest":repository::digest(&previous.evidence)?,
        "threshold":flow.definition.config.threshold,"verdict":run.verdict,"findings":findings}),
    );
    Ok(())
}

fn event_steps(value: &Value) -> Vec<&Value> {
    value["events"]
        .as_array()
        .map(|a| a.iter().map(|event| &event["step"]).collect())
        .unwrap_or_default()
}

pub fn similarity(a: &Path, b: &Path, masks: &[[u32; 4]]) -> Result<(f64, f64)> {
    let a = image::ImageReader::open(a)?.decode()?.to_rgb8();
    let b = image::ImageReader::open(b)?.decode()?.to_rgb8();
    if a.dimensions() != b.dimensions() {
        return Ok((0.0, 0.0));
    }
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut worst: f64 = 1.0;
    for y in (0..a.height()).step_by(16) {
        for x in (0..a.width()).step_by(16) {
            let mut tile_sum = 0.0;
            let mut tile_count = 0;
            for py in y..(y + 16).min(a.height()) {
                for px in x..(x + 16).min(a.width()) {
                    if masks
                        .iter()
                        .any(|[mx, my, w, h]| px >= *mx && px < mx + w && py >= *my && py < my + h)
                    {
                        continue;
                    }
                    let delta: f64 = a
                        .get_pixel(px, py)
                        .0
                        .iter()
                        .zip(b.get_pixel(px, py).0)
                        .map(|(a, b)| f64::from(a.abs_diff(b)) / (255.0 * 3.0))
                        .sum();
                    tile_sum += delta;
                    tile_count += 1;
                }
            }
            if tile_count > 0 {
                worst = worst.min(1.0 - tile_sum / tile_count as f64);
            }
            sum += tile_sum;
            count += tile_count;
        }
    }
    if count == 0 {
        bail!("Comparison has no unmasked pixels");
    }
    Ok((1.0 - sum / count as f64, worst))
}
