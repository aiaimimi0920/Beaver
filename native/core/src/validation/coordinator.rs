use super::{model::Run, repository, requests};
use crate::{files::Files, store::Store};
use anyhow::{Context, Result};
use serde_json::{json, Value};

/// A broken record must not prevent other projects from advancing.
pub fn refresh(store: &mut Store, files: &Files) -> Result<bool> {
    refresh_matching(store, files, |_| true)
}

pub(crate) fn refresh_matching(
    store: &mut Store,
    files: &Files,
    owns: impl Fn(&str) -> bool,
) -> Result<bool> {
    let mut changed = false;
    for kind in ["validationCoverage", "validationFeedback"] {
        for mut record in store.list::<Value>(kind)? {
            if !owns(record["projectId"].as_str().unwrap_or("")) {
                continue;
            }
            let Some(id) = record["id"].as_str().map(str::to_owned) else {
                continue;
            };
            let before = record.clone();
            let result = if kind == "validationCoverage" {
                super::coverage::refresh(store, files, &mut record)
            } else {
                refresh_feedback(store, files, &mut record)
            };
            if let Err(error) = result {
                record["status"] = json!(if kind == "validationCoverage" {
                    "missing"
                } else {
                    "coordinationFailed"
                });
                record["reason"] = json!(error.to_string());
            }
            if record != before {
                store.put(kind, &id, &record)?;
                changed = true;
            }
        }
    }
    Ok(super::automatic_repair::refresh_matching(store, files, owns)? || changed)
}

fn refresh_feedback(store: &mut Store, files: &Files, feedback: &mut Value) -> Result<()> {
    if ["taskQueued", "reviewQueued"].contains(&feedback["status"].as_str().unwrap_or("")) {
        let id = feedback["id"]
            .as_str()
            .context("Missing feedback ID")?
            .to_owned();
        let task: Value = repository::get(
            store,
            "task",
            feedback["taskId"]
                .as_str()
                .context("Missing feedback task")?,
        )?;
        feedback["taskStatus"] = task["status"].clone();
        if task["status"] != "completed" {
            return Ok(());
        }
        if feedback["status"] == "reviewQueued" {
            feedback["status"] = json!("reviewReady");
            feedback["findings"] = task["report"].clone();
            return Ok(());
        }
        if task["accepted"] != true {
            return Ok(());
        }
        let project = feedback["projectId"].as_str().context("Missing project")?;
        let original: Run = repository::get(
            store,
            "validationRun",
            feedback["runId"].as_str().context("Missing original run")?,
        )?;
        let snapshot = repository::snapshot(store, files, project)?;
        let mut run = repository::build_run(
            store,
            project,
            snapshot,
            original.flow,
            task["id"].as_str().map(str::to_owned),
            None,
        )?;
        run.managed = true;
        feedback["rerunId"] = json!(run.id);
        feedback["status"] = json!("rerunning");
        requests::commit(
            store,
            vec![
                ("validationRun", run.id.clone(), serde_json::to_value(run)?),
                ("validationFeedback", id, feedback.clone()),
            ],
        )?;
    } else if feedback["status"] == "rerunning" {
        let run: Run = repository::get(
            store,
            "validationRun",
            feedback["rerunId"].as_str().context("Missing rerun")?,
        )?;
        if ["queued", "running"].contains(&run.status.as_str()) {
            return Ok(());
        }
        feedback["status"] = json!(if run.status == "completed" {
            "evidenceReady"
        } else {
            "rerunFailed"
        });
        feedback["verdict"] = json!(run.verdict);
        feedback["reason"] = json!(run.error);
    }
    Ok(())
}
