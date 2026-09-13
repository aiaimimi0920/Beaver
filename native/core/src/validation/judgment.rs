use super::{
    model::{Baseline, Run},
    repository,
};
use crate::store::Store;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeSet;

pub const METHOD: &str = "pixel-tiles-and-event-keyframes-v1";

pub fn human_complete(run: &Run) -> bool {
    let confirmed: BTreeSet<&str> = run
        .confirmations
        .iter()
        .filter(|v| {
            v["source"] == "user" && v["snapshotId"] == run.snapshot_id && v["runId"] == run.id
        })
        .filter_map(|v| v["evidenceIds"].as_array())
        .flatten()
        .filter_map(|v| v.as_str())
        .collect();
    !run.evidence.is_empty()
        && run
            .evidence
            .iter()
            .all(|e| confirmed.contains(e.id.as_str()))
}

pub fn baseline_valid(baseline: &Baseline, previous: &Run) -> Result<()> {
    let flow = previous.flow.as_ref().context("Baseline flow missing")?;
    let ids: BTreeSet<_> = previous.evidence.iter().map(|e| &e.id).collect();
    ensure!(
        baseline.source == "user"
            && baseline.run_id == previous.id
            && baseline.project_id == previous.project_id
            && baseline.flow_id == flow.id
            && baseline.signature == flow.definition.signature()?
            && baseline.snapshot_id == previous.snapshot_id
            && human_complete(previous)
            && baseline.evidence_ids.iter().collect::<BTreeSet<_>>() == ids,
        "Baseline is not a complete human-approved run"
    );
    Ok(())
}

pub fn green(store: &Store, run: &Run) -> Result<bool> {
    if run.kind != "visual"
        || run.status != "completed"
        || run.error.is_some()
        || run.evidence.is_empty()
    {
        return Ok(false);
    }
    if run.verdict == "userPassed" {
        return Ok(human_complete(run));
    }
    if run.verdict != "autoPassed" {
        return Ok(false);
    }
    let Some(id) = run.baseline_id.as_deref() else {
        return Ok(false);
    };
    let baseline: Baseline = repository::get(store, "validationBaseline", id)?;
    let previous: Run = repository::get(store, "validationRun", &baseline.run_id)?;
    baseline_valid(&baseline, &previous)?;
    let flow = run.flow.as_ref().context("Missing visual flow")?;
    if baseline.project_id != run.project_id
        || baseline.flow_id != flow.id
        || baseline.signature != flow.definition.signature()?
        || previous.engine_version != run.engine_version
        || previous.runner_version != run.runner_version
    {
        return Ok(false);
    }
    let digest = repository::digest(&run.evidence)?;
    let previous_digest = repository::digest(&previous.evidence)?;
    Ok(run.judgments.last().is_some_and(|j| {
        j["source"] == "automatic"
            && j["method"] == METHOD
            && j["verdict"] == "autoPassed"
            && j["runId"] == run.id
            && j["snapshotId"] == run.snapshot_id
            && j["baselineId"] == baseline.id
            && j["baselineSnapshotId"] == previous.snapshot_id
            && j["evidenceDigest"] == digest
            && j["baselineEvidenceDigest"] == previous_digest
            && j["threshold"].as_f64() == Some(flow.definition.config.threshold)
            && j["findings"].as_array().is_some_and(|items| {
                items.len() == run.evidence.len() && items.iter().all(|f| f["passed"] == true)
            })
    }))
}
