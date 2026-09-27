//! Explicit numbered-region correspondence between two immutable attempt checkpoints.
use crate::object_run_recovery::candidate::publication::{
    deferred::frames, followup::frames::Reference,
};
use crate::{object_attempt::Attempt, object_task_types::valid_id};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Region {
    Matched {
        source_region: usize,
        target_region: usize,
    },
    Absent {
        source_region: usize,
        note: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Confirmation {
    pub source_attempt_id: String,
    pub source_frame: Reference,
    pub regions: Vec<Region>,
    pub confirmed: bool,
}

impl Confirmation {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.confirmed
                && valid_id(&self.source_attempt_id)
                && (1..=8).contains(&self.regions.len()),
            "INVALID_FEEDBACK_RELOCATION"
        );
        for (index, region) in self.regions.iter().enumerate() {
            let source = match region {
                Region::Matched {
                    source_region,
                    target_region,
                } => {
                    ensure!(*target_region < 8, "INVALID_FEEDBACK_RELOCATION");
                    *source_region
                }
                Region::Absent {
                    source_region,
                    note,
                } => {
                    ensure!(
                        !note.trim().is_empty() && note.len() <= 1000,
                        "INVALID_FEEDBACK_RELOCATION"
                    );
                    *source_region
                }
            };
            ensure!(source == index, "INVALID_FEEDBACK_RELOCATION");
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Resolved {
    pub confirmation: Confirmation,
    pub source_frame: Value,
    #[serde(skip)]
    pub data_url: String,
}

pub(crate) fn resolve(
    db: &Connection,
    target: &Attempt,
    target_frame: &Reference,
    confirmation: &Confirmation,
) -> Result<Resolved> {
    confirmation.validate()?;
    let source = crate::object_attempt_view::read(
        db,
        &target.preparation.project_id,
        &confirmation.source_attempt_id,
    )?;
    ensure!(
        source.id != target.id
            && source.preparation.run.id == target.preparation.run.id
            && source.preparation.run.object_id == target.preparation.run.object_id,
        "FEEDBACK_RELOCATION_SOURCE_MISMATCH"
    );
    let mut original = frames::resolve(db, &source, &confirmation.source_frame)?;
    let current = frames::resolve(db, target, target_frame)?;
    let source_regions = original["selection"]["regions"]
        .as_array()
        .context("PREVIEW_FEEDBACK_SELECTION_REQUIRED")?;
    let target_regions = current["selection"]["regions"]
        .as_array()
        .context("PREVIEW_FEEDBACK_SELECTION_REQUIRED")?;
    ensure!(
        confirmation.regions.len() == source_regions.len(),
        "FEEDBACK_RELOCATION_INCOMPLETE"
    );
    for region in &confirmation.regions {
        if let Region::Matched { target_region, .. } = region {
            ensure!(
                *target_region < target_regions.len(),
                "FEEDBACK_RELOCATION_TARGET_REGION"
            );
        }
    }
    let data_url = original["frame"]
        .as_object_mut()
        .context("PREVIEW_FRAME_PNG_REQUIRED")?
        .remove("dataUrl")
        .and_then(|v| v.as_str().map(str::to_owned))
        .context("PREVIEW_FRAME_PNG_REQUIRED")?;
    Ok(Resolved {
        confirmation: confirmation.clone(),
        source_frame: original,
        data_url,
    })
}
