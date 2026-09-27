use super::{
    evidence,
    model::{Baseline, Run},
    repository, requests,
};
use crate::{files::Files, store::Store};
use anyhow::{bail, Context, Result};
use serde_json::json;

pub fn confirm(
    files: &Files,
    store: &mut Store,
    run_id: &str,
    snapshot_id: &str,
    evidence_ids: &[String],
    request_id: &str,
    source: &str,
) -> Result<Run> {
    if source != "ui" {
        bail!("Human confirmation is only available through the Beaver user interface");
    }
    let mut run: Run = repository::get(store, "validationRun", run_id)?;
    anyhow::ensure!(
        run.kind != "objectPreview",
        "Object previews are not acceptance evidence"
    );
    if run.snapshot_id != snapshot_id {
        bail!("Snapshot changed; refresh the evidence being reviewed");
    }
    evidence::validate(files, &run)?;
    let input = json!({"projectId":run.project_id,"runId":run_id,"snapshotId":snapshot_id,"evidenceIds":evidence_ids,"requestId":request_id});
    let request = requests::Request::new("validation.evidence.confirm", &input)?;
    if request.replay(store)?.is_some() {
        return Ok(run);
    }
    if evidence_ids.is_empty()
        || evidence_ids
            .iter()
            .any(|id| !run.evidence.iter().any(|e| &e.id == id))
    {
        bail!("Select evidence belonging to this run");
    }
    run.confirmations.push(json!({"requestId":request_id,"runId":run.id,"evidenceIds":evidence_ids,"snapshotId":snapshot_id,"at":repository::now(),"source":"user"}));
    let mut records = vec![];
    if super::judgment::human_complete(&run) {
        let flow = run.flow.as_ref().context("Missing flow")?;
        let baseline = Baseline {
            id: repository::id(),
            project_id: run.project_id.clone(),
            flow_id: flow.id.clone(),
            signature: flow.definition.signature()?,
            run_id: run.id.clone(),
            snapshot_id: run.snapshot_id.clone(),
            evidence_ids: run.evidence.iter().map(|e| e.id.clone()).collect(),
            confirmed_at: repository::now(),
            source: "user".into(),
            previous_id: repository::baseline(store, flow)?.map(|b| b.id),
        };
        records.push((
            "validationBaseline",
            baseline.id.clone(),
            serde_json::to_value(baseline)?,
        ));
        run.verdict = "userPassed".into();
    }
    records.push(("validationRun", run.id.clone(), serde_json::to_value(&run)?));
    request.finish(store, json!({"runId":run.id}), records)?;
    Ok(run)
}
