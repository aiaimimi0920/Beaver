use crate::{
    asset_delivery, asset_delivery_files,
    asset_task::State,
    asset_work,
    asset_work_contract::{Action, Outcome},
    asset_work_inputs::{self, Captured},
    files::{Files, Snapshot},
    task_callback_contract::Request,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
#[path = "task_callback_prepare_tests.rs"]
mod tests;

pub(crate) struct Prepared {
    request: Value,
    inputs: Option<Vec<Captured>>,
    delivery: Option<Snapshot>,
}

pub(crate) fn required(request: &Request) -> bool {
    matches!(
        request,
        Request::SubmitDelivery { .. }
            | Request::Work {
                change: Action::Begin { .. }
                    | Action::Finish {
                        outcome: Outcome::Completed,
                        ..
                    },
                ..
            }
    )
}

/// Validate state before releasing the mutex. The transaction repeats its authoritative checks.
pub(crate) fn preflight(state: &State, request: &Request) -> Result<()> {
    match request {
        Request::Work {
            asset_revision,
            stage_id,
            ..
        } => asset_work::current(state, *asset_revision, stage_id),
        Request::SubmitDelivery {
            asset_revision,
            stage_id,
            input_candidates,
            summary,
            ..
        } => {
            ensure!(
                !summary.trim().is_empty() && summary.len() <= 12000,
                "Supply a candidate summary of at most 12000 bytes"
            );
            asset_delivery::can_submit(state, *asset_revision, stage_id, input_candidates)?;
            asset_work::completed_attempts(state, stage_id)?;
            Ok(())
        }
        _ => anyhow::bail!("Operation does not require file preparation"),
    }
}

impl Prepared {
    /// The only constructor performs IO outside the store lock, never trusting model proof flags.
    pub(crate) fn capture(
        files: &Files,
        workspace: &Path,
        state: &State,
        request: &Request,
        input: &Value,
    ) -> Result<Self> {
        let mut proof = Self {
            request: input.clone(),
            inputs: None,
            delivery: None,
        };
        match request {
            Request::Work {
                change: Action::Begin { input_files, .. },
                ..
            } => {
                proof.inputs = Some(asset_work_inputs::capture(files, workspace, input_files)?);
            }
            Request::Work {
                change:
                    Action::Finish {
                        attempt_id,
                        outcome: Outcome::Completed,
                        ..
                    },
                ..
            } => {
                let inputs =
                    asset_work_inputs::for_attempts(state, std::slice::from_ref(attempt_id))?;
                asset_work_inputs::verify(files, workspace, &inputs)?;
            }
            Request::SubmitDelivery {
                stage_id, paths, ..
            } => {
                let ids = asset_work::completed_attempts(state, stage_id)?;
                let inputs = asset_work_inputs::for_attempts(state, &ids)?;
                proof.delivery = Some(asset_delivery_files::capture(files, workspace, paths)?);
                asset_work_inputs::verify(files, workspace, &inputs)?;
            }
            _ => anyhow::bail!("Operation does not require file preparation"),
        }
        Ok(proof)
    }

    pub(crate) fn authorize(&self, request: &Value) -> Result<()> {
        ensure!(
            self.request == *request,
            "File preparation does not match callback"
        );
        Ok(())
    }

    pub(crate) fn inputs(&self) -> Option<&[Captured]> {
        self.inputs.as_deref()
    }

    pub(crate) fn delivery(&self) -> Result<&Snapshot> {
        self.delivery
            .as_ref()
            .context("Missing prepared candidate files")
    }
}
