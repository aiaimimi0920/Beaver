use super::{flow::Action, model::Run, repository};
use crate::files::{file_hash, safe_path, Files};
use anyhow::{bail, ensure, Context, Result};
use std::{collections::BTreeSet, io::Read};

pub fn media_path(files: &Files, run: &Run, evidence_id: &str) -> Result<std::path::PathBuf> {
    let evidence = run
        .evidence
        .iter()
        .find(|e| e.id == evidence_id)
        .context("Evidence not registered")?;
    let path = safe_path(&repository::run_dir(files, &run.id)?, &evidence.file)?;
    ensure!(
        file_hash(&path)?.as_ref() == Some(&evidence.sha256),
        "Evidence is missing or changed"
    );
    Ok(path)
}

pub fn validate(files: &Files, run: &Run) -> Result<()> {
    ensure!(
        run.kind == "visual"
            && run.status == "completed"
            && run.error.is_none()
            && !run.evidence.is_empty(),
        "Only a complete visual run with valid evidence can be confirmed"
    );
    let flow = run.flow.as_ref().context("Missing flow")?;
    ensure!(
        run.snapshot_id == repository::digest(&run.snapshot)? && flow.project_id == run.project_id,
        "Evidence candidate or flow ownership changed"
    );
    flow.definition.validate()?;
    ensure!(
        run.completed_steps == flow.definition.steps.len(),
        "Flow has incomplete steps"
    );
    let directory = repository::run_dir(files, &run.id)?;
    let mut ids = BTreeSet::new();
    let mut points = BTreeSet::new();
    for evidence in &run.evidence {
        ensure!(
            ids.insert(&evidence.id) && points.insert((&evidence.kind, &evidence.point)),
            "Duplicate evidence identifier or point"
        );
        ensure!(
            evidence.start.is_finite()
                && evidence.end.is_finite()
                && evidence.start >= 0.0
                && evidence.end >= evidence.start,
            "Invalid evidence timeline"
        );
        let path = safe_path(&directory, &evidence.file)?;
        ensure!(
            file_hash(&path)?.as_ref() == Some(&evidence.sha256),
            "Evidence is missing or has changed: {}",
            evidence.point
        );
        match evidence.kind.as_str() {
            "image" => {
                let image = image::ImageReader::open(path)?.decode()?;
                ensure!(
                    image.width() == flow.definition.config.width
                        && image.height() == flow.definition.config.height,
                    "Evidence resolution changed"
                );
            }
            "video" => {
                let mut file = std::fs::File::open(path)?;
                let mut header = [0u8; 4];
                file.read_exact(&mut header)?;
                ensure!(
                    header == [0x1a, 0x45, 0xdf, 0xa3]
                        && file.metadata()?.len() >= 1000
                        && evidence.end > 0.0,
                    "Video is empty or invalid"
                );
                let events = evidence.state["events"]
                    .as_array()
                    .context("Video events missing")?;
                ensure!(
                    events.len() == flow.definition.steps.len(),
                    "Video timeline incomplete"
                );
                for (event, step) in events.iter().zip(&flow.definition.steps) {
                    ensure!(
                        event["step"] == step.id,
                        "Video events do not match the recorded flow"
                    );
                }
            }
            _ => bail!("Unknown evidence media type"),
        }
    }
    for step in &flow.definition.steps {
        let suffix = if matches!(step.action, Action::Capture) {
            "capture"
        } else if flow.definition.video {
            "keyframe"
        } else {
            continue;
        };
        ensure!(
            run.evidence
                .iter()
                .any(|e| e.kind == "image" && e.point == format!("{}:{suffix}", step.id)),
            "Required capture point missing: {}",
            step.id
        );
    }
    ensure!(
        !flow.definition.video
            || run
                .evidence
                .iter()
                .filter(|e| e.kind == "video" && e.point == "recording")
                .count()
                == 1,
        "Required video missing"
    );
    Ok(())
}
