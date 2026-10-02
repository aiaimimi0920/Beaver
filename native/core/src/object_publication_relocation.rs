//! Final owner correspondence is bound to immutable feedback and the exact candidate output.
use super::{deferred, followup::frames::Reference, Preview, Request};
use crate::object_run_recovery::resume::rework::relocation::Region;
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Requirement {
    pub source_digest: String,
    pub region_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Confirmation {
    pub source_digest: String,
    pub target_frame: Reference,
    pub regions: Vec<Region>,
    pub confirmed: bool,
}

pub(super) fn approve(request: &Request, preview: &Preview) -> Result<()> {
    for item in &preview.feedback {
        let decision = request
            .feedback
            .iter()
            .find(|d| d.request_id == item.request_id)
            .context("OBJECT_PUBLICATION_FEEDBACK_UNRESOLVED")?;
        match (&item.relocation_requirement, &decision.final_relocation) {
            (Some(required), Some(confirmation)) => {
                ensure!(
                    confirmation.confirmed && confirmation.source_digest == required.source_digest,
                    "OBJECT_PUBLICATION_RELOCATION_SOURCE_CHANGED"
                );
                ensure!(
                    (1..=8).contains(&required.region_count)
                        && confirmation.regions.len() == required.region_count,
                    "OBJECT_PUBLICATION_RELOCATION_INCOMPLETE"
                );
                for (index, region) in confirmation.regions.iter().enumerate() {
                    let source = match region {
                        Region::Matched {
                            source_region,
                            target_region,
                        } => {
                            ensure!(
                                *target_region < 8,
                                "OBJECT_PUBLICATION_RELOCATION_TARGET_REGION"
                            );
                            *source_region
                        }
                        Region::Absent {
                            source_region,
                            note,
                        } => {
                            ensure!(
                                !note.trim().is_empty() && note.len() <= 1000,
                                "OBJECT_PUBLICATION_RELOCATION_REASON_REQUIRED"
                            );
                            *source_region
                        }
                    };
                    ensure!(source == index, "OBJECT_PUBLICATION_RELOCATION_INCOMPLETE");
                }
            }
            (None, None) => {}
            (Some(_), None) => anyhow::bail!(
                "OBJECT_PUBLICATION_FINAL_RELOCATION_REQUIRED: {}",
                item.request_id
            ),
            (None, Some(_)) => anyhow::bail!("OBJECT_PUBLICATION_RELOCATION_SOURCE_CHANGED"),
        }
    }
    Ok(())
}

pub(super) fn validate(db: &Connection, request: &Request, preview: &Preview) -> Result<()> {
    approve(request, preview)?;
    let attempt =
        crate::object_attempt_view::read(db, &request.project_id, &request.target.attempt_id)?;
    ensure!(
        crate::object_attempt_view::Target::from_record(&attempt) == request.target,
        "OBJECT_PUBLICATION_RELOCATION_TARGET_CHANGED"
    );
    for decision in &request.feedback {
        let Some(confirmation) = &decision.final_relocation else {
            continue;
        };
        let frame = deferred::frames::resolve(db, &attempt, &confirmation.target_frame)
            .context("OBJECT_PUBLICATION_RELOCATION_TARGET_CHANGED")?;
        let count = frame["selection"]["regions"]
            .as_array()
            .context("PREVIEW_FEEDBACK_SELECTION_REQUIRED")?
            .len();
        for region in &confirmation.regions {
            if let Region::Matched { target_region, .. } = region {
                ensure!(
                    *target_region < count,
                    "OBJECT_PUBLICATION_RELOCATION_TARGET_REGION"
                );
            }
        }
    }
    Ok(())
}
