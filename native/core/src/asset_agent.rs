use crate::{
    asset_checkpoint, asset_feedback, asset_preview::Client, asset_reference, asset_stages,
    asset_task, asset_tool, store::Store,
};
use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Context {
    pub store: Arc<Mutex<Store>>,
    pub root: PathBuf,
    pub client: Client,
}

pub struct Reply {
    pub value: Value,
    pub delivered: Option<String>,
}

impl Context {
    fn read(&self) -> Result<asset_task::State> {
        let db = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        asset_task::get(&db, &self.client.task_id)
    }

    pub fn pending_notice(&self) -> Result<Option<String>> {
        let state = self.read()?;
        Ok(asset_task::eligible(&state)
            .filter(|f| {
                matches!(
                    f.status.as_str(),
                    "received" | "delivering" | "pendingVerification"
                )
            })
            .map(|f| f.id.clone()))
    }

    pub fn uncertain(&self, reason: &str) -> Result<()> {
        let db = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        asset_task::mark_uncertain(&db, &self.client.task_id, reason)
    }

    pub async fn checkpoint(&self) -> Result<String> {
        asset_checkpoint::save(&self.store, &self.client).await
    }

    fn active(&self, db: &Store, params: &Value) -> Result<asset_task::State> {
        let task: Value = db
            .get("task", &self.client.task_id)?
            .context("Task no longer exists")?;
        anyhow::ensure!(
            task["status"] == "running"
                && params["threadId"].as_str().is_some_and(|id| !id.is_empty())
                && params["turnId"].as_str().is_some_and(|id| !id.is_empty())
                && task["threadId"] == params["threadId"]
                && task["turnId"] == params["turnId"],
            "Stale asset tool invocation"
        );
        let state = asset_task::get(db, &self.client.task_id)?;
        anyhow::ensure!(
            state.session_id.as_deref() == Some(&self.client.session_id),
            "Blender session changed"
        );
        Ok(state)
    }

    pub async fn call(&self, params: &Value) -> Result<Reply> {
        let mut check = {
            let db = self
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            self.active(&db, params)?
        };
        let input = &params["arguments"];
        let operation = input["operation"]
            .as_str()
            .context("Missing asset operation")?;
        if operation == "poll" {
            let state = {
                let db = self
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
                let mut state = self.active(&db, params)?;
                asset_feedback::reserve_delivery(&mut state);
                asset_task::save(&db, &state)?;
                state
            };
            if let Some(feedback) = asset_task::eligible(&state) {
                let png = asset_reference::annotated(&feedback.reference, &feedback.annotations)?;
                return Ok(Reply {
                    value: asset_tool::image_result(feedback, &png),
                    delivered: Some(feedback.id.clone()),
                });
            }
            return Ok(Reply {
                value: asset_tool::text_result(asset_tool::projection(&state)),
                delivered: None,
            });
        }
        let stages = if operation == "stages" {
            Some((
                input["revision"]
                    .as_u64()
                    .context("Missing plan revision: call state, then submit stages with its current top-level revision")?,
                serde_json::from_value::<Vec<asset_task::Stage>>(input["stages"].clone())?,
            ))
        } else {
            None
        };
        if let Some((revision, stages)) = &stages {
            asset_stages::update(&mut check, *revision, stages.clone())?;
        }
        let needs_checkpoint = matches!(
            operation,
            "checkpoint" | "stages" | "finishRound" | "complete" | "execute"
        );
        if needs_checkpoint {
            // Validate first. A malformed receipt must not save or alter the scene.
            if operation == "finishRound" {
                asset_stages::complete_round(&mut check)?;
            }
            if matches!(operation, "complete" | "execute") {
                let id = input["feedbackId"]
                    .as_str()
                    .context("Missing feedback ID")?;
                if operation == "execute" {
                    let feedback = check
                        .feedback
                        .iter_mut()
                        .find(|f| f.id == id)
                        .context("Unknown feedback")?;
                    feedback.checkpoint = Some("validation only".into());
                }
                asset_feedback::receipt(&mut check, id, operation, input)?;
            }
        }
        let checkpoint = if needs_checkpoint {
            Some(self.checkpoint().await?)
        } else {
            None
        };
        if operation == "finishRound" {
            asset_checkpoint::final_frame(&self.store, &self.root, &self.client).await?;
        }
        let db = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let mut state = self.active(&db, params)?;
        match operation {
            "state" | "checkpoint" => {}
            "stages" => {
                let (revision, stages) = stages.context("Missing stages")?;
                asset_stages::update(&mut state, revision, stages)?;
            }
            "finishRound" => asset_stages::complete_round(&mut state)?,
            "acknowledge" | "deciding" | "execute" | "check" | "complete" | "verifyApplied"
            | "verifyNotApplied" | "fail" | "retryFailed" => {
                let id = input["feedbackId"]
                    .as_str()
                    .context("Missing feedback ID")?;
                if operation == "execute" {
                    state
                        .feedback
                        .iter_mut()
                        .find(|f| f.id == id)
                        .context("Unknown feedback")?
                        .checkpoint = checkpoint;
                }
                asset_feedback::receipt(&mut state, id, operation, input)?;
            }
            _ => bail!("Unknown asset operation"),
        }
        asset_task::save(&db, &state)?;
        if operation != "state" {
            db.event(&self.client.task_id, &asset_task::now(), "assetStage", &json!({"operation":operation,"round":state.round,"revision":state.revision,"feedbackId":input["feedbackId"]}).to_string())?;
        }
        Ok(Reply {
            value: asset_tool::text_result(asset_tool::projection(&state)),
            delivered: None,
        })
    }

    /// Called only after the image-bearing tool response was written successfully.
    pub fn delivered(&self, id: &str) -> Result<()> {
        let db = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let mut state = asset_task::get(&db, &self.client.task_id)?;
        let feedback = state
            .feedback
            .iter_mut()
            .find(|f| f.id == id)
            .context("Feedback disappeared")?;
        if feedback.status == "delivering" {
            feedback.status = "waitingSwitch".into();
            feedback.delivered_at = Some(asset_task::now());
            feedback.history.push(
                json!({"at":asset_task::now(),"status":"waitingSwitch","transport":"inputImage"}),
            );
            asset_task::save(&db, &state)?;
        }
        Ok(())
    }
}
