use super::{
    model::{Release, Run},
    repository,
};
use crate::store::Store;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

/// Clone durable records under the store lock, then hash their files outside it.
pub struct Prepared {
    value: Value,
    checks: Vec<(String, Result<(Run, Option<Run>)>)>,
}

pub fn prepare(store: &Store, release: &Release) -> Result<Prepared> {
    let value = super::release::display(store, release)?;
    let checks = value["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["passed"] == true)
        .filter_map(|item| item["runId"].as_str())
        .map(|id| {
            let check = (|| {
                let run: Run = repository::get(store, "validationRun", id)?;
                let baseline = if run.kind == "visual" && run.verdict == "autoPassed" {
                    Some(
                        super::comparison::baseline_run(store, &run)?
                            .context("Comparison baseline missing")?
                            .1,
                    )
                } else {
                    None
                };
                Ok((run, baseline))
            })();
            (id.into(), check)
        })
        .collect();
    Ok(Prepared { value, checks })
}

pub fn list(store: &Store, project: &str) -> Result<Vec<Prepared>> {
    let mut releases: Vec<Release> = store.list("validationRelease")?;
    releases.retain(|release| release.project_id == project);
    releases.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    releases
        .iter()
        .take(30)
        .map(|release| prepare(store, release))
        .collect()
}

impl Prepared {
    pub fn finish(mut self, data: &Path) -> Value {
        for (id, check) in self.checks {
            let integrity = check.and_then(|(run, baseline)| {
                if run.kind == "code" {
                    super::code::validate(data, &run)?;
                } else {
                    super::evidence::validate(data, &run)?;
                }
                if let Some(previous) = baseline {
                    super::evidence::validate(data, &previous)?;
                }
                Ok(())
            });
            if let Err(error) = integrity {
                self.value["ready"] = json!(false);
                self.value["integrityError"] = json!(error.to_string());
                for item in self.value["items"].as_array_mut().into_iter().flatten() {
                    if item["runId"] == id {
                        item["passed"] = json!(false);
                        item["integrityError"] = json!(error.to_string());
                    }
                }
            }
        }
        self.value
    }
}
