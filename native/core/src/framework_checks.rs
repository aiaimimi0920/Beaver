use crate::{
    asset_delivery_files::{self as artifacts, Candidate},
    files::{Files, Snapshot},
    framework_contract::Configuration,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn digest(value: &impl serde::Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub fn run(
    files: &Files,
    candidate: &Candidate,
    snapshot: &Snapshot,
    inputs: &[(String, Snapshot)],
    configuration: &Configuration,
    cancelled: &AtomicBool,
) -> Result<Value> {
    ensure!(!cancelled.load(Ordering::SeqCst), "Operation cancelled");
    let directory = artifacts::export_snapshot(files, snapshot)?;
    ensure!(!cancelled.load(Ordering::SeqCst), "Operation cancelled");
    let mut exports = Vec::new();
    for (attempt, snapshot) in inputs {
        ensure!(!cancelled.load(Ordering::SeqCst), "Operation cancelled");
        exports.push(json!({"attemptId":attempt,"manifest":snapshot,"path":if snapshot.is_empty() {None} else {Some(artifacts::export_snapshot(files,snapshot)?)} }));
    }
    let context = json!({"protocolVersion":1,"candidate":candidate,"configuration":configuration,"snapshot":snapshot,"workspace":directory,"attemptInputs":exports});
    let mut reports = Vec::new();
    for rule in &configuration.rules {
        ensure!(!cancelled.load(Ordering::SeqCst), "Operation cancelled");
        let result = crate::framework_command::run(&rule.checker, &directory, &context, cancelled);
        reports.push(match result {
            Ok(evidence) => {
                let pass = evidence["report"]["ruleId"] == rule.id && evidence["report"]["ruleVersion"] == rule.version && evidence["report"]["verdict"] == "pass";
                json!({"ruleId":rule.id,"version":rule.version,"passed":pass,"evidence":evidence})
            }
            Err(error) => json!({"ruleId":rule.id,"version":rule.version,"passed":false,"error":error.to_string()}),
        });
    }
    // A checker may write a report, but must not change its frozen inputs.
    artifacts::verify_workspace(files, &directory, snapshot)?;
    for ((_, snapshot), export) in inputs.iter().zip(&exports) {
        if let Some(path) = export["path"].as_str() {
            artifacts::verify_workspace(files, std::path::Path::new(path), snapshot)?;
        }
    }
    Ok(
        json!({"candidateId":candidate.id,"candidateSha256":digest(candidate)?,"configurationRevision":configuration.revision,"configurationSha256":digest(configuration)?,"passed":reports.iter().all(|r|r["passed"]==true),"reports":reports,"snapshot":snapshot,"attemptInputs":exports,"exportPath":directory,"source":"beaver-checker","runnerVersion":1,"time":crate::asset_task::now()}),
    )
}

pub fn gate(store: &Store, task: &str, candidate: &Candidate) -> Result<()> {
    let configuration = crate::framework::configuration(store, task)?;
    if !configuration.rules.is_empty() {
        let report: Value = store
            .get(&format!("framework-check/{task}"), &candidate.id)?
            .context("Run required candidate checks before approval")?;
        ensure!(
            report["passed"] == true
                && report["candidateSha256"] == digest(candidate)?
                && report["configurationRevision"] == configuration.revision
                && report["configurationSha256"] == digest(&configuration)?,
            "Candidate technical checks are missing, failed or stale"
        );
    }
    if configuration.semantic_required {
        let judgment: Value = store
            .get(
                &format!("framework-judgment/{task}"),
                &format!("owner-{}", candidate.id),
            )?
            .context("Owner semantic judgment is required")?;
        ensure!(
            judgment["verdict"] == "pass"
                && judgment["candidateSha256"] == digest(candidate)?
                && judgment["configurationRevision"] == configuration.revision
                && judgment["configurationSha256"] == digest(&configuration)?,
            "Owner semantic judgment failed or is stale"
        );
    }
    Ok(())
}
